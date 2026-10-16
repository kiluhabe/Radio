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
        // 内部プルアップ＋NO接点なので、閉じている間だけLOW。
        // 押しボタン（アクションボタン）でも、位置で開閉する電源スイッチでも同じ。
        self.pin.is_low().unwrap_or(false)
    }
}
