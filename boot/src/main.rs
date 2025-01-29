#![no_main]
#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use uefi::mem::memory_map::MemoryMap;
use core::{mem, ptr};
use core::error::Error;
use elf::ElfBytes;
use elf::abi::PT_LOAD;
use elf::endian::LittleEndian;
use uefi::prelude::*;
use uefi::{boot, println};
use uefi::boot::MemoryType;
use uefi::fs::FileSystem;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};

type StartPtr = *const ();
type Start = extern "sysv64" fn(&mut [[u32; 1920]; 1080]) -> !;

fn load_kernel() -> Result<StartPtr, Box<dyn Error>> {
    let fs_proto = boot::get_image_file_system(boot::image_handle())?;
    let mut fs = FileSystem::new(fs_proto);

    let buf = fs.read(cstr16!("\\kernel.elf"))?;
    let elf: ElfBytes<LittleEndian> = ElfBytes::minimal_parse(&buf)?;

    elf.segments()
        .ok_or("elf does not contain segments")?
        .into_iter()
        .filter(|phdr| phdr.p_type == PT_LOAD)
        .for_each(|phdr| unsafe {
            println!("[0x{:x}] Init {} bytes", phdr.p_paddr, phdr.p_memsz);
            println!("[0x{:x}] Copy {} bytes", phdr.p_paddr, phdr.p_filesz);

            let dst = phdr.p_vaddr as *mut u8;
            ptr::write_bytes(dst, 0, phdr.p_memsz as usize);

            let src = buf.as_ptr().add(phdr.p_offset as usize);
            ptr::copy(src, dst, phdr.p_filesz as usize);
        });

    fn display_mem(typ: MemoryType, name: &str) -> Result<(), Box<dyn Error>> {
        let mmap = boot::memory_map(typ)?;
        for entry in mmap.entries() {
            println!("Memory {}: PHYS 0x{:x} VIRT 0x{:x}: {} pages", name, entry.phys_start, entry.virt_start, entry.page_count);
        }

        Ok(())
    }

    display_mem(MemoryType::LOADER_CODE, "LOADER_CODE")?;
    display_mem(MemoryType::LOADER_DATA, "LOADER_DATA")?;
    display_mem(MemoryType::BOOT_SERVICES_CODE, "BOOT_SERVICES_CODE")?;
    display_mem(MemoryType::BOOT_SERVICES_DATA, "BOOT_SERVICES_DATA")?;
    display_mem(MemoryType::RUNTIME_SERVICES_CODE, "RUNTIME_SERVICES_CODE")?;
    display_mem(MemoryType::RUNTIME_SERVICES_DATA, "RUNTIME_SERVICES_DATA")?;
    display_mem(MemoryType::ACPI_NON_VOLATILE, "ACPI_NON_VOLATILE")?;
    display_mem(MemoryType::ACPI_RECLAIM, "ACPI_RECLAIM")?;
    display_mem(MemoryType::MMIO, "MMIO")?;
    display_mem(MemoryType::MMIO_PORT_SPACE, "MMIO_PORT_SPACE")?;

    boot::stall(15000000);

    Ok(elf.ehdr.e_entry as StartPtr)
}

fn setup_video() -> Result<*mut [[u32; 1920]; 1080], Box<dyn Error>> {
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)?;

    let mode = gop.modes()
        .filter(|mode| {
            let info = mode.info();
            info.resolution() == (1920, 1080) &&
                info.pixel_format() == PixelFormat::Bgr &&
                info.stride() == 1920
        })
        .nth(0)
        .ok_or("no graphic modes available")?;
    gop.set_mode(&mode)?;

    Ok(gop.frame_buffer().as_mut_ptr() as *mut [[u32; 1920]; 1080])
}

#[uefi::entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let start_ptr = load_kernel().unwrap();
    let start = unsafe { mem::transmute::<StartPtr, Start>(start_ptr) };

    let fb_ptr = setup_video().unwrap();
    let fb = unsafe { &mut *fb_ptr };

    unsafe { boot::exit_boot_services(MemoryType::BOOT_SERVICES_DATA); }

    start(fb);

    loop {}
    Status::SUCCESS
}
