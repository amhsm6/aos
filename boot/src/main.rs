#![no_main]
#![no_std]

extern crate alloc;

use core::alloc::Layout;
use core::{mem, ptr, u64};
use alloc::alloc::alloc;
use alloc::vec::Vec;
use anyhow::{anyhow, Error, Result};
use elf::ElfBytes;
use elf::abi::PT_LOAD;
use elf::endian::LittleEndian;
use uefi::mem::memory_map::MemoryMap;
use uefi::println;
use uefi::prelude::*;
use uefi::boot::MemoryType;
use uefi::fs::FileSystem;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
use uefi::proto::console::text::Input;
use uefi::table::cfg::ACPI2_GUID;
use x86_64::{addr, PhysAddr, VirtAddr};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags, PhysFrame, Size2MiB};

const KERNEL_START: u64 = 0xffffffffaf000000;
const KERNEL_END:   u64 = 0xffffffffffffffff;
const KERNEL_SIZE:  u64 = KERNEL_END - KERNEL_START + 1;

type ACPIAddr = u64;
type Framebuffer<'a> = &'a mut [[u32; 1920]; 1080];
type KStart = extern "sysv64" fn(ACPIAddr, Framebuffer) -> !;

#[derive(Clone, Copy)]
struct MemoryPool {
    start: u64,
    end:   u64
}

struct Memory {
    kernel: MemoryPool,
    global: MemoryPool,
    free:   Vec<MemoryPool>
}

impl Memory {
    fn build() -> Result<Memory> {
        println!("[+] Building Memory Map");

        let mut kernel = None;
        let mut global = MemoryPool { start: u64::MAX, end: u64::MIN };

        let free = boot::memory_map(MemoryType::BOOT_SERVICES_DATA)?
            .entries()
            .map(|e| (e.phys_start, e.phys_start + e.page_count * 4096, e.ty))
            .inspect(|(start, end, _)| {
                global.start = global.start.min(*start);
                global.end = global.end.max(*end);
            })
            .filter(|(_, _, typ)| *typ == MemoryType::CONVENTIONAL)
            .map(|(start, end, _)| {
                let start = addr::align_up(start, Size2MiB::SIZE);
                let end = addr::align_down(end, Size2MiB::SIZE);
                (start, end)
            })
            .filter(|(start, end)| end > start)
            .map(|(start, end)| {
                if kernel.is_none() && end - start >= KERNEL_SIZE {
                    let kernel_end = start + KERNEL_SIZE;
                    kernel = Some(MemoryPool { start, end: kernel_end });

                    MemoryPool { start: kernel_end, end }
                } else {
                    MemoryPool { start, end }
                }
            })
            .collect::<Vec<MemoryPool>>();

        Ok(
            Memory {
                kernel: kernel.ok_or(anyhow!("Not enough memory"))?,
                global,
                free
            }
        )
    }

    unsafe fn map(&mut self) -> Result<()> {
        println!("[+] Mapping Memory");

        let ptframe = self.allocate_frame().ok_or(anyhow!("Unable to allocate frame"))?;

        let pt = &mut *(ptframe.start_address().as_u64() as *mut PageTable);
        pt.zero();
        let mut page_table = OffsetPageTable::new(pt, VirtAddr::zero());

        self.map_pool(&mut page_table, self.kernel, KERNEL_START)?;
        self.map_pool(&mut page_table, MemoryPool { start: self.global.start, end: self.kernel.start }, 0)?;
        self.map_pool(&mut page_table, MemoryPool { start: self.kernel.end, end: self.global.end }, self.kernel.end)?;

        Cr3::write(ptframe, Cr3::read().1);

        Ok(())
    }

