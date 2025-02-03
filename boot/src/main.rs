#![no_main]
#![no_std]

extern crate alloc;

use alloc::alloc::alloc;
use anyhow::{anyhow, Result};
use core::alloc::Layout;
use core::{mem, ptr};
use elf::ElfBytes;
use elf::abi::PT_LOAD;
use elf::endian::LittleEndian;
use uefi::println;
use uefi::prelude::*;
use uefi::boot::MemoryType;
use uefi::fs::FileSystem;
use uefi::mem::memory_map::MemoryMap;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
use x86_64::{addr, PhysAddr, VirtAddr};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags, PhysFrame, Size2MiB, Size4KiB};

const KERNEL_START: u64 = 0xffffffffaf000000;
const KERNEL_END: u64 = 0xffffffffffffffff;
const KERNEL_SIZE: u64 = KERNEL_END - KERNEL_START + 1;

type Framebuffer<'a> = &'a mut [[u32; 1920]; 1080];
type Start = extern "sysv64" fn(Framebuffer) -> !;

#[derive(Clone, Copy)]
struct MemoryPool {
    start: u64,
    end:   u64
}

impl MemoryPool {
    fn find() -> Result<MemoryPool> {
        boot::memory_map(MemoryType::BOOT_SERVICES_DATA)?
            .entries()
            .find_map(|entry| {
                if entry.ty != MemoryType::CONVENTIONAL { return None; }
                if entry.page_count * 4096 < KERNEL_SIZE { return None; }

                let start = addr::align_up(entry.phys_start, Size2MiB::SIZE);
                let end = addr::align_down(entry.phys_start + entry.page_count * 4096, Size2MiB::SIZE);
                if end - start < KERNEL_SIZE { return None; }

                Some(MemoryPool { start, end })
            })
            .ok_or(anyhow!("Not enough memory"))
    }
}

struct FAllocator;

unsafe impl<S: PageSize> FrameAllocator<S> for FAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        unsafe {
            let ptr = alloc(Layout::from_size_align(S::SIZE as usize, S::SIZE as usize).ok()?);
            PhysFrame::from_start_address(PhysAddr::new(ptr as u64)).ok()
        }
    }
}

fn load_kernel(pool: MemoryPool) -> Result<Start> {
    let fs_proto = boot::get_image_file_system(boot::image_handle())?;
    let mut fs = FileSystem::new(fs_proto);

    let buf = fs.read(cstr16!("\\kernel.elf"))?;
    let elf: ElfBytes<LittleEndian> = ElfBytes::minimal_parse(&buf)?;

    elf.segments()
        .ok_or(anyhow!("Elf does not contain segments"))?
        .into_iter()
        .filter(|phdr| phdr.p_type == PT_LOAD)
        .for_each(|phdr| unsafe {
            let src = buf.as_ptr().add(phdr.p_offset as usize);
            let dst = (pool.start + phdr.p_paddr) as *mut u8;

            println!("Copy {} bytes to 0x{:x} -- 0x{:x}", phdr.p_memsz, dst as u64, dst as u64 + phdr.p_memsz);

            ptr::write_bytes(dst, 0, phdr.p_memsz as usize);
            ptr::copy(src, dst, phdr.p_filesz as usize);
        });

    unsafe { Ok(mem::transmute(elf.ehdr.e_entry)) }
}

fn setup_paging(pool: MemoryPool) -> Result<()> {
    unsafe {
        let mut falloc = FAllocator;

        let p4frame: PhysFrame<Size4KiB> = falloc.allocate_frame().ok_or(anyhow!("Unable to allocate page table"))?;
        let p4 = &mut *(p4frame.start_address().as_u64() as *mut PageTable);

        p4.zero();
        let mut page_table = OffsetPageTable::new(p4, VirtAddr::zero());

        let vstart: Page<Size2MiB> = Page::containing_address(VirtAddr::new(KERNEL_START));
        let vend = Page::containing_address(VirtAddr::new(KERNEL_END));
        let pstart = PhysFrame::containing_address(PhysAddr::new(pool.start));
        let pend = PhysFrame::containing_address(PhysAddr::new(pool.start + KERNEL_SIZE - 1));

        let pages = Page::range_inclusive(vstart, vend);
        let frames = PhysFrame::range_inclusive(pstart, pend);

        if pages.len() != frames.len() { return Err(anyhow!("Pages don't match frames")); }

        println!("Mapping 0x{:x} -- 0x{:x} to 0x{:x} -- 0x{:x}", pool.start, pool.start + KERNEL_SIZE - 1, KERNEL_START, KERNEL_END);
        for (page, frame) in pages.zip(frames) {
            page_table
                .map_to(page, frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE, &mut falloc)
                .map_err(|e| anyhow!("{e:?}"))?
                .flush();
        }

        let (oldp4frame, flags) = Cr3::read();

        let oldp4 = &*(oldp4frame.start_address().as_u64() as *const PageTable);
        for (i, entry) in oldp4.iter().enumerate() {
            if !entry.is_unused() { p4[i] = entry.clone(); }
        }

        Cr3::write(p4frame, flags);

        Ok(())
    }
}

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

    let pool = MemoryPool::find()?;
    println!("Kernel Memory Pool: 0x{:x} -- 0x{:x}", pool.start, pool.end);

    let start = load_kernel(pool)?;
    setup_paging(pool)?;

    let fb = setup_video()?;

    unsafe { boot::exit_boot_services(MemoryType::BOOT_SERVICES_DATA); }

    start(fb);

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
