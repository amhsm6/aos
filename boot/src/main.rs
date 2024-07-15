#![no_main]
#![no_std]

use core::{mem, ptr, str};
use uefi::println;
use uefi::prelude::*;
use uefi::fs::FileSystem;
use uefi::table::boot::MemoryType;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
use uefi::proto::device_path::DevicePath;
use uefi::proto::device_path::acpi::{Acpi, Expanded};
use elf::ElfBytes;
use elf::abi::PT_LOAD;
use elf::endian::LittleEndian;

type StartPtr = *const ();
type Start = extern "sysv64" fn(&mut [[u32; 1920]; 1080]) -> !;

fn load_kernel(bs: &BootServices, handle: Handle) -> StartPtr {
    let fs_proto = bs.get_image_file_system(handle).unwrap();
    let mut fs = FileSystem::new(fs_proto);

    let buf = fs.read(cstr16!("\\kernel.elf")).unwrap();
    let elf: ElfBytes<LittleEndian> = ElfBytes::minimal_parse(&buf).unwrap();

    elf.segments().unwrap()
        .into_iter()
        .filter(|x| x.p_type == PT_LOAD)
        .for_each(|phdr| unsafe {
            println!("{} of {} bytes will be copied to 0x{:x}", phdr.p_filesz, phdr.p_memsz, phdr.p_vaddr);

            let dst = phdr.p_vaddr as *mut u8;
            ptr::write_bytes(dst, 0, phdr.p_memsz as usize);

            let src = buf.as_ptr().add(phdr.p_offset as usize);
            ptr::copy(src, dst, phdr.p_filesz as usize);
        });

    elf.ehdr.e_entry as StartPtr
}

fn setup_video(bs: &BootServices) -> *mut [[u32; 1920]; 1080] {
    let gop_handle = bs.get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = bs.open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();

    let mode = gop.modes(bs)
        .filter(|mode| {
            let info = mode.info();
            info.resolution() == (1920, 1080) &&
                info.pixel_format() == PixelFormat::Bgr &&
                info.stride() == 1920
        })
        .nth(0).unwrap();
    gop.set_mode(&mode).unwrap();

    gop.frame_buffer().as_mut_ptr() as *mut [[u32; 1920]; 1080]
}

#[uefi::entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();

    let start_ptr = load_kernel(system_table.boot_services(), handle);
    let start = unsafe { mem::transmute::<StartPtr, Start>(start_ptr) };

    let bs = system_table.boot_services();
    let devpath_handle = bs.get_handle_for_protocol::<DevicePath>().unwrap();
    let devpath = bs.open_protocol_exclusive::<DevicePath>(devpath_handle).unwrap();

    devpath.instance_iter()
        .for_each(|dev| {
            dev.node_iter()
                .for_each(|node| {
                    println!("{node:?}");
                    /*if let Ok(node) = TryInto::<&Acpi>::try_into(node) {
                        println!("{}", node.hid());
                        /*if let Ok(string) = str::from_utf8(node.hid_str()) {
                            println!("{}", string);
                        }*/
                    }*/
                });
        });

    loop {}

    let fb_ptr = setup_video(system_table.boot_services());
    let fb = unsafe { &mut *fb_ptr };

    system_table.exit_boot_services(MemoryType::BOOT_SERVICES_DATA);
    uefi::allocator::exit_boot_services();

    start(fb);

    loop {}
    Status::SUCCESS
}
