// Positive values indicate clockwise rotation. If the encoder reads
// backwards on real hardware, flip the sign of every entry in this table.
const TRANSITION_TABLE: [i8; 16] = [
     0,  1, -1,  0,
    -1,  0,  0,  1,
     1,  0,  0, -1,
     0, -1,  1,  0,
];

pub struct QuadratureDecoder {
    prev_state: u8,
    raw_accum: i32,
}

impl QuadratureDecoder {
    pub fn new() -> Self {
        QuadratureDecoder {
            prev_state: 0,
            raw_accum: 0,
        }
    }

    pub fn step(&mut self, a: bool, b: bool) -> i32 {
        let new_state = ((a as u8) << 1) | (b as u8);
        let index = ((self.prev_state << 2) | new_state) as usize;
        self.raw_accum += TRANSITION_TABLE[index] as i32;
        self.prev_state = new_state;

        if self.raw_accum >= 4 {
            self.raw_accum -= 4;
            1
        } else if self.raw_accum <= -4 {
            self.raw_accum += 4;
            -1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_full_clockwise_detent_returns_delta_one() {
        let mut dec = QuadratureDecoder::new();
        // CW一巡: 00 -> 01 -> 11 -> 10 -> 00
        assert_eq!(dec.step(false, false), 0); // 初期状態と同じなので変化なし
        assert_eq!(dec.step(false, true), 0);  // 4分の1ステップ目、まだ1ディテント未満
        assert_eq!(dec.step(true, true), 0);   // 2/4
        assert_eq!(dec.step(true, false), 0);  // 3/4
        assert_eq!(dec.step(false, false), 1); // 4/4、1ディテント分の変化を返す
    }

    #[test]
    fn one_full_counter_clockwise_detent_returns_delta_minus_one() {
        let mut dec = QuadratureDecoder::new();
        // CCW一巡: 00 -> 10 -> 11 -> 01 -> 00
        assert_eq!(dec.step(false, false), 0);
        assert_eq!(dec.step(true, false), 0);
        assert_eq!(dec.step(true, true), 0);
        assert_eq!(dec.step(false, true), 0);
        assert_eq!(dec.step(false, false), -1);
    }

    #[test]
    fn no_change_returns_zero() {
        let mut dec = QuadratureDecoder::new();
        assert_eq!(dec.step(false, false), 0);
        assert_eq!(dec.step(false, false), 0);
    }
}