    unsafe fn map_pool(&mut self, page_table: &mut OffsetPageTable, pool: MemoryPool, vstart: u64) -> Result<()> {
        let pstart = pool.start;
        let pend = pool.end - 1;
        let vend = vstart + pend - pstart;

        println!("Mapping 0x{:x} -- 0x{:x} to 0x{:x} -- 0x{:x}", pstart, pend, vstart, vend);

        let vstart: Page<Size2MiB> = Page::from_start_address(VirtAddr::new(vstart)).map_err(Error::msg)?;
        let vend = Page::containing_address(VirtAddr::new(vend));
        let pstart = PhysFrame::from_start_address(PhysAddr::new(pstart)).map_err(Error::msg)?;
        let pend = PhysFrame::containing_address(PhysAddr::new(pend));

        let pages = Page::range_inclusive(vstart, vend);
        let frames = PhysFrame::range_inclusive(pstart, pend);

        if pages.len() != frames.len() { return Err(anyhow!("Incorrect mapping")); }

        for (page, frame) in pages.zip(frames) {
            page_table
                .map_to(page, frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE, self)
                .map_err(|e| anyhow!("{e:?}"))?
                .flush();
        }

        Ok(())
    }
}

unsafe impl<S: PageSize> FrameAllocator<S> for Memory {
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        unsafe {
            let layout = Layout::from_size_align(S::SIZE as usize, S::SIZE as usize).ok()?;
            let ptr = alloc(layout);
            PhysFrame::from_start_address(PhysAddr::new(ptr as u64)).ok()
        }
    }
}

fn load_kernel(mem: &Memory) -> Result<KStart> {
    println!("[+] Loading Kernel");

    let fs_proto = boot::get_image_file_system(boot::image_handle())?;
    let mut fs = FileSystem::new(fs_proto);

    let buf = fs.read(cstr16!("\\kernel.elf"))?;
    let elf: ElfBytes<LittleEndian> = ElfBytes::minimal_parse(&buf)?;

    elf.segments()
        .ok_or(anyhow!("Elf does not contain segments"))?
        .into_iter()
        .filter(|phdr| phdr.p_type == PT_LOAD)
        .map(|phdr| {
            let src = elf.segment_data(&phdr)?;
            let dst = (mem.kernel.start + phdr.p_paddr) as *mut u8;
            let size = phdr.p_memsz as usize;

            println!("Copy {} bytes to 0x{:x} -- 0x{:x}", size, dst as u64, dst as usize + size);

            unsafe {
                ptr::write_bytes(dst, 0, size);
                ptr::copy(src.as_ptr(), dst, src.len());
            }

            Ok(())
        })
        .collect::<Result<()>>()?;

    unsafe { Ok(mem::transmute(elf.ehdr.e_entry)) }
}

fn find_acpi() -> Result<ACPIAddr> {
    println!("[+] Locating ACPI Table");

    system::with_config_table(|entries| {
        entries.iter()
            .find(|e| e.guid == ACPI2_GUID)
            .map(|e| e.address as u64)
    }).ok_or(anyhow!("ACPI Table not found"))
}

fn wait_for_key() -> Result<()> {
    let input_handle = boot::get_handle_for_protocol::<Input>()?;
    let mut input = boot::open_protocol_exclusive::<Input>(input_handle)?;

    println!("[!] Press any key to proceed...");
    while input.read_key()?.is_none() {}

    Ok(())
}

// TODO: remove
fn setup_video<'a>() -> Result<Framebuffer<'a>> {
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)?;

    let mode = gop.modes()
        .find(|mode| {
            let info = mode.info();
            info.resolution() == (1920, 1080) &&
                info.pixel_format() == PixelFormat::Bgr &&
                info.stride() == 1920
        })
        .ok_or(anyhow!("No graphic modes available"))?;

    gop.set_mode(&mode)?;

    let ptr = gop.frame_buffer().as_mut_ptr() as *mut [[u32; 1920]; 1080];
    unsafe { Ok(&mut *ptr) }
}

fn init() -> Result<()> {
    uefi::helpers::init()?;
    system::with_stdout(|stdout| stdout.clear())?;

    let mut mem = Memory::build()?;

    let kstart = load_kernel(&mem)?;
    let acpi = find_acpi()?;
    unsafe { mem.map()? };
    wait_for_key()?;

    println!("[+] Starting Kernel");

    let fb = setup_video()?;

    unsafe {
        boot::exit_boot_services(MemoryType::BOOT_SERVICES_DATA);
    }

    kstart(acpi, fb);

    Ok(())
}

#[uefi::entry]
fn main() -> Status {
    match init() {
        Ok(_) => {}
        Err(err) => {
            println!("ERROR: {err}");
        }
    }

    loop {}
    Status::SUCCESS
}
