#![no_main]
#![no_std]

mod alloc;
mod video;
mod input;

use video::{Framebuffer, Printer, Color};
use input::Keyboard;

use core::panic::PanicInfo;

#[no_mangle]
extern fn _start(fb: Framebuffer<'static>) -> ! {
    Printer::init_global(fb, &video::fonts::CYLBURN, 60.0, Color::new(212.0, 78.0, 159.0));

    let mut kb = Keyboard::new();

    loop {
        if let Some(x) = kb.read_char() {
            print!("{x}");
        }
    }

    loop {}
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("[Panic]: {}", info);

    loop {}
}
