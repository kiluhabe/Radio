// Positive values indicate clockwise rotation. Signs are flipped from the
// textbook table: as wired (A=GP12, B=GP13 via the encoder's own cable) the
// textbook orientation read backwards on real hardware.
const TRANSITION_TABLE: [i8; 16] = [
     0, -1,  1,  0,
     1,  0,  0, -1,
    -1,  0,  0,  1,
     0,  1, -1,  0,
];

/// Raw A/B transitions per mechanical detent.
///
/// Measured on the actual hardware: one click emitted two steps when this
/// was 2, so each detent really walks a full 00->01->11->10->00 quadrature
/// cycle (4 transitions), not the half-cycle the EC10E1220501 datasheet's
/// waveform diagram suggested. One click now moves exactly one `FREQ_STEP`
/// (0.1MHz).
const COUNTS_PER_DETENT: i32 = 4;

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

        if self.raw_accum >= COUNTS_PER_DETENT {
            self.raw_accum -= COUNTS_PER_DETENT;
            1
        } else if self.raw_accum <= -COUNTS_PER_DETENT {
            self.raw_accum += COUNTS_PER_DETENT;
            -1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 実機のEC10E1220501は1クリックでフルサイクル(4遷移)を歩く。
    // テーブルは符号反転済みなので、00 -> 10 -> 11 -> 01 -> 00 の向きが
    // 正方向(周波数が上がる向き)になる。

    #[test]
    fn one_click_returns_delta_one() {
        let mut dec = QuadratureDecoder::new();
        assert_eq!(dec.step(false, false), 0); // 初期状態と同じなので変化なし
        assert_eq!(dec.step(true, false), 0);  // 1/4
        assert_eq!(dec.step(true, true), 0);   // 2/4
        assert_eq!(dec.step(false, true), 0);  // 3/4
        assert_eq!(dec.step(false, false), 1); // 4/4、1クリック分
    }

    #[test]
    fn one_click_the_other_way_returns_delta_minus_one() {
        let mut dec = QuadratureDecoder::new();
        assert_eq!(dec.step(false, false), 0);
        assert_eq!(dec.step(false, true), 0);  // 1/4
        assert_eq!(dec.step(true, true), 0);   // 2/4
        assert_eq!(dec.step(true, false), 0);  // 3/4
        assert_eq!(dec.step(false, false), -1); // 4/4
    }

    #[test]
    fn two_consecutive_clicks_return_delta_one_each() {
        let mut dec = QuadratureDecoder::new();
        assert_eq!(dec.step(false, false), 0);
        for _ in 0..2 {
            assert_eq!(dec.step(true, false), 0);
            assert_eq!(dec.step(true, true), 0);
            assert_eq!(dec.step(false, true), 0);
            assert_eq!(dec.step(false, false), 1);
        }
    }

    #[test]
    fn no_change_returns_zero() {
        let mut dec = QuadratureDecoder::new();
        assert_eq!(dec.step(false, false), 0);
        assert_eq!(dec.step(false, false), 0);
    }
}
