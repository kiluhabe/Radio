use embedded_hal::digital::InputPin;

pub struct GpioButton<P> {
    pin: P,
}

impl<P: InputPin> GpioButton<P> {
    pub fn new(pin: P) -> Self {
        GpioButton { pin }
    }
}

impl<P: InputPin> radio_core::ButtonInput for GpioButton<P> {
    fn is_pressed(&mut self) -> bool {
        // GP6はINPUT_PULLUPでNO接点。押下時にLOWになるためis_low()==trueが「押されている」
        self.pin.is_low().unwrap_or(false)
    }
}
