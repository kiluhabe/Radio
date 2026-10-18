#![no_std]
#![no_main]

mod adapters;

use core::cell::RefCell;

use waveshare_rp2040_zero as bsp;

use bsp::entry;
use bsp::hal::{
    clocks::init_clocks_and_plls,
    fugit::RateExtU32,
    gpio::{FunctionI2C, FunctionPio0, PullUp},
    pac,
    pio::PIOExt,
    sio::Sio,
    watchdog::Watchdog,
    Timer, I2C,
};
// Anonymous: `.freq()` needs the HAL's `Clock` trait in scope, but the name
// would collide with `radio_core::Clock` imported just below.
use bsp::hal::Clock as _;
use panic_halt as _;
use radio_core::{
    ButtonDebouncer, ButtonEvent, ButtonInput, Clock, Display, Encoder, Mode, RadioState, Storage,
    Tuner,
};
use ws2812_pio::Ws2812Direct;

/// Settle time after the last change before a flash write is committed, so
/// turning the disc doesn't hammer flash on every step.
const SAVE_DELAY_MS: u32 = 2000;

/// The RDA5807's own digital attenuator is pinned once at init; volume is a
/// physical pot downstream of the audio output.
const FIXED_VOL: u8 = 15;

/// Busy-wait millisecond delay built on `radio_core::Clock`, for the settle
/// gap the RDA5807 needs between `set_frequency()` and `set_volume()` during
/// power-up. No SysTick delay peripheral is wired up in this composition root
/// (only a free-running `Timer`), so a spin-wait against `now_ms()` is the
/// simplest equivalent.
fn busy_wait_ms(clock: &adapters::clock::HalClock, ms: u32) {
    let start = clock.now_ms();
    while clock.now_ms().wrapping_sub(start) < ms {}
}

/// Redraws the screen. Called only at the points that actually change what is
/// displayed -- a redraw pushes the whole framebuffer over I2C and blocks the
/// loop's input polling while it does.
fn redraw<D: Display>(radio: &RadioState, display: &mut D) {
    display.render(&radio.current_screen());
}

/// Commits settings to flash once they have been dirty for `delay_ms`
/// (0 forces a save of any pending change).
fn save_if_due<S: Storage>(radio: &mut RadioState, storage: &mut S, now: u32, delay_ms: u32) {
    if radio.should_save(now, delay_ms) {
        storage.save(&radio.settings);
        radio.clear_dirty();
    }
}

/// Restores the display, powers up and reconfigures the tuner, retunes,
/// unmutes and redraws.
fn power_up<T: Tuner, D: Display>(
    radio: &RadioState,
    tuner: &mut T,
    display: &mut D,
    clock: &adapters::clock::HalClock,
) {
    display.power_save(false);
    tuner.power_up();
    tuner.set_mute(true);
    tuner.set_frequency(radio.settings.freq);
    busy_wait_ms(clock, 50);
    tuner.set_volume(FIXED_VOL);
    tuner.set_mute(false);
    redraw(radio, display);
}

