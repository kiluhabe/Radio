use crate::types::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Tune {
        freq: u16,
    },
    Fav {
        index: usize,
        count: usize,
        freq: Option<u16>,
    },
}

pub trait Tuner {
    fn power_up(&mut self);
    fn power_down(&mut self);
    fn set_frequency(&mut self, khz_x100: u16);
    fn set_mute(&mut self, mute: bool);
    fn set_volume(&mut self, vol: u8);
}

pub trait Display {
    fn power_save(&mut self, save: bool);
    fn render(&mut self, screen: &Screen);
}

pub trait Encoder {
    fn position(&mut self) -> i32;
}

pub trait ButtonInput {
    fn is_pressed(&mut self) -> bool;
}

pub trait Storage {
    fn load(&mut self) -> Option<Settings>;
    fn save(&mut self, settings: &Settings);
}

pub trait Clock {
    fn now_ms(&self) -> u32;
}
