pub const FREQ_MIN: u16 = 7600;
pub const FREQ_MAX: u16 = 10800;
pub const FREQ_STEP: u16 = 10;
pub const FREQ_INIT: u16 = 8130;
pub const MAX_FAVS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Tune,
    Fav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonEvent {
    None,
    Short,
    Long,
    Double,
}

pub struct Settings {
    pub freq: u16,
    pub favs: heapless::Vec<u16, MAX_FAVS>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            freq: FREQ_INIT,
            favs: heapless::Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_uses_freq_init_and_empty_favs() {
        let s = Settings::default();
        assert_eq!(s.freq, FREQ_INIT);
        assert!(s.favs.is_empty());
    }

    #[test]
    fn favs_rejects_push_beyond_max_favs() {
        let mut favs: heapless::Vec<u16, MAX_FAVS> = heapless::Vec::new();
        for i in 0..MAX_FAVS {
            assert!(favs.push(FREQ_MIN + i as u16).is_ok());
        }
        assert!(favs.push(FREQ_MIN + MAX_FAVS as u16).is_err());
    }
}
