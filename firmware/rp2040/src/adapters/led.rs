//! WS2812B status LED on the front panel (GP9, driven from PIO0).
//!
//! Generic over `SmartLedsWrite` rather than naming the concrete
//! `Ws2812Direct<PIO0, SM0, _>` type, so the PIO plumbing stays in main.rs's
//! composition root.

use smart_leds_trait::{SmartLedsWrite, RGB8};

/// Kept low deliberately: a 5050 WS2812B at full drive is far too bright for
/// a panel indicator viewed head-on, and the panel diffuser only spreads the
/// light, it does not attenuate it.
const LEVEL: u8 = 24;

pub const OFF: RGB8 = RGB8 { r: 0, g: 0, b: 0 };
pub const TUNE: RGB8 = RGB8 {
    r: LEVEL,
    g: LEVEL / 2,
    b: 0,
};
pub const FAV: RGB8 = RGB8 {
    r: 0,
    g: LEVEL / 3,
    b: LEVEL,
};

pub struct StatusLed<W> {
    ws: W,
    last: Option<RGB8>,
}

impl<W: SmartLedsWrite<Color = RGB8>> StatusLed<W> {
    pub fn new(ws: W) -> Self {
        StatusLed { ws, last: None }
    }

    /// Writes only when the colour actually changes.
    ///
    /// `Ws2812Direct::write` returns as soon as the bits are queued into the
    /// PIO FIFO and does not enforce the WS2812B's ~50us inter-frame reset
    /// gap. Skipping no-op writes keeps back-to-back frames from running
    /// together, which would otherwise be latched as garbage.
    pub fn set(&mut self, color: RGB8) {
        if self.last == Some(color) {
            return;
        }
        let _ = self.ws.write([color].into_iter());
        self.last = Some(color);
    }
}
