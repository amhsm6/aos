#![no_main]
#![no_std]

mod alloc;
mod video;

use video::{Framebuffer, Printer};

use core::panic::PanicInfo;

#[no_mangle]
extern fn _start(fb: Framebuffer<'static>) -> ! {
    Printer::init_global(fb, &video::fonts::CYLBURN, 60.0);

    println!("FooBar");

    loop {}
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("[PANIC]: {}", info);

    loop {}
}
