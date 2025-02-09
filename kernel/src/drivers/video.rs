pub mod framebuffer;
pub mod printer;
pub mod fonts;

use core::fmt::{Arguments, Write};

use framebuffer::Framebuffer;
use printer::{Printer, Color};

static mut PRINTER: Option<Printer<'static>> = None;

impl Printer<'static> {
    pub fn init_global(fb: Framebuffer<'static>, bytes: &'static [u8], scale: f32, color: Color) {
        unsafe {
            PRINTER = Some(Printer::new(fb, bytes, scale, color).unwrap());
        }
    }
}

pub fn _print(args: Arguments) {
    unsafe {
        let printer = PRINTER.as_mut().unwrap();
        printer.write_fmt(args).unwrap();
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::drivers::video::_print(core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::drivers::video::_print(core::format_args!("{}{}", core::format_args!($($arg)*), "\n")));
}
