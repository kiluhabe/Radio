use embedded_hal_02::blocking::i2c::{Read, Write, WriteRead};
use rda5807m::register_address::{Register, TuningBitFlag};
use rda5807m::{Address, Rda5708m};

/// REG03 (tuning register) BAND/SPACE bits for Japan's wide FM band
/// (76-108MHz) at 100kHz channel spacing. This is the Rust-side equivalent
/// of the Arduino PU2CLR library's `rx.setBand(2); rx.setSpace(0);`, and
/// matches radio-core's FREQ_MIN=7600/FREQ_MAX=10800 (76.00-108.00MHz) and
/// FREQ_STEP=10 (0.01MHz units => 100kHz steps).
///
/// Values per rda5807m's `register_address.rs`:
///   BAND_76_108_MHZ = 0b10 << 2 (bits 3-2 = 10: "world wide")
///   SPACE_100_KHZ   = 0b00      (bits 1-0 = 00: 100kHz)
const BAND_SPACE_JAPAN_WIDE_100KHZ: u16 = TuningBitFlag::BAND_76_108_MHZ | TuningBitFlag::SPACE_100_KHZ;

/// rda5807m's `Address` newtype wraps a `pub(crate) u8`, so external crates
/// cannot read the address value back out of `Address::default()`. Its
/// value is fixed by the crate at `DEVICE_BASE_RANDOM_ADDRESS = 0b10001`
/// (see rda5807m src/lib.rs), which is also the standard RDA5807 7-bit I2C
/// address in random-access mode. Hardcoded here so we can address the bus
/// directly when the crate's own driver has been temporarily destroyed.
const RDA5807_I2C_ADDRESS: u8 = 0x11;

pub struct Rda5807Tuner<I2C> {
    // Wrapped in `Option` so `configure_band_and_space` can `take()` it,
    // `destroy()` it to reclaim the raw I2C bus for a register write the
    // crate doesn't expose, and rebuild it with `Rda5708m::new()`.
    driver: Option<Rda5708m<I2C>>,
}

impl<I2C, E> Rda5807Tuner<I2C>
where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
{
    pub fn new(i2c: I2C) -> Self {
        Rda5807Tuner {
            driver: Some(Rda5708m::new(i2c, Address::default())),
        }
    }

    fn driver(&mut self) -> &mut Rda5708m<I2C> {
        self.driver
            .as_mut()
            .expect("Rda5807Tuner driver missing outside configure_band_and_space")
    }

    /// rda5807m 0.1.0 exposes no setter for REG03's BAND/SPACE bits (only
    /// the `get_band_and_spacing()` getter), and the crate's own
    /// `write_register`/`update_register` are private `fn`s, not `pub` or
    /// even `pub(crate)`-visible-enough to reach from outside the crate.
    /// So there is no way to write REG03 through the crate's public API.
    ///
    /// Fallback: `destroy()` the driver to reclaim the raw `I2C` bus, then
    /// issue the same wire-format write the crate's private
    /// `write_register` uses internally (register byte, then data high
    /// byte, then data low byte — see rda5807m src/lib.rs
    /// `fn write_register`: `i2c.write(address, &[register, (data >> 8) as
    /// u8, (data & 0xff) as u8])`), before rebuilding the driver with
    /// `Rda5708m::new()`.
    ///
    /// Must be called *after* `driver.start()`: `start()` itself
    /// unconditionally writes REG03 = `BAND_87_108_MHZ | SPACE_100_KHZ`
    /// (see rda5807m src/lib.rs `start()`), clobbering whatever was there
    /// before. Once set here, later `set_frequency()` calls preserve these
    /// bits: it reads REG03 first via `get_band_and_spacing()` and only
    /// masks in CHAN/TUNE bits when writing back
    /// (`update_register_by_old` computes `(new_value & mask) | (old_value
    /// & !mask)`), so BAND/SPACE survive every subsequent tune.
    fn configure_band_and_space(&mut self) -> Result<(), rda5807m::Error<E>> {
        let driver = self
            .driver
            .take()
            .expect("Rda5807Tuner driver missing outside configure_band_and_space");
        let mut i2c = driver.destroy();
        let result = i2c.write(
            RDA5807_I2C_ADDRESS,
            &[
                Register::RDA5807M_REG_TUNING,
                (BAND_SPACE_JAPAN_WIDE_100KHZ >> 8) as u8,
                (BAND_SPACE_JAPAN_WIDE_100KHZ & 0xFF) as u8,
            ],
        );
        self.driver = Some(Rda5708m::new(i2c, Address::default()));
        result.map_err(rda5807m::Error::I2C)
    }
}

impl<I2C, E> radio_core::Tuner for Rda5807Tuner<I2C>
where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
{
    fn power_up(&mut self) {
        let _ = self.driver().start();
        let _ = self.configure_band_and_space();
    }

    fn power_down(&mut self) {
        let _ = self.driver().stop();
    }

    fn set_frequency(&mut self, khz_x100: u16) {
        // radio-core's Settings::freq (and this trait's khz_x100 param) is
        // in units of 0.01MHz (FREQ_MIN=7600 => 76.00MHz), but rda5807m's
        // set_frequency expects plain kHz (76.00MHz => 76_000) since it
        // computes `(freq - 76_000) / spacing_khz` internally. Convert by
        // x10, or set_frequency underflows/mistunes.
        let freq_khz = khz_x100 as u32 * 10;
        let _ = self.driver().set_frequency(freq_khz);
    }

    fn set_mute(&mut self, mute: bool) {
        let _ = self.driver().mute(mute);
    }

    fn set_volume(&mut self, vol: u8) {
        let _ = self.driver().set_volume(vol);
    }

    fn rssi(&mut self) -> u8 {
        self.driver().get_rssi().unwrap_or(0)
    }
}
