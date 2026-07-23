use crate::ports::Screen;
use crate::types::{Mode, Settings, FREQ_MAX, FREQ_MIN, FREQ_STEP, MAX_FAVS};

pub struct RadioState {
    pub mode: Mode,
    pub settings: Settings,
    pub fav_index: usize,
    dirty: bool,
    last_change_ms: u32,
}

impl RadioState {
    pub fn new(settings: Settings) -> Self {
        RadioState {
            mode: Mode::Tune,
            settings,
            fav_index: 0,
            dirty: false,
            last_change_ms: 0,
        }
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Tune => Mode::Fav,
            Mode::Fav => Mode::Tune,
        };
        if self.mode == Mode::Fav {
            self.enter_fav_mode();
        }
    }

    pub fn find_fav(&self, freq: u16) -> Option<usize> {
        self.settings.favs.iter().position(|&f| f == freq)
    }

    pub fn add_current_to_favs(&mut self, now_ms: u32) -> bool {
        if self.find_fav(self.settings.freq).is_some() {
            return false;
        }
        if self.settings.favs.push(self.settings.freq).is_err() {
            return false; // 満杯
        }
        self.mark_dirty(now_ms);
        true
    }

    pub fn delete_selected_fav(&mut self, now_ms: u32) {
        if self.settings.favs.is_empty() {
            return;
        }
        self.settings.favs.remove(self.fav_index);
        if self.settings.favs.is_empty() {
            self.fav_index = 0;
            self.mode = Mode::Tune;
        } else if self.fav_index >= self.settings.favs.len() {
            self.fav_index = self.settings.favs.len() - 1;
        }
        if let Some(&freq) = self.settings.favs.get(self.fav_index) {
            self.settings.freq = freq;
        }
        self.mark_dirty(now_ms);
    }

    pub fn enter_fav_mode(&mut self) {
        if self.settings.favs.is_empty() {
            self.fav_index = 0;
            return;
        }
        match self.find_fav(self.settings.freq) {
            Some(idx) => self.fav_index = idx,
            None => {
                self.fav_index = 0;
                self.settings.freq = self.settings.favs[0];
            }
        }
    }

    pub fn apply_encoder_delta(&mut self, delta: i32, now_ms: u32) {
        match self.mode {
            Mode::Tune => {
                let range = (FREQ_MAX - FREQ_MIN + FREQ_STEP) as i32;
                let mut next = self.settings.freq as i32 + delta * FREQ_STEP as i32;
                while next < FREQ_MIN as i32 {
                    next += range;
                }
                while next > FREQ_MAX as i32 {
                    next -= range;
                }
                self.settings.freq = next as u16;
                self.mark_dirty(now_ms);
            }
            Mode::Fav => {
                let count = self.settings.favs.len();
                if count == 0 {
                    return;
                }
                let fc = count as i32;
                let mut new_idx = (self.fav_index as i32 + delta) % fc;
                if new_idx < 0 {
                    new_idx += fc;
                }
                self.fav_index = new_idx as usize;
                self.settings.freq = self.settings.favs[self.fav_index];
                self.mark_dirty(now_ms);
            }
        }
    }

    pub fn mark_dirty(&mut self, now_ms: u32) {
        self.dirty = true;
        self.last_change_ms = now_ms;
    }

    pub fn should_save(&self, now_ms: u32, save_delay_ms: u32) -> bool {
        self.dirty && now_ms.wrapping_sub(self.last_change_ms) >= save_delay_ms
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn current_screen(&self, rssi: u8) -> Screen {
        match self.mode {
            Mode::Tune => Screen::Tune {
                freq: self.settings.freq,
                rssi,
            },
            Mode::Fav => Screen::Fav {
                index: self.fav_index,
                count: self.settings.favs.len(),
                freq: self.settings.favs.get(self.fav_index).copied(),
                rssi,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_mode_toggles_between_tune_and_fav() {
        let mut rs = RadioState::new(Settings::default());
        assert_eq!(rs.mode, Mode::Tune);
        rs.cycle_mode();
        assert_eq!(rs.mode, Mode::Fav);
        rs.cycle_mode();
        assert_eq!(rs.mode, Mode::Tune);
    }

    #[test]
    fn add_current_to_favs_succeeds_once_then_rejects_duplicate() {
        let mut rs = RadioState::new(Settings::default());
        rs.settings.freq = 9010;
        assert!(rs.add_current_to_favs(0));
        assert_eq!(rs.settings.favs.as_slice(), &[9010]);
        assert!(!rs.add_current_to_favs(0)); // 重複は拒否
        assert_eq!(rs.settings.favs.len(), 1);
    }

    #[test]
    fn add_current_to_favs_rejects_when_full() {
        let mut rs = RadioState::new(Settings::default());
        for i in 0..MAX_FAVS {
            rs.settings.freq = FREQ_MIN + i as u16;
            assert!(rs.add_current_to_favs(0));
        }
        rs.settings.freq = FREQ_MAX;
        assert!(!rs.add_current_to_favs(0));
        assert_eq!(rs.settings.favs.len(), MAX_FAVS);
    }

    #[test]
    fn apply_encoder_delta_in_tune_mode_steps_frequency() {
        let mut rs = RadioState::new(Settings::default());
        rs.settings.freq = 9010;
        rs.apply_encoder_delta(1, 0);
        assert_eq!(rs.settings.freq, 9010 + FREQ_STEP);
        rs.apply_encoder_delta(-2, 0);
        assert_eq!(rs.settings.freq, 9010 - FREQ_STEP);
    }

    #[test]
    fn apply_encoder_delta_in_tune_mode_wraps_at_bounds() {
        let mut rs = RadioState::new(Settings::default());
        rs.settings.freq = FREQ_MAX;
        rs.apply_encoder_delta(1, 0);
        assert_eq!(rs.settings.freq, FREQ_MIN);

        rs.settings.freq = FREQ_MIN;
        rs.apply_encoder_delta(-1, 0);
        assert_eq!(rs.settings.freq, FREQ_MAX);
    }

    #[test]
    fn apply_encoder_delta_in_fav_mode_wraps_fav_index() {
        let mut rs = RadioState::new(Settings::default());
        rs.settings.favs.push(9010).unwrap();
        rs.settings.favs.push(9020).unwrap();
        rs.settings.favs.push(9030).unwrap();
        rs.enter_fav_mode();
        rs.mode = Mode::Fav;
        assert_eq!(rs.fav_index, 0);
        rs.apply_encoder_delta(1, 0);
        assert_eq!(rs.fav_index, 1);
        rs.apply_encoder_delta(-2, 0);
        assert_eq!(rs.fav_index, 2); // (1 - 2) mod 3 = -1 mod 3 = 2
    }

    #[test]
    fn should_save_true_only_after_delay_since_dirty() {
        let mut rs = RadioState::new(Settings::default());
        assert!(!rs.should_save(1000, 2000));
        rs.apply_encoder_delta(1, 1000); // dirty化, last_change_ms=1000
        assert!(!rs.should_save(2500, 2000)); // 1500ms経過、まだ足りない
        assert!(rs.should_save(3000, 2000)); // 2000ms経過
        rs.clear_dirty();
        assert!(!rs.should_save(9999, 2000));
    }

    #[test]
    fn current_screen_reflects_tune_mode() {
        let mut rs = RadioState::new(Settings::default());
        rs.settings.freq = 9010;
        match rs.current_screen(42) {
            Screen::Tune { freq, rssi } => {
                assert_eq!(freq, 9010);
                assert_eq!(rssi, 42);
            }
            _ => panic!("expected Screen::Tune"),
        }
    }

    #[test]
    fn current_screen_reflects_fav_mode_with_empty_favs() {
        let mut rs = RadioState::new(Settings::default());
        rs.mode = Mode::Fav;
        match rs.current_screen(0) {
            Screen::Fav { index, count, freq, .. } => {
                assert_eq!(index, 0);
                assert_eq!(count, 0);
                assert_eq!(freq, None);
            }
            _ => panic!("expected Screen::Fav"),
        }
    }
}
