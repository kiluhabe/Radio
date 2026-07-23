use embedded_hal::digital::InputPin;
use radio_core::QuadratureDecoder;

pub struct GpioEncoder<PA, PB> {
    pin_a: PA,
    pin_b: PB,
    decoder: QuadratureDecoder,
    position: i32,
}

impl<PA: InputPin, PB: InputPin> GpioEncoder<PA, PB> {
    pub fn new(pin_a: PA, pin_b: PB) -> Self {
        GpioEncoder {
            pin_a,
            pin_b,
            decoder: QuadratureDecoder::new(),
            position: 0,
        }
    }
}

impl<PA: InputPin, PB: InputPin> radio_core::Encoder for GpioEncoder<PA, PB> {
    fn position(&mut self) -> i32 {
        let a = self.pin_a.is_high().unwrap_or(false);
        let b = self.pin_b.is_high().unwrap_or(false);
        self.position += self.decoder.step(a, b);
        self.position
    }
}
