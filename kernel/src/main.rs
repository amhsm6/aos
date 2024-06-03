#![no_main]
#![no_std]

mod alloc;
mod video;

use video::{Framebuffer, Printer, Color};

use core::panic::PanicInfo;

#[no_mangle]
extern fn _start(fb: Framebuffer<'static>) -> ! {
    Printer::init_global(fb, &video::fonts::CYLBURN, 60.0, Color::new(212.0, 78.0, 159.0));

    println!("Alexandra Yanikova");

    loop {}
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("[Panic]: {}", info);

    loop {}
}