/// Force-saves if dirty (regardless of the settle delay), then mutes and
/// powers down the tuner and blanks the display.
fn power_down<T: Tuner, D: Display, S: Storage>(
    radio: &mut RadioState,
    tuner: &mut T,
    display: &mut D,
    storage: &mut S,
    now: u32,
) {
    save_if_due(radio, storage, now, 0);
    tuner.set_mute(true);
    tuner.power_down();
    display.power_save(true);
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
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

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let ws = Ws2812Direct::new(
        pins.gp9.into_function::<FunctionPio0>(),
        &mut pio,
        sm0,
        // PIO state machines are clocked from clk_sys, not clk_peri.
        clocks.system_clock.freq(),
    );
    let mut led = adapters::led::StatusLed::new(ws);
    led.set(adapters::led::OFF);

    // GP26/GP27 is a valid I2C1 SDA/SCL pair on the RP2040 -- matches
    // hardware/diylc/radio-rpv2.diy, where the OLED and the FM module share
    // this one bus.
    let sda = pins.gp26.reconfigure::<FunctionI2C, PullUp>();
    let scl = pins.gp27.reconfigure::<FunctionI2C, PullUp>();
    let i2c = I2C::i2c1(
        pac.I2C1,
        sda,
        scl,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    // OLED and tuner share one I2C1 bus; both need mutable access to the
    // same peripheral instance, so it lives behind a RefCell and each
    // adapter gets its own embedded-hal-bus RefCellDevice handle onto it.
    let i2c_cell = RefCell::new(i2c);
    let tuner_i2c = embedded_hal_bus::i2c::RefCellDevice::new(&i2c_cell);
    let display_i2c = embedded_hal_bus::i2c::RefCellDevice::new(&i2c_cell);

    let mut tuner = adapters::tuner::Rda5807Tuner::new(tuner_i2c);
    let mut display = adapters::display::Ssd1306Display::new(display_i2c);

    // Mechanical quadrature encoder (A=GP12, B=GP13), decoded via
    // `radio_core::QuadratureDecoder` -- see quadrature.rs for the
    // detent/pulse assumptions this expects.
    let mut disc = adapters::encoder::GpioEncoder::new(
        pins.gp12.into_pull_up_input(),
        pins.gp13.into_pull_up_input(),
    );

    // Sole button: short press cycles TUNE <-> FAV, double press adds/deletes
    // a favourite. It is the only way to switch modes.
    let mut action_btn = adapters::button::GpioButton::new(pins.gp11.into_pull_up_input());

    // The volume pot's built-in switch is the master power control: closed
    // (pin pulled low) means powered. Unlike the action button this is a
    // level, not an event -- there is no press/release to debounce, only a
    // position to follow.
    let mut power_sw = adapters::button::GpioButton::new(pins.gp10.into_pull_up_input());

    let clock = adapters::clock::HalClock::new(timer);
    let mut storage = adapters::storage::FlashStorage::new();

    let initial_settings = storage.load().unwrap_or_default();
    let mut radio = RadioState::new(initial_settings);

    let mut debouncer = ButtonDebouncer::new();

    // Start in the powered-off state regardless of the switch: the loop's
    // edge detection below brings the tuner and display up on the first
    // iteration if the switch is already closed, so the power-up sequence
    // lives in exactly one place.
    let mut power_on = false;
    display.power_save(true);

    // Seeds the disc's resting position so the first read is a no-op rather
    // than one huge jump from zero.
    let mut last_disc_pos = disc.position();

    loop {
        let now = clock.now_ms();
        let sw_on = power_sw.is_pressed();

        if sw_on != power_on {
            if sw_on {
                power_up(&radio, &mut tuner, &mut display, &clock);
                // Resync the disc so anything that moved while off isn't
                // applied as a jump after power-on.
                last_disc_pos = disc.position();
                // The debouncer is not polled while powered off, so its
                // timestamps go stale. Without this reset, a press left
                // half-recognised at power-off (e.g. inside the double-press
                // window) is emitted as a spurious Short or Double on the
                // very next poll after power-on.
                debouncer = ButtonDebouncer::new();
            } else {
                power_down(&mut radio, &mut tuner, &mut display, &mut storage, now);
                led.set(adapters::led::OFF);
            }
            power_on = sw_on;
        }

        if !power_on {
            continue;
        }

        let raw_pressed = action_btn.is_pressed();
        match debouncer.poll(raw_pressed, now) {
            ButtonEvent::Short => {
                radio.cycle_mode();
                tuner.set_frequency(radio.settings.freq);
                redraw(&radio, &mut display);
            }
            ButtonEvent::Double => {
                match radio.mode {
                    Mode::Tune => {
                        radio.add_current_to_favs(now);
                    }
                    Mode::Fav => {
                        radio.delete_selected_fav(now);
                        // radio-core only updates settings.freq; push it to
                        // the tuner here. When the list emptied out freq is
                        // unchanged, so this is a harmless re-tune.
                        tuner.set_frequency(radio.settings.freq);
                    }
                }
                redraw(&radio, &mut display);
            }
            // Long press is deliberately unassigned now that the pot switch
            // owns power on/off.
            _ => {}
        }

        // Disc: tunes in TUNE mode, moves the favourite selection in FAV
        // mode. Frozen while the action button is held so a double-press
        // doesn't nudge the frequency.
        // Also frozen in FAV mode with 0 or 1 favourites: there is nothing
        // to select between, so any spurious movement would otherwise still
        // trigger a real (audible) retune + redraw for no visible effect.
        let disc_pos = disc.position();
        let encoder_frozen =
            raw_pressed || (radio.mode == Mode::Fav && radio.settings.favs.len() <= 1);
        if encoder_frozen {
            last_disc_pos = disc_pos;
        } else if disc_pos != last_disc_pos {
            let delta = disc_pos - last_disc_pos;
            last_disc_pos = disc_pos;
            radio.apply_encoder_delta(delta, now);
            tuner.set_frequency(radio.settings.freq);
            redraw(&radio, &mut display);
        }

        led.set(match radio.mode {
            Mode::Tune => adapters::led::TUNE,
            Mode::Fav => adapters::led::FAV,
        });

        save_if_due(&mut radio, &mut storage, now, SAVE_DELAY_MS);
    }
}
