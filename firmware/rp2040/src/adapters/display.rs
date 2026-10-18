use core::fmt::Write;
use embedded_graphics::prelude::*;
use embedded_hal::i2c::I2c;
use heapless::String;
use radio_core::Screen;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;

use crate::adapters::bitmap_font;
use crate::adapters::font_data::{LABEL, NUMERAL};

/// Right edge of the numerals' advance box. Aligning on advances (all digits
/// share one advance) rather than on ink keeps the string from shifting
/// sideways as the last digit changes while tuning; the trailing '0' still
/// ends its ink 2px inside the panel edge.
const NUM_RIGHT: i32 = 128;
const NUM_TOP: i32 = 2;
const LABEL_BASELINE: i32 = 56;

pub struct Ssd1306Display<I2C> {
    display: Ssd1306<
        ssd1306::prelude::I2CInterface<I2C>,
        ssd1306::size::DisplaySize128x64,
        BufferedGraphicsMode<ssd1306::size::DisplaySize128x64>,
    >,
}

impl<I2C: I2c> Ssd1306Display<I2C> {
    pub fn new(i2c: I2C) -> Self {
        let interface = ssd1306::I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().ok();
        Ssd1306Display { display }
    }
}

impl<I2C: I2c> radio_core::Display for Ssd1306Display<I2C> {
    fn power_save(&mut self, save: bool) {
        let _ = self.display.set_display_on(!save);
    }

    /// Mode-less minimal screen: the frequency is the only thing drawn (TUNE
    /// and FAV render identically -- there is deliberately no mode tag or
    /// favourite index).
    ///
    /// Callers must redraw only on state change, never on a timer: each
    /// redraw pushes the whole 1024-byte framebuffer over an I2C bus that has
    /// no pull-up resistors fitted, which is suspected of starving input polling
    /// and of coupling into the analog audio path.
    fn render(&mut self, screen: &Screen) {
        self.display.clear_buffer();

        let freq = match *screen {
            Screen::Tune { freq, .. } => Some(freq),
            Screen::Fav { freq, .. } => freq,
        };

        if let Some(freq) = freq {
            let mut num: String<8> = String::new();
            // FREQ_STEP is 0.1MHz -- one decimal digit is all the resolution
            // there is; a second digit would always read 0.
            let _ = write!(num, "{}.{}", freq / 100, (freq / 10) % 10);

            let ink_l = NUMERAL.ink_left(num.as_str());
            let x = NUM_RIGHT - NUMERAL.advance_width(num.as_str());
            let origin = Point::new(x, NUM_TOP + NUMERAL.ascent);
            bitmap_font::draw(&mut self.display, &NUMERAL, num.as_str(), origin);

            // Label's left ink edge sits 1px right of the numerals' left ink edge.
            let lab_l = LABEL.ink_left("MHz");
            let lab_origin = Point::new(x + ink_l + 1 - lab_l, LABEL_BASELINE);
            bitmap_font::draw(&mut self.display, &LABEL, "MHz", lab_origin);
        }

        let _ = self.display.flush();
    }
}
