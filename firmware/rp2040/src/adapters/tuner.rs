use embedded_hal::i2c::I2c;

/// RDA5807's 7-bit I2C address in random-access mode (the mode that allows
/// addressing any register directly, register-byte-then-data, rather than
/// sequential-access mode's fixed write-starts-at-REG02 behavior).
const RDA5807_I2C_ADDRESS: u8 = 0x11;

const REG_CONFIG: u8 = 0x02;
const REG_TUNING: u8 = 0x03;
const REG_VOLUME: u8 = 0x05;
const REG_RSSI: u8 = 0x0B;

// REG02 (config) bits actually used here. Names/values per the RDA5807
// datasheet (see also rda5807m::register_address::ConfigBitFlags, which
// this module intentionally no longer depends on -- see module doc below).
const DHIZ: u16 = 1 << 15; // audio output high-Z: 0=high-Z, 1=normal
const DMUTE: u16 = 1 << 14; // mute: 0=muted, 1=normal
const BASS: u16 = 1 << 12; // bass boost enable
const SEEKUP: u16 = 1 << 9; // seek direction (unused here, no seek feature)
const RDS: u16 = 1 << 3; // RDS/RBDS enable
const NEW_DEMOD: u16 = 1 << 2; // "NEW" bit: improved demodulation method
const ENABLE: u16 = 1; // power-up enable

/// REG02 baseline with every bit *except* DMUTE fixed: DHIZ/BASS/SEEKUP/
/// RDS/NEW_DEMOD/ENABLE. `set_mute()` ORs in DMUTE (or not) on top of this
/// instead of a read-modify-write -- see module doc for why.
const CONFIG_BASELINE_MUTE_BIT_CLEAR: u16 = DHIZ | BASS | SEEKUP | RDS | NEW_DEMOD | ENABLE;

// REG03 (tuning) bits.
const BAND_76_108_MHZ: u16 = 0b10 << 2; // bits 3-2 = 10: "world wide" 76-108MHz
const SPACE_100_KHZ: u16 = 0b00; // bits 1-0 = 00: 100kHz channel spacing
const TUNE: u16 = 1 << 4; // tune-start strobe; chip clears it when done
const CHAN_SHIFT: u8 = 6; // CHAN occupies bits 15-6

/// BAND/SPACE bits for Japan's wide FM band (76-108MHz) at 100kHz channel
/// spacing. Matches radio-core's FREQ_MIN=7600/FREQ_MAX=10800
/// (76.00-108.00MHz) and FREQ_STEP=10 (0.01MHz units => 100kHz steps).
const BAND_SPACE_JAPAN_WIDE_100KHZ: u16 = BAND_76_108_MHZ | SPACE_100_KHZ;

// REG05 (volume) bits.
const LNA_PORT_SEL_SHIFT: u8 = 6;
const SEEK_TH_SHIFT: u8 = 8;
const VOLUME_MASK: u16 = 0b1111;

/// REG05 baseline for the non-volume fields (LNA port select, seek
/// threshold). Values taken directly from the hardware-verified PU2CLR
/// Arduino library's `powerUp()` (`LNA_PORT_SEL=2` i.e. LNAP single-ended
/// input, `SEEKTH=8`), since nothing in this codebase's own power-up
/// sequence establishes a baseline for this register otherwise.
/// `set_volume()` ORs the requested volume into this fixed baseline.
const VOLUME_BASELINE_NO_VOLUME_BITS: u16 = (2 << LNA_PORT_SEL_SHIFT) | (8 << SEEK_TH_SHIFT);

// REG0B (RSSI status) bits.
const RSSI_MASK: u16 = 0b1111_1110_0000_0000;
const RSSI_SHIFT: u8 = 9;

