use core::fmt::Write;
use anyhow::{anyhow, Result};
use rusttype::{Font, Scale, Point};

use crate::drivers::video::framebuffer::{Framebuffer, Pixel, HRES};

pub struct Color {
    r: f32,
    g: f32,
    b: f32
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b }
    }
}

pub struct Printer<'a> {
    fb:    Framebuffer<'a>,
    font:  Font<'a>,
    scale: Scale,
    color: Color,
    pos:   Point<f32>
}

impl<'a> Printer<'a> {
    pub fn new(fb: Framebuffer<'a>, bytes: &'a [u8], scale: f32, color: Color) -> Result<Printer<'a>> {
        let font = Font::try_from_bytes(bytes).ok_or(anyhow!("Error parsing font"))?;
        let scale = Scale::uniform(scale);

        let v_metrics = font.v_metrics(scale);
        let pos = rusttype::point(0.0, v_metrics.ascent + v_metrics.line_gap);

        Ok(Printer { fb, font, scale, color, pos })
    }

    pub fn newline(&mut self) {
        let v_metrics = self.font.v_metrics(self.scale);
        self.pos.x = 0.0;
        self.pos.y += v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    }

    pub fn put_char(&mut self, c: char) -> Result<()> {
        if c == '\n' {
            self.newline();
            return Ok(());
        }

        let glyph = self.font.glyph(c).scaled(self.scale);
        let h_metrics = glyph.h_metrics();
        let mut glyph = glyph.positioned(self.pos);

        if let Some(bounds) = glyph.pixel_bounding_box() {
            if bounds.max.x >= HRES as i32 {
                self.newline();
            }

            if bounds.min.x < 0 {
                self.pos = self.pos + rusttype::vector(-bounds.min.x as f32, 0.0);
            }

            glyph.set_position(self.pos);
            let bounds = glyph.pixel_bounding_box().expect("Impossible");

            glyph.draw(|x, y, a| {
                let x = bounds.min.x as usize + x as usize;
                let y = bounds.min.y as usize + y as usize;

                let p = Pixel {
                    red:   (self.color.r * a) as u8,
                    green: (self.color.g * a) as u8,
                    blue:  (self.color.b * a) as u8
                };

                self.fb[y][x] = self.fb[y][x].max(p);
            });
        }

        self.pos = self.pos + rusttype::vector(h_metrics.advance_width, 0.0);

        Ok(())
    }
}

impl Write for Printer<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        s.chars()
            .map(|c| self.put_char(c))
            .try_for_each(|r| r.map_err(|_| core::fmt::Error))
    }
}
