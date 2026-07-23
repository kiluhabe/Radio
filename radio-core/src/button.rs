use crate::types::ButtonEvent;

const BTN_DEBOUNCE_MS: u32 = 50;
const LONG_PRESS_MS: u32 = 1500;
const DOUBLE_WINDOW_MS: u32 = 400;

pub struct ButtonDebouncer {
    last_debounce: u32,
    last_reading: bool,
    stable_pressed: bool,
    press_started_at: u32,
    long_armed: bool,
    last_short_release_at: u32,
    awaiting_double: bool,
    double_consumed: bool,
}

impl ButtonDebouncer {
    pub fn new() -> Self {
        ButtonDebouncer {
            last_debounce: 0,
            last_reading: false,
            stable_pressed: false,
            press_started_at: 0,
            long_armed: false,
            last_short_release_at: 0,
            awaiting_double: false,
            double_consumed: false,
        }
    }

    pub fn poll(&mut self, raw_pressed: bool, now_ms: u32) -> ButtonEvent {
        if raw_pressed != self.last_reading {
            self.last_debounce = now_ms;
            self.last_reading = raw_pressed;
        }

        let mut state_changed = false;
        if now_ms.wrapping_sub(self.last_debounce) > BTN_DEBOUNCE_MS
            && raw_pressed != self.stable_pressed
        {
            self.stable_pressed = raw_pressed;
            state_changed = true;
        }

        if state_changed && self.stable_pressed {
            if self.awaiting_double {
                self.awaiting_double = false;
                self.double_consumed = true;
                self.long_armed = false;
                return ButtonEvent::Double;
            }
            self.press_started_at = now_ms;
            self.long_armed = true;
        }

        if self.long_armed
            && self.stable_pressed
            && now_ms.wrapping_sub(self.press_started_at) >= LONG_PRESS_MS
        {
            self.long_armed = false;
            return ButtonEvent::Long;
        }

        if state_changed && !self.stable_pressed {
            if self.double_consumed {
                self.double_consumed = false;
                return ButtonEvent::None;
            }
            if self.long_armed {
                self.long_armed = false;
                self.awaiting_double = true;
                self.last_short_release_at = now_ms;
            }
        }

        if self.awaiting_double
            && now_ms.wrapping_sub(self.last_short_release_at) > DOUBLE_WINDOW_MS
        {
            self.awaiting_double = false;
            return ButtonEvent::Short;
        }

        ButtonEvent::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_press_emits_short_after_double_window_elapses() {
        let mut btn = ButtonDebouncer::new();
        assert_eq!(btn.poll(false, 0), ButtonEvent::None);
        assert_eq!(btn.poll(true, 10), ButtonEvent::None);
        assert_eq!(btn.poll(true, 70), ButtonEvent::None); // debounce確定、long_armed開始
        assert_eq!(btn.poll(false, 100), ButtonEvent::None);
        assert_eq!(btn.poll(false, 160), ButtonEvent::None); // release debounce確定、awaiting_double開始
        assert_eq!(btn.poll(false, 600), ButtonEvent::Short); // 440ms経過、ダブルプレス窓を超えた
    }

    #[test]
    fn holding_past_long_press_ms_emits_long() {
        let mut btn = ButtonDebouncer::new();
        assert_eq!(btn.poll(false, 0), ButtonEvent::None);
        assert_eq!(btn.poll(true, 10), ButtonEvent::None);
        assert_eq!(btn.poll(true, 70), ButtonEvent::None);
        assert_eq!(btn.poll(true, 1600), ButtonEvent::Long); // 70から1530ms保持、1500ms超え
    }

    #[test]
    fn two_presses_within_window_emit_double() {
        let mut btn = ButtonDebouncer::new();
        assert_eq!(btn.poll(false, 0), ButtonEvent::None);
        assert_eq!(btn.poll(true, 10), ButtonEvent::None);
        assert_eq!(btn.poll(true, 70), ButtonEvent::None);
        assert_eq!(btn.poll(false, 100), ButtonEvent::None);
        assert_eq!(btn.poll(false, 160), ButtonEvent::None); // awaiting_double開始
        assert_eq!(btn.poll(true, 200), ButtonEvent::None);
        assert_eq!(btn.poll(true, 260), ButtonEvent::Double); // 160から100ms後の再押下、400ms以内
    }
}