/// Minimal, self-contained RDA5807 register driver.
///
/// This does not wrap the `rda5807m` crate. That crate's `set_frequency`/
/// `mute`/`set_volume` are read-modify-write: each reads its target
/// register (REG03/REG02/REG05) back over I2C before merging in the new
/// bits (`get_band_and_spacing()`/`update_register()` in rda5807m's
/// `lib.rs`). On this hardware (M5807PA, an RDA5807P-compatible clone) that
/// readback does not reflect what was actually written, which silently
/// corrupted every field outside the one being changed -- observed as the
/// RSSI meter responding correctly to tuning (REG0B, a genuine read-back
/// status register, is unaffected) while the demodulated station never
/// changed (REG03's CHAN bits were corrupted on every write). Confirmed to
/// be a firmware-side regression, not a hardware fault: the same physical
/// board tuned correctly under the prior Arduino firmware (via the PU2CLR
/// RDA5807 library), which -- for exactly its tuning/config registers -- never reads
/// REG02/REG03/REG05 back either; it keeps its own in-RAM shadow of each
/// register and writes the whole thing on every change. This driver
/// follows that same proven pattern directly: every register write here is
/// a complete, known-good value computed from a fixed baseline plus the
/// one field being changed, with no register read anywhere in the write
/// path. `rssi()` is the only read, and REG0B is a true status register
/// (0x0A-0x0F range) unaffected by this issue.
///
/// Keeping this self-contained inside the one file that implements
/// `radio_core::Tuner` (rather than patching around a third-party crate's
/// behavior) preserves the actual abstraction boundary the ports/adapters
/// architecture cares about: `radio-core` never knows or depends on RDA5807
/// register layout either way, so swapping to a different tuner chip later
/// only means writing a different `radio_core::Tuner` adapter -- unrelated
/// to whether this one happens to hand-roll its register access or lean on
/// a crate.
pub struct Rda5807Tuner<I2C> {
    i2c: I2C,
}

impl<I2C: I2c> Rda5807Tuner<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Rda5807Tuner { i2c }
    }

    fn write_register(&mut self, register: u8, value: u16) {
        let _ = self.i2c.write(
            RDA5807_I2C_ADDRESS,
            &[register, (value >> 8) as u8, (value & 0xFF) as u8],
        );
    }

    fn read_register(&mut self, register: u8) -> u16 {
        let mut buf = [0u8; 2];
        let _ = self
            .i2c
            .write_read(RDA5807_I2C_ADDRESS, &[register], &mut buf);
        ((buf[0] as u16) << 8) | buf[1] as u16
    }
}

impl<I2C: I2c> radio_core::Tuner for Rda5807Tuner<I2C> {
    fn power_up(&mut self) {
        // Matches what the old `Rda5708m::start()` wrote to REG02
        // (DHIZ|DMUTE|BASS|SEEKUP|RDS|NEW|ENABLE); the composition root
        // calls `set_mute(true)` immediately after this anyway, so the
        // DMUTE bit here is transient.
        self.write_register(REG_CONFIG, CONFIG_BASELINE_MUTE_BIT_CLEAR | DMUTE);
        self.write_register(REG_TUNING, BAND_SPACE_JAPAN_WIDE_100KHZ);
    }

    fn power_down(&mut self) {
        self.write_register(REG_CONFIG, 0);
    }

    fn set_frequency(&mut self, khz_x100: u16) {
        // radio-core's Settings::freq (and this trait's khz_x100 param) is
        // in units of 0.01MHz (FREQ_MIN=7600 => 76.00MHz => CHAN=0).
        let freq_khz = khz_x100 as u32 * 10;
        let chan = ((freq_khz - 76_000) / 100) as u16;
        let value = BAND_SPACE_JAPAN_WIDE_100KHZ | (chan << CHAN_SHIFT) | TUNE;
        self.write_register(REG_TUNING, value);
    }

    fn set_mute(&mut self, mute: bool) {
        // DMUTE=1 means "normal" (unmuted) per the datasheet, so this
        // trait's `mute` flag maps *inversely* onto DMUTE -- matching this
        // project's existing, already-relied-upon `radio_core::Tuner::
        // set_mute` convention (`set_mute(true)` mutes, `set_mute(false)`
        // unmutes) at the register level.
        let dmute_bit = if mute { 0 } else { DMUTE };
        self.write_register(REG_CONFIG, CONFIG_BASELINE_MUTE_BIT_CLEAR | dmute_bit);
    }

    fn set_volume(&mut self, vol: u8) {
        let vol = (vol as u16) & VOLUME_MASK;
        self.write_register(REG_VOLUME, VOLUME_BASELINE_NO_VOLUME_BITS | vol);
    }

    fn rssi(&mut self) -> u8 {
        let reg = self.read_register(REG_RSSI);
        ((reg & RSSI_MASK) >> RSSI_SHIFT) as u8
    }
}
