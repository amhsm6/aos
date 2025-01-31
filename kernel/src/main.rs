#![no_main]
#![no_std]

mod alloc;
mod video;
mod input;

use input::keyboard::Keyboard;
use video::framebuffer::Framebuffer;
use video::printer::{Color, Printer};
use x86_64::structures::gdt::GlobalDescriptorTable;

use core::panic::PanicInfo;

#[no_mangle]
extern "sysv64" fn _start(fb: Framebuffer<'static>) -> ! {
    Printer::init_global(fb, &video::fonts::CYLBURN, 60.0, Color::new(212.0, 78.0, 159.0));

    let ptr = x86_64::instructions::tables::sgdt();
    let gdt = unsafe { &*ptr.base.as_ptr::<GlobalDescriptorTable>() };

    println!("{gdt:?}");

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
