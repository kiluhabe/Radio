use core::fmt::Write;
use embedded_graphics::mono_font::ascii::FONT_9X18_BOLD;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use embedded_hal::i2c::I2c;
use heapless::String;
use radio_core::Screen;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::*;
use ssd1306::Ssd1306;

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

    fn render(&mut self, screen: &Screen) {
        self.display.clear_buffer();
        let style = MonoTextStyle::new(&FONT_9X18_BOLD, BinaryColor::On);

        let mut line1: String<24> = String::new();
        let mut line2: String<24> = String::new();
        let rssi;

        match *screen {
            Screen::Tune { freq, rssi: r } => {
                rssi = r;
                let _ = write!(line1, "TUNE");
                let _ = write!(line2, "{}.{:02} MHz", freq / 100, freq % 100);
            }
            Screen::Fav { index, count, freq, rssi: r } => {
                rssi = r;
                if count == 0 {
                    let _ = write!(line1, "FAV (empty)");
                } else {
                    let _ = write!(line1, "FAV {}/{}", index + 1, count);
                }
                if let Some(f) = freq {
                    let _ = write!(line2, "{}.{:02} MHz", f / 100, f % 100);
                }
            }
        }

        let _ = Text::new(line1.as_str(), Point::new(0, 10), style).draw(&mut self.display);
        let _ = Text::new(line2.as_str(), Point::new(0, 40), style).draw(&mut self.display);

        let fill_w = (rssi as u32).min(126);
        let _ = Rectangle::new(Point::new(0, 58), Size::new(128, 6))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(&mut self.display);
        if fill_w > 0 {
            let _ = Rectangle::new(Point::new(1, 59), Size::new(fill_w, 4))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(&mut self.display);
        }

        let _ = self.display.flush();
    }
}
