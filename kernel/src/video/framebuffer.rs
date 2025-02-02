pub const HRES: usize = 1920;
pub const VRES: usize = 1080;

#[repr(C, align(4))]
#[derive(Clone, Copy, PartialOrd, PartialEq, Eq, Ord)]
pub struct Pixel {
    pub blue:  u8,
    pub green: u8,
    pub red:   u8
}

pub type Framebuffer<'a> = &'a mut [[Pixel; HRES]; VRES];
