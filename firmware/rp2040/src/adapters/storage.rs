//! Flash-backed `Storage` adapter (MVP).
//!
//! Same approach as the Arduino version's `EEPROM.put`/`EEPROM.get`: a flat,
//! magic-number-validated struct written raw to a dedicated flash region.
//! Not wear-leveled -- that's explicitly out of scope for this MVP.
//!
//! RP2040 flash can only be erased in 4KB sectors, so `memory.x` reserves the
//! last 4KB of flash as `SETTINGS_FLASH` and `FLASH_OFFSET` below must match
//! its origin (both are expressed as an offset from the flash address space
//! base, 0x1000_0000).
//!
//! ## Deviations from the task brief's reference code
//!
//! The brief's sketch was written against `rp2040-flash` "0.5" and flagged
//! that the real API might differ. Checked against the actual crate source
//! for 0.6.0 (the version that resolves today) on docs.rs / crates.io:
//!
//! - **No `flash_read` function exists.** RP2040 flash is memory-mapped
//!   (XIP) during normal execution -- reading it is just a pointer
//!   dereference at `0x1000_0000 + offset`; the crate only provides helpers
//!   for the operations that require briefly leaving XIP mode (erase,
//!   program, unique-id/JEDEC-id reads). `load()` below reads directly
//!   through that mapped address instead of calling a crate function.
//! - **`flash_range_erase_and_program` is used instead of separate
//!   `flash_range_erase` + `flash_range_program` calls.** The brief called
//!   both individually; the crate's own `flash_example.rs` uses the combined
//!   function for exactly this erase-then-rewrite-a-sector pattern, and it
//!   does the erase+program as a single trip out of XIP mode (one
//!   `flash_exit_xip`/`flash_enter_cmd_xip` pair) rather than two.
//! - `flash_range_erase_and_program(addr, data, use_boot2)` keeps the same
//!   argument order/types the brief assumed (`addr: u32`, `data: &[u8]`,
//!   `use_boot2: bool`), so no change was needed there. `use_boot2: true` is
//!   passed, matching the crate's own example, so the XIP engine is
//!   re-initialized via a copy of the 2nd-stage bootloader after flashing.
//!
//! ## Safety
//!
//! `rp2040-flash`'s erase/program functions require that nothing else
//! access flash while they run (interrupts disabled, no second core running
//! flash-resident code, no DMA touching flash). This firmware is
//! single-core (core 1 is never started) and has no DMA use anywhere, so
//! wrapping the call in `cortex_m::interrupt::free` satisfies all of the
//! crate's documented preconditions.

use radio_core::{Settings, MAX_FAVS};

const MAGIC: u32 = 0xAB20_4001;

/// Offset of `SETTINGS_FLASH` from the flash address space base
/// (0x1000_0000). Must match `memory.x`'s `SETTINGS_FLASH` origin.
const FLASH_OFFSET: u32 = 2048 * 1024 - 4096;
const SECTOR_SIZE: usize = 4096;

/// Base address RP2040 flash is memory-mapped to during normal (XIP)
/// execution.
const XIP_BASE: usize = 0x1000_0000;

#[repr(C)]
struct StoredSettings {
    magic: u32,
    freq: u16,
    fav_count: u16,
    favs: [u16; MAX_FAVS],
}

pub struct FlashStorage;

impl FlashStorage {
    pub fn new() -> Self {
        FlashStorage
    }
}

impl radio_core::Storage for FlashStorage {
    fn load(&mut self) -> Option<Settings> {
        let mut buf = [0u8; core::mem::size_of::<StoredSettings>()];
        let src = (XIP_BASE + FLASH_OFFSET as usize) as *const u8;
        for (i, b) in buf.iter_mut().enumerate() {
            // SAFETY: flash is memory-mapped for reads during normal
            // execution. `src..src+buf.len()` stays within the
            // SETTINGS_FLASH sector reserved in memory.x.
            *b = unsafe { core::ptr::read_volatile(src.add(i)) };
        }
        // SAFETY: `buf` holds exactly size_of::<StoredSettings>() bytes
        // freshly read from flash above. `read_unaligned` is used because
        // `buf` (a `[u8; N]`) only guarantees alignment 1, not the
        // alignment 4 that `StoredSettings` (starting with a `u32`)
        // requires for a plain `read`.
        let stored: StoredSettings =
            unsafe { core::ptr::read_unaligned(buf.as_ptr() as *const StoredSettings) };

        if stored.magic != MAGIC {
            return None;
        }
        if stored.fav_count as usize > MAX_FAVS {
            return None;
        }

        let mut settings = Settings {
            freq: stored.freq,
            favs: heapless::Vec::new(),
        };
        for i in 0..stored.fav_count as usize {
            if settings.favs.push(stored.favs[i]).is_err() {
                return None;
            }
        }
        Some(settings)
    }

    fn save(&mut self, settings: &Settings) {
        let mut stored = StoredSettings {
            magic: MAGIC,
            freq: settings.freq,
            fav_count: settings.favs.len() as u16,
            favs: [0; MAX_FAVS],
        };
        for (i, &f) in settings.favs.iter().enumerate() {
            stored.favs[i] = f;
        }

        // SAFETY: StoredSettings is #[repr(C)] with no padding-sensitive
        // invariants (all fields are plain integers), so reinterpreting it
        // as bytes is sound.
        let bytes: [u8; core::mem::size_of::<StoredSettings>()] =
            unsafe { core::mem::transmute_copy(&stored) };
        let mut sector_buf = [0u8; SECTOR_SIZE];
        sector_buf[..bytes.len()].copy_from_slice(&bytes);

        // SAFETY: interrupts are disabled for the whole erase+program call,
        // and this firmware never starts core 1 or uses DMA against flash,
        // satisfying rp2040-flash's documented preconditions.
        cortex_m::interrupt::free(|_| unsafe {
            rp2040_flash::flash::flash_range_erase_and_program(FLASH_OFFSET, &sector_buf, true);
        });
    }
}
