# 🧰 Parts List

## 🔩 Core parts

| Part | Model / spec | Qty | Notes |
|---|---|---|---|
| Microcontroller board | Arduino Pro Mini 3.3V / ATmega328P / 8 MHz | 1 | 32KB flash, 2KB RAM, 1KB EEPROM |
| USB-serial adapter | CH340 / CP2102 etc. | 1 | For uploads and serial monitor |
| FM tuner module | RDA5807M family | 1 | I2C, 3.3V compatible |
| OLED display | SSD1306 128×64 I2C | 1 | Address `0x3C` |
| Rotary encoder | Yushakobo A080028-01-1 (EC12 compatible, 24 detents, push switch, 20 mm shaft) | 1 | A/B/C + D/E (switch), 5 terminals |
| DIP breakout board | Akizuki RECNV-3 (#107241) | 1 | ⚠️ **Not EC12 compatible** (3+5 pin layout). Wired directly with leads in this build |
| Stereo mini jack | MJ-355W | 1 | Audio output |
| Small speaker | Any | 1 | Connected via the stereo mini jack |
| Breadboard | SAD-101 | 1 | Prototype only |

## ⚡ Passive components

| Part | Spec | Qty | Use |
|---|---|---|---|
| Electrolytic capacitor | 200 µF / ≥6.3V | 2 | One across VCC–GND each at the Pro Mini and OLED |
| Pin headers | 2.54 mm | as needed | For Pro Mini and module connections |
| Jumper wires | Solid / stranded | as needed | Also used to wire the encoder directly |

## 🔋 Power margin notes

The 3.3V LDO on the USB-serial adapter is right at the edge of its capacity. With **Pro Mini + RDA5807M + OLED + rotary encoder** all running together:

- Brown-out resets during flash writes
- SSD1306 internal charge pump resets periodically, making the screen flicker
- ❄Cold-start sometimes fails

**Adding a 200 µF-class electrolytic at the VCC pin of both the Pro Mini and the OLED** fixed it.
