# 📻 Minimal Desktop FM Radio

A breadboard FM radio built with an Arduino Pro Mini, RDA5807M tuner, SSD1306 OLED, and a rotary encoder.

## 🎯 Why

- Streaming radio over Bluetooth from a phone is easy, but unlocking the phone every time got tedious — I wanted a minimal physical interface dedicated to radio.
- I missed the saturated, radio-ish sound character instead of the clean audio of smartphone apps.


## ✅ Goals

- Tune the Japanese FM band (76–108 MHz).
- Persist frequency, volume, and favorites across power-off.
- Drive everything from a single knob (rotary encoder + push switch).

## 📍 Status

**Working:**

- Tuning (100 kHz step, wraps at 76.00 / 108.00 MHz)
- Volume (0–15, 0 fully mutes)
- OLED display (frequency / volume / RSSI signal bar)
- Mode cycle: short press toggles `TUNE → VOL → FAV → TUNE`
- Power on/off: long press (1.5 s) toggles
- Favorites: 16 slots, double-press to add or delete
- EEPROM persistence (auto-saves 2 s after the last change)

**Open issues:**

- Sketch upload occasionally fails (suspected USB-serial adapter clock drift).
- Still a breadboard prototype — no enclosure yet.

## 📚 Docs

- [docs/parts.md](docs/parts.md) — parts list
- [docs/wiring.md](docs/wiring.md) — wiring diagram and pin assignments


## 🔨 Build

```bash
arduino-cli compile --fqbn arduino:avr:pro:cpu=8MHzatmega328 .
arduino-cli upload -p /dev/cu.usbserial-XX --fqbn arduino:avr:pro:cpu=8MHzatmega328 .
```

Required libraries (via Arduino Library Manager):

- PU2CLR RDA5807
- U8g2
- RotaryEncoder (by Matthias Hertel)
- EEPROM (Arduino built-in)

## 📄 License

Unspecified (personal hobby log).
