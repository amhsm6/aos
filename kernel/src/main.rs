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
extern "sysv64" fn astart(fb: Framebuffer<'static>, acpi: *const ()) -> ! {
    Printer::init_global(fb, video::fonts::CYLBURN, 60.0, Color::new(212.0, 78.0, 159.0));
    
    acpi::parse(acpi);

    let mut kb = Keyboard::new();

    loop {
        if let Some(x) = kb.read_char() {
            print!("{x}");
        }
    }
}

#[used]
#[link_section = ".ltext._start"]
static KERNEL_START: [u8; 16] = [0x48, 0xc7, 0xc4, 0xff, 0xff, 0xff, 0xff, 0x48, 0xc7, 0xc0, 0x00, 0x00, 0x00, 0xaf, 0xff, 0xe0];

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("[Panic]: {}", info);

    loop {}
}
