#![no_std]
#![no_main]

mod adapters;

use core::cell::RefCell;

use rp_pico as bsp;

use bsp::entry;
use bsp::hal::{
    clocks::init_clocks_and_plls,
    fugit::RateExtU32,
    gpio::{FunctionI2C, PullUp},
    pac,
    sio::Sio,
    watchdog::Watchdog,
    Timer, I2C,
};
use defmt_rtt as _;
use panic_probe as _;
use radio_core::{
    ButtonDebouncer, ButtonEvent, ButtonInput, Clock, Display, Encoder, Mode, RadioState, Storage,
    Tuner,
};

/// Matches `radio-rp.ino`'s `SAVE_DELAY_MS`: settle time after the last
/// change before a flash write is committed, so turning the encoder doesn't
/// hammer flash on every detent.
const SAVE_DELAY_MS: u32 = 2000;

/// Matches `radio-rp.ino`'s `FIXED_VOL`: the RDA5807's own digital
/// attenuator is pinned once at init, volume is a physical pot downstream.
const FIXED_VOL: u8 = 15;

/// `embedded-hal-bus`'s `RefCellDevice` (used below for the display) only
/// implements the embedded-hal 1.0 `I2c` trait. `Rda5807Tuner` is built
/// against rda5807m 0.1.0, which predates embedded-hal 1.0 and needs the
/// legacy `embedded_hal_02::blocking::i2c::{Read, Write, WriteRead}` traits
/// instead (see the `embedded-hal-02` comment in Cargo.toml). `embedded-hal-
/// bus` 0.2 has no equivalent wrapper for those legacy traits, so this small
/// shim borrows the same shared `RefCell<I2C>` and forwards to the
/// underlying rp2040-hal `I2C` peripheral's own legacy-trait implementation
/// (rp2040-hal implements both trait generations directly on the same type).
struct LegacyI2cDevice<'a, T>(&'a RefCell<T>);

impl<'a, T, E> embedded_hal_02::blocking::i2c::Write for LegacyI2cDevice<'a, T>
where
    T: embedded_hal_02::blocking::i2c::Write<Error = E>,
{
    type Error = E;
    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), E> {
        self.0.borrow_mut().write(addr, bytes)
    }
}

impl<'a, T, E> embedded_hal_02::blocking::i2c::Read for LegacyI2cDevice<'a, T>
where
    T: embedded_hal_02::blocking::i2c::Read<Error = E>,
{
    type Error = E;
    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), E> {
        self.0.borrow_mut().read(addr, buffer)
    }
}

impl<'a, T, E> embedded_hal_02::blocking::i2c::WriteRead for LegacyI2cDevice<'a, T>
where
    T: embedded_hal_02::blocking::i2c::WriteRead<Error = E>,
{
    type Error = E;
    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), E> {
        self.0.borrow_mut().write_read(addr, bytes, buffer)
    }
}

/// Busy-wait millisecond delay built on `radio_core::Clock`, matching
/// `radio-rp.ino`'s `delay(50);` between `rx.setFrequency()` and
/// `rx.setVolume()` in `radioPowerOn()`. No SysTick delay peripheral is
/// wired up in this composition root (only a free-running `Timer`), so a
/// spin-wait against `now_ms()` is the simplest faithful equivalent.
fn busy_wait_ms(clock: &adapters::clock::HalClock, ms: u32) {
    let start = clock.now_ms();
    while clock.now_ms().wrapping_sub(start) < ms {}
}

