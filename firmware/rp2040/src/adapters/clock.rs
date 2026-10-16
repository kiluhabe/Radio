use waveshare_rp2040_zero::hal::Timer;

pub struct HalClock {
    timer: Timer,
}

impl HalClock {
    pub fn new(timer: Timer) -> Self {
        HalClock { timer }
    }
}

impl radio_core::Clock for HalClock {
    fn now_ms(&self) -> u32 {
        (self.timer.get_counter().ticks() / 1000) as u32
    }
}
