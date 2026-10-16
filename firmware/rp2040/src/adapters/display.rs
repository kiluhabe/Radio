use core::fmt::Write;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_hal::i2c::I2c;
use heapless::String;
use radio_core::Screen;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;

use crate::adapters::vecfont;

/// Height in px of the vector numeral glyph cell -- leaves headroom below
/// for the "MHz" label within the 64px panel.
const NUMERAL_HEIGHT: i32 = 44;

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
    /// favourite index, see the design brief this implements).
    ///
    /// The signal-strength "degradation" effect (noise dots, tear bands,
    /// numeral flicker) has been removed. It required redrawing on a timer
    /// rather than only on state change, and every redraw pushes the whole
    /// 1024-byte framebuffer over an I2C bus that has no pull-up resistors
    /// fitted -- suspected of starving the main loop's input polling and of
    /// coupling into the analog audio path. Signal strength is currently not
    /// displayed at all; reintroduce it only via something that does not need
    /// a free-running redraw.
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

            let num_w = vecfont::measure_number(num.as_str(), NUMERAL_HEIGHT);
            let x = 128 - 4 - num_w as i32;
            vecfont::draw_number(&mut self.display, num.as_str(), Point::new(x, 2), NUMERAL_HEIGHT);

            let mhz_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
            let _ = Text::new("MHz", Point::new(x, NUMERAL_HEIGHT + 12), mhz_style)
                .draw(&mut self.display);
        }

        let _ = self.display.flush();
    }
}