/// Reads RSSI and redraws the screen, matching `radio-rp.ino`'s
/// `drawScreen()` (which itself refreshes `cachedRssi` from `rx.getRssi()`
/// every time it draws). Called only at the same points the Arduino
/// `loop()` calls `drawScreen()` -- not unconditionally every iteration --
/// so idle RSSI-only changes don't spuriously redraw either.
fn redraw<T: Tuner, D: Display>(radio: &RadioState, tuner: &mut T, display: &mut D) {
    let rssi = tuner.rssi();
    display.render(&radio.current_screen(rssi));
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let clocks = init_clocks_and_plls(
        12_000_000u32,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda = pins.gpio4.reconfigure::<FunctionI2C, PullUp>();
    let scl = pins.gpio5.reconfigure::<FunctionI2C, PullUp>();
    let i2c = I2C::i2c0(
        pac.I2C0,
        sda,
        scl,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    // OLED and tuner share one I2C0 bus; both need mutable access to the
    // same peripheral instance, so it lives behind a RefCell and each
    // adapter gets its own borrowing handle (embedded-hal-bus's
    // RefCellDevice for the display's embedded-hal 1.0 `I2c` needs, the
    // local LegacyI2cDevice shim above for the tuner's legacy needs).
    let i2c_cell = RefCell::new(i2c);
    let tuner_i2c = LegacyI2cDevice(&i2c_cell);
    let display_i2c = embedded_hal_bus::i2c::RefCellDevice::new(&i2c_cell);

    let mut tuner = adapters::tuner::Rda5807Tuner::new(tuner_i2c);
    let mut display = adapters::display::Ssd1306Display::new(display_i2c);

    let pin_sw = pins.gpio6.into_pull_up_input();
    let mut button_input = adapters::button::GpioButton::new(pin_sw);

    let pin_a = pins.gpio2.into_pull_up_input();
    let pin_b = pins.gpio3.into_pull_up_input();
    let mut encoder = adapters::encoder::GpioEncoder::new(pin_a, pin_b);

    let clock = adapters::clock::HalClock::new(timer);
    let mut storage = adapters::storage::FlashStorage::new();

    let initial_settings = storage.load().unwrap_or_default();
    let mut radio = RadioState::new(initial_settings);

    // Mirrors setup(): power up muted, tune to the restored/default
    // frequency, then unmute at the fixed volume.
    tuner.power_up();
    tuner.set_mute(true);
    tuner.set_frequency(radio.settings.freq);
    tuner.set_volume(FIXED_VOL);
    tuner.set_mute(false);

    // Mirrors setup()'s final `drawScreen();` call.
    redraw(&radio, &mut tuner, &mut display);

    let mut debouncer = ButtonDebouncer::new();
    let mut last_encoder_pos = encoder.position();
    let mut power_on = true;

    loop {
        let now = clock.now_ms();
        let raw_pressed = button_input.is_pressed();
        let ev = debouncer.poll(raw_pressed, now);

        // Mirrors loop()'s `if (ev == BTN_LONG) { ... } else if (powerOn) { ... }`.
        if ev == ButtonEvent::Long {
            if power_on {
                // Mirrors radioPowerOff(): force-save if dirty (regardless
                // of the settle delay), mute + power down the tuner, blank
                // the display.
                if radio.should_save(now, 0) {
                    storage.save(&radio.settings);
                    radio.clear_dirty();
                }
                tuner.set_mute(true);
                tuner.power_down();
                display.power_save(true);
                power_on = false;
            } else {
                // Mirrors radioPowerOn(): restore display, power up +
                // reconfigure the tuner, retune, unmute, redraw, and
                // resync the encoder's last-known position so a stale
                // delta accumulated while off isn't applied post power-on.
                display.power_save(false);
                tuner.power_up();
                tuner.set_mute(true);
                tuner.set_frequency(radio.settings.freq);
                busy_wait_ms(&clock, 50);
                tuner.set_volume(FIXED_VOL);
                tuner.set_mute(false);
                redraw(&radio, &mut tuner, &mut display);
                last_encoder_pos = encoder.position();
                power_on = true;
            }
        } else if power_on {
            match ev {
                ButtonEvent::Short => {
                    // Mirrors cycleMode() + drawScreen().
                    radio.cycle_mode();
                    redraw(&radio, &mut tuner, &mut display);
                }
                ButtonEvent::Double => {
                    // Mirrors the BTN_DOUBLE branch: add-to-favs in Tune
                    // mode, delete-selected in Fav mode.
                    match radio.mode {
                        Mode::Tune => {
                            radio.add_current_to_favs(now);
                        }
                        Mode::Fav => {
                            radio.delete_selected_fav(now);
                            // deleteSelectedFav() retunes the hardware
                            // itself when favorites remain; pushing here
                            // unconditionally is a harmless no-op when the
                            // list emptied out (freq is left unchanged).
                            tuner.set_frequency(radio.settings.freq);
                        }
                    }
                    redraw(&radio, &mut tuner, &mut display);
                }
                _ => {}
            }
        }

        // Mirrors the encoder-handling block, gated by `powerOn` the same
        // way loop() returns early while off. `raw_pressed` stands in for
        // Arduino's debounced `stableBtnState == LOW` freeze check --
        // ButtonDebouncer intentionally doesn't expose its internal stable
        // state, and the push switch is wired to its own GPIO separate from
        // the encoder (see radio-rp.ino's pin comment), so the two states
        // differ by at most one debounce window with no observable effect.
        if power_on {
            let pos = encoder.position();
            if raw_pressed {
                last_encoder_pos = pos;
            } else if pos != last_encoder_pos {
                let delta = pos - last_encoder_pos;
                last_encoder_pos = pos;
                radio.apply_encoder_delta(delta, now);
                tuner.set_frequency(radio.settings.freq);
                redraw(&radio, &mut tuner, &mut display);
            }
        }

        if radio.should_save(now, SAVE_DELAY_MS) {
            storage.save(&radio.settings);
            radio.clear_dirty();
        }
    }
}
