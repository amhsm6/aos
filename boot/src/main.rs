#![no_main]
#![no_std]

extern crate alloc;

use alloc::format;
use alloc::alloc::alloc;
use alloc::boxed::Box;
use alloc::string::ToString;
use core::alloc::Layout;
use core::{mem, ptr};
use core::error::Error;
use elf::ElfBytes;
use elf::abi::PT_LOAD;
use elf::endian::LittleEndian;
use uefi::prelude::*;
use uefi::{boot, println};
use uefi::boot::MemoryType;
use uefi::fs::FileSystem;
use uefi::mem::memory_map::MemoryMap;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
use x86_64::{addr, PhysAddr, VirtAddr};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags, PhysFrame, Size2MiB, Translate};
use x86_64::structures::paging::frame::PhysFrameRange;
use x86_64::structures::paging::page::PageRange;

type Framebuffer<'a> = &'a mut [[u32; 1920]; 1080];
type Start = extern "sysv64" fn(Framebuffer) -> !;

#[derive(Clone, Copy)]
struct MemoryPool {
    start: u64,
    end: u64
}

impl MemoryPool {
    fn find() -> Result<MemoryPool, Box<dyn Error>> {
        let mmap = boot::memory_map(MemoryType::BOOT_SERVICES_DATA)?;
        let pool = mmap.entries()
            .find(|entry| entry.ty == MemoryType::CONVENTIONAL && entry.page_count >= 1024 * 1024 * 1024 / 4096)
            .ok_or("Not enough memory")?;

        let start = addr::align_up(pool.phys_start, Size2MiB::SIZE);
        let end = addr::align_down(pool.phys_start + pool.page_count * 4096, Size2MiB::SIZE);

        Ok(MemoryPool { start, end })
    }
}

struct FAllocator;

unsafe impl<S: PageSize> FrameAllocator<S> for FAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        unsafe {
            let ptr = alloc(Layout::from_size_align(S::SIZE as usize, S::SIZE as usize).ok()?);
            let x = PhysFrame::from_start_address(PhysAddr::new(ptr as u64)).ok();
            println!("{x:?}");
            x
        }
    }
}

fn load_kernel(pool: MemoryPool) -> Result<Start, Box<dyn Error>> {
    let fs_proto = boot::get_image_file_system(boot::image_handle())?;
    let mut fs = FileSystem::new(fs_proto);

    let buf = fs.read(cstr16!("\\kernel.elf"))?;
    let elf: ElfBytes<LittleEndian> = ElfBytes::minimal_parse(&buf)?;

    elf.segments()
        .ok_or("Elf does not contain segments")?
        .into_iter()
        .filter(|phdr| phdr.p_type == PT_LOAD)
        .for_each(|phdr| unsafe {
            let src = buf.as_ptr().add(phdr.p_offset as usize);
            let dst = (pool.start + phdr.p_paddr) as *mut u8;

            println!("[0x{:x} -- 0x{:x}] Copy {} bytes", dst as u64, dst as u64 + phdr.p_memsz, phdr.p_memsz);

            ptr::write_bytes(dst, 0, phdr.p_memsz as usize);
            ptr::copy(src, dst, phdr.p_filesz as usize);
        });

    unsafe { Ok(mem::transmute(elf.ehdr.e_entry)) }
}

fn setup_paging(pool: MemoryPool) -> Result<(), Box<dyn Error>> {
    unsafe {
        let (pt4, _) = Cr3::read();
        let pt4ptr = pt4.start_address().as_u64() as *mut PageTable;
        let mut page_table = OffsetPageTable::new(&mut *pt4ptr, VirtAddr::zero());

        let vstart = Page::from_start_address(VirtAddr::new(0xffff800000000000))
            .map_err(|e| e.to_string())?;

        let vend = Page::from_start_address(VirtAddr::new(0xffff800000000000 + pool.end - pool.start))
            .map_err(|e| e.to_string())?;

        let pstart = PhysFrame::from_start_address(PhysAddr::new(pool.start))
            .map_err(|e| e.to_string())?;
        
        let pend = PhysFrame::from_start_address(PhysAddr::new(pool.end))
            .map_err(|e| e.to_string())?;

        let pages: PageRange<Size2MiB> = Page::range(vstart, vend);
        let frames: PhysFrameRange<Size2MiB> = PhysFrame::range(pstart, pend);

        let mut falloc = FAllocator;
        for (page, frame) in pages.zip(frames) {
            println!("{page:?} {frame:?}");

            page_table
                .map_to(page, frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE, &mut falloc)
                .map_err(|e| format!("{e:?}"))?
                .flush();
        }

        println!("{:?}", page_table.translate(VirtAddr::new(0xffff800000000000)));
        loop {}

        Ok(())
    }
}

fn setup_video<'a>() -> Result<Framebuffer<'a>, Box<dyn Error>> {
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)?;

    let mode = gop.modes()
        .find(|mode| {
            let info = mode.info();
            info.resolution() == (1920, 1080) &&
                info.pixel_format() == PixelFormat::Bgr &&
                info.stride() == 1920
        })
        .ok_or("No graphic modes available")?;

    gop.set_mode(&mode)?;

    let ptr = gop.frame_buffer().as_mut_ptr() as *mut [[u32; 1920]; 1080];
    unsafe { Ok(&mut *ptr) }
}

fn init() -> Result<(), Box<dyn Error>> {
    uefi::helpers::init()?;
    system::with_stdout(|stdout| stdout.clear())?;

    let pool = MemoryPool::find()?;
    println!("Memory Pool: 0x{:x} -- 0x{:x}", pool.start, pool.end);

    let start = load_kernel(pool)?;
    setup_paging(pool)?;

    let fb = setup_video()?;

    unsafe { boot::exit_boot_services(MemoryType::BOOT_SERVICES_DATA); }

    start(fb);
}

#[uefi::entry]
fn main() -> Status {
    match init() {
        Ok(_) => {
            loop {}
        }
        Err(err) => {
            println!("ERROR: {}", err);
            loop {}
        }
    }

    Status::SUCCESS
}
