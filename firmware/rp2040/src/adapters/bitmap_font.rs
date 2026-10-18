use embedded_graphics::image::{Image, ImageRaw};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;

/// One pre-rasterized glyph. `x_off`/`y_off` place the 1bpp bitmap's top-left
/// relative to the pen origin on the baseline (`y_off` is negative above it).
pub struct Glyph {
    pub ch: char,
    pub width: u32,
    pub x_off: i32,
    pub y_off: i32,
    pub advance: i32,
    pub data: &'static [u8],
}

pub struct Font {
    /// Tallest ink above the baseline, in px.
    pub ascent: i32,
    pub glyphs: &'static [Glyph],
}

impl Font {
    fn glyph(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| g.ch == ch)
    }

    /// Left ink edge of `text` relative to the pen origin; 0 if empty.
    pub fn ink_left(&self, text: &str) -> i32 {
        text.chars()
            .find_map(|c| self.glyph(c))
            .map_or(0, |g| g.x_off)
    }

    /// Sum of advances for `text`; unknown chars are skipped.
    pub fn advance_width(&self, text: &str) -> i32 {
        text.chars().filter_map(|c| self.glyph(c)).map(|g| g.advance).sum()
    }
}

/// Draws `text` with the pen starting at `origin` (on the baseline).
pub fn draw<D>(display: &mut D, font: &Font, text: &str, origin: Point)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let mut pen = origin.x;
    for g in text.chars().filter_map(|c| font.glyph(c)) {
        let raw = ImageRaw::<BinaryColor>::new(g.data, g.width);
        let _ = Image::new(&raw, Point::new(pen + g.x_off, origin.y + g.y_off)).draw(display);
        pen += g.advance;
    }
}
