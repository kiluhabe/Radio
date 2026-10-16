//! Thin hairline numeral font for the minimal frequency-only OLED screen.
//!
//! `embedded-graphics`' bundled fonts are all bitmap fonts at a fixed
//! bold weight -- there is no way to get a weight-100 hairline digit out of
//! them at any size. Each digit here is instead a small set of 1px `Line`
//! segments traced along a classic seven-segment layout's centerlines,
//! which is what a genuinely thin digital numeral looks like at this scale
//! (~10 line segments per glyph, per the design brief this is ported from).

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle};

/// One segment as a (start, end) pair in a normalized 0.0..=1.0 box, x
/// narrower than y since digits are taller than wide.
const SEG_T: (f32, f32, f32, f32) = (0.08, 0.0, 0.52, 0.0);
const SEG_TR: (f32, f32, f32, f32) = (0.6, 0.05, 0.6, 0.47);
const SEG_BR: (f32, f32, f32, f32) = (0.6, 0.53, 0.6, 0.95);
const SEG_B: (f32, f32, f32, f32) = (0.08, 1.0, 0.52, 1.0);
const SEG_BL: (f32, f32, f32, f32) = (0.0, 0.53, 0.0, 0.95);
const SEG_TL: (f32, f32, f32, f32) = (0.0, 0.05, 0.0, 0.47);
const SEG_M: (f32, f32, f32, f32) = (0.06, 0.5, 0.54, 0.5);

const ALL_SEGS: [(f32, f32, f32, f32); 7] =
    [SEG_T, SEG_TR, SEG_BR, SEG_B, SEG_BL, SEG_TL, SEG_M];

/// Bitmask into `ALL_SEGS` (bit 0 = T, 1 = TR, 2 = BR, 3 = B, 4 = BL, 5 =
/// TL, 6 = M) for each digit 0-9, standard seven-segment encoding.
const DIGIT_MASKS: [u8; 10] = [
    0b0111111, // 0: T TR BR B BL TL
    0b0000110, // 1: TR BR
    0b1011011, // 2: T TR M BL B
    0b1001111, // 3: T TR M BR B
    0b1100110, // 4: TL M TR BR
    0b1101101, // 5: T TL M BR B
    0b1111101, // 6: T TL M BL BR B
    0b0000111, // 7: T TR BR
    0b1111111, // 8: all
    0b1101111, // 9: T TR BR B TL M
];

/// Width-to-height ratio of one glyph cell (matches the 0.0..=0.6 x-range
/// the segments above are defined in).
const GLYPH_ASPECT: f32 = 0.6;
/// Gap between glyphs, as a fraction of glyph height.
const GLYPH_GAP: f32 = 0.02;
/// Decimal point size and baseline gap, as a fraction of glyph height.
const DOT_SIZE_FRAC: f32 = 0.05;

/// Draws `text` (digits `0`-`9` and `.` only -- anything else is skipped)
/// as hairline numerals with the given cell height in pixels, top-left
/// origin at `pos`. Returns the total width drawn, in pixels, so callers
/// can right-align a label after it.
pub fn draw_number<D>(display: &mut D, text: &str, pos: Point, height: i32) -> u32
where
    D: DrawTarget<Color = embedded_graphics::pixelcolor::BinaryColor>,
{
    let style = PrimitiveStyle::with_stroke(embedded_graphics::pixelcolor::BinaryColor::On, 1);
    let h = height as f32;
    let w = h * GLYPH_ASPECT;
    let gap = h * GLYPH_GAP;
    let dot = (h * DOT_SIZE_FRAC).max(1.0) as u32;

    let mut cursor_x = pos.x as f32;
    for ch in text.chars() {
        if ch == '.' {
            let dot_y = pos.y as f32 + h - dot as f32;
            let _ = embedded_graphics::primitives::Rectangle::new(
                Point::new(cursor_x as i32, dot_y as i32),
                Size::new(dot, dot),
            )
            .into_styled(PrimitiveStyle::with_fill(embedded_graphics::pixelcolor::BinaryColor::On))
            .draw(display);
            cursor_x += w * 0.5 + gap;
            continue;
        }
        let Some(digit) = ch.to_digit(10) else { continue };
        let mask = DIGIT_MASKS[digit as usize];
        for (i, seg) in ALL_SEGS.iter().enumerate() {
            if mask & (1 << i) == 0 {
                continue;
            }
            // Segment x/y are both normalized to `h` -- x's 0.0..=0.6 range
            // (not 0.0..=1.0) is what makes the glyph cell narrower than it
            // is tall; scaling x by `w` here instead would double-apply
            // GLYPH_ASPECT and draw every digit at 60% of its own width.
            let (x0, y0, x1, y1) = *seg;
            let start = Point::new(
                (cursor_x + x0 * h) as i32,
                (pos.y as f32 + y0 * h) as i32,
            );
            let end = Point::new(
                (cursor_x + x1 * h) as i32,
                (pos.y as f32 + y1 * h) as i32,
            );
            let _ = Line::new(start, end).into_styled(style).draw(display);
        }
        cursor_x += w + gap;
    }

    (cursor_x - pos.x as f32).max(0.0) as u32
}

/// Measures what `draw_number` would draw, without drawing it -- used to
/// right-align the numeral block before committing to a draw position.
pub fn measure_number(text: &str, height: i32) -> u32 {
    let h = height as f32;
    let w = h * GLYPH_ASPECT;
    let gap = h * GLYPH_GAP;
    let mut total = 0.0;
    for ch in text.chars() {
        if ch == '.' {
            total += w * 0.5 + gap;
        } else if ch.is_ascii_digit() {
            total += w + gap;
        }
    }
    total.max(0.0) as u32
}
