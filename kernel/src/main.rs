#![no_main]
#![no_std]

mod acpi;
mod alloc;
mod video;
mod input;

use core::panic::PanicInfo;

use input::keyboard::Keyboard;
use video::framebuffer::Framebuffer;
use video::printer::{Color, Printer};

#[no_mangle]
#[link_section = ".ltext.astart"]
extern "sysv64" fn astart(acpi: usize, fb: Framebuffer<'static>) -> ! {
    Printer::init_global(fb, video::fonts::SF_PRO, 40.0, Color::new(255.0, 255.0, 255.0));
    
    println!("0x{acpi:x}");
    acpi::parse(acpi);

    let mut kb = Keyboard::new();

    loop {
        if let Some(x) = kb.read_char() {
            print!("{x}");
        }
    }
}

#[used]
#[link_section = ".ltext.kstart"]
static KSTART: [u8; 44] = [
    0x0f, 0x20, 0xd8, 0x48, 0xc7, 0xc3, 0xff, 0x07, 0x00, 0x00, 0x48, 0x21,
    0xc3, 0x48, 0x89, 0xf8, 0x48, 0x89, 0xf7, 0x48, 0x89, 0xd6, 0x48, 0x09,
    0xd8, 0x0f, 0x22, 0xd8, 0x48, 0xc7, 0xc4, 0xff, 0xff, 0xff, 0xff, 0x48,
    0xc7, 0xc0, 0x00, 0x00, 0x00, 0xaf, 0xff, 0xe0
];

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("[Panic]: {}", info);

    loop {}
}
