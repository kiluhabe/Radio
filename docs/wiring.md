# 🔌 Wiring

## 📌 Pin assignments

### Arduino Pro Mini

| Pin | Connects to | Use |
|---|---|---|
| `VCC` | 3.3V rail | Power (3.3V from the USB-serial adapter) |
| `GND` | GND rail | Ground |
| `RAW` | (unused) | LDO input. Not needed when powered via USB-serial |
| `RST` | (unused / optional reset button) | Manual reset for uploads |
| `D0` (RXI) | USB-serial TX | Upload / serial RX |
| `D1` (TXO) | USB-serial RX | Upload / serial TX |
| `D2` (INT0) | Encoder A | Internal pull-up |
| `D3` (INT1) | Encoder B | Internal pull-up |
| `D4` | Encoder push switch | Internal pull-up |
| `A4` (SDA) | I2C SDA bus | Shared by RDA5807M / OLED |
| `A5` (SCL) | I2C SCL bus | Shared by RDA5807M / OLED |

### Devices on the I2C bus

| Address | Device |
|---|---|
| `0x10` | RDA5807M (random access mode) |
| `0x11` | RDA5807M (secondary) |
| `0x3C` | SSD1306 OLED |
| `0x60` | RDA5807M (sequential access, TEA5767-compatible) |


## 🔋 Power stabilization

The USB-serial adapter's 3.3V LDO has little headroom, so **place a large electrolytic capacitor at the VCC pin of each IC**:

```
              3.3V rail ●─────●─── to each IC's VCC
                        │     │
                     (+)│     │(+)
                  ┌───────┐ ┌───────┐
                  │ 200µF │ │ 200µF │
                  │       │ │       │
                  └───────┘ └───────┘
                     (-)│     │(-)
                        │     │
              GND  rail ●─────●

              ↑near Pro Mini   ↑near OLED
```
