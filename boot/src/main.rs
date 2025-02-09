#![no_std]
#![no_main]

extern crate alloc;

use core::alloc::Layout;
use core::{mem, ptr};
use alloc::alloc::alloc;
use alloc::vec::Vec;
use anyhow::{anyhow, Result};
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
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, PageSize, PageTable, PhysFrame, Size2MiB};

use kernel::mem::{MemoryPool, KERNEL_END, KERNEL_SIZE, KERNEL_START};
use kernel::drivers::video::framebuffer::Framebuffer;

// TODO: do not pass framebuffer

type KStart = extern "sysv64" fn(u64, &[MemoryPool], Framebuffer) -> !;

struct Memory {
    kernel: MemoryPool,
    free:   Vec<MemoryPool>
}

impl Memory {
    fn build() -> Result<Memory> {
        println!("[+] Building Memory Map");

        let mut kernel = None;

        let free = boot::memory_map(MemoryType::BOOT_SERVICES_DATA)?
            .entries()
            .filter(|e| e.ty == MemoryType::CONVENTIONAL)
            .map(|e| {
                let start = addr::align_up(e.phys_start, Size2MiB::SIZE);
                let end = addr::align_down(e.phys_start + e.page_count * 4096, Size2MiB::SIZE);
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

        println!("Mapping 0x{:x} -- 0x{:x} to 0x{:x} -- 0x{:x}", self.kernel.start, self.kernel.end - 1, KERNEL_START, KERNEL_END);
        let kpool = self.kernel;
        kpool.map(&mut page_table, self, KERNEL_START)?;

        let (oldptframe, flags) = Cr3::read();
        let oldpt = & *(oldptframe.start_address().as_u64() as *const PageTable);
        for (i, entry) in oldpt.iter().enumerate() {
            if !entry.is_unused() { pt[i] = entry.clone(); }
        }

        Cr3::write(ptframe, flags);

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
        .try_for_each(|phdr| {
            let src = elf.segment_data(&phdr)?;
            let dst = (mem.kernel.start + phdr.p_paddr) as *mut u8;
            let size = phdr.p_memsz as usize;

            println!("Copy {} bytes to 0x{:x} -- 0x{:x}", size, dst as u64, dst as usize + size - 1);

            unsafe {
                ptr::write_bytes(dst, 0, size);
                ptr::copy(src.as_ptr(), dst, src.len());
            }

            anyhow::Ok(())
        })?;

    unsafe { Ok(mem::transmute(elf.ehdr.e_entry)) }
}

fn find_acpi() -> Result<u64> {
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

    let ptr = gop.frame_buffer().as_mut_ptr();
    unsafe { Ok(mem::transmute(ptr)) }
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
        let _ = boot::exit_boot_services(MemoryType::BOOT_SERVICES_DATA);
    }

    kstart(acpi, &mem.free, fb);

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

    panic!();
    Status::SUCCESS
}
