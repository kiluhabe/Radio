#include <Wire.h>
#include <RDA5807.h>
#include <U8g2lib.h>
#include <RotaryEncoder.h>
#include <EEPROM.h>

// Pin assignment per docs/soldering-rp.md / hardware/diylc/README.md.
// Encoder (EC12E2430803) has no integrated push switch; the push button is a
// separate tactile switch (8MS8P1B05VS2QES-1) wired to GP6.
constexpr uint8_t PIN_ENC_A  = 2;
constexpr uint8_t PIN_ENC_B  = 3;
constexpr uint8_t PIN_SW     = 6;

constexpr uint16_t FREQ_MIN  = 7600;
constexpr uint16_t FREQ_MAX  = 10800;
constexpr uint16_t FREQ_STEP = 10;
constexpr uint16_t FREQ_INIT = 8130;

// Volume is now a physical dual-gang pot on LOUT/ROUT ahead of the jack
// (see docs/parts-rp.md); the RDA5807's own digital attenuator is pinned
// high once at init so the pot does all the attenuating, not fought against.
constexpr uint8_t FIXED_VOL = 15;

constexpr uint8_t MAX_FAVS = 16;

constexpr uint32_t BTN_DEBOUNCE_MS  = 50;
constexpr uint32_t LONG_PRESS_MS    = 1500;
constexpr uint32_t DOUBLE_WINDOW_MS = 400;
constexpr uint32_t SAVE_DELAY_MS    = 2000;

constexpr uint16_t EEPROM_MAGIC = 0xAB16; // bumped: Settings layout changed (vol field removed again)
constexpr int      EEPROM_ADDR  = 0;
constexpr size_t   EEPROM_SIZE  = 256; // flash-emulated; must cover sizeof(Settings)

struct Settings {
  uint16_t magic;
  uint16_t freq;
  uint8_t  favCount;
  uint16_t favs[MAX_FAVS];
};

enum Mode : uint8_t { MODE_TUNE = 0, MODE_FAV = 1 };
enum BtnEvent : uint8_t { BTN_NONE = 0, BTN_SHORT = 1, BTN_LONG = 2, BTN_DOUBLE = 3 };

RDA5807 rx;
U8G2_SSD1306_128X64_NONAME_1_HW_I2C u8g2(U8G2_R0, U8X8_PIN_NONE);
RotaryEncoder encoder(PIN_ENC_A, PIN_ENC_B, RotaryEncoder::LatchMode::FOUR3);

uint16_t currentFreq = FREQ_INIT;
Mode     mode        = MODE_TUNE;
long     lastEncPos  = 0;

uint16_t favs[MAX_FAVS];
uint8_t  favCount    = 0;
uint8_t  favIndex    = 0;

uint32_t lastDebounce       = 0;
bool     lastBtnReading     = HIGH;
bool     stableBtnState     = HIGH;
uint32_t pressStartedAt     = 0;
bool     longArmed          = false;
uint32_t lastShortReleaseAt = 0;
bool     awaitingDouble     = false;
bool     doubleConsumed     = false;
bool     powerOn            = true;

bool     dirty          = false;
uint32_t lastChangeTime = 0;

uint8_t  cachedRssi     = 0;

static void drawScreen();

static void encoderISR() {
  encoder.tick();
}

static long readEncoderPosition() {
  noInterrupts();
  long pos = encoder.getPosition();
  interrupts();
  return pos;
}

static BtnEvent pollBtn() {
  bool reading = digitalRead(PIN_SW);
  if (reading != lastBtnReading) {
    lastDebounce = millis();
    lastBtnReading = reading;
  }

  bool stateChanged = false;
  if (millis() - lastDebounce > BTN_DEBOUNCE_MS && reading != stableBtnState) {
    stableBtnState = reading;
    stateChanged = true;
  }

  if (stateChanged && stableBtnState == LOW) {
    if (awaitingDouble) {
      awaitingDouble = false;
      doubleConsumed = true;
      longArmed = false;
      return BTN_DOUBLE;
    }
    pressStartedAt = millis();
    longArmed = true;
  }

  if (longArmed && stableBtnState == LOW && (millis() - pressStartedAt) >= LONG_PRESS_MS) {
    longArmed = false;
    return BTN_LONG;
  }

  if (stateChanged && stableBtnState == HIGH) {
    if (doubleConsumed) {
      doubleConsumed = false;
      return BTN_NONE;
    }
    if (longArmed) {
      longArmed = false;
      awaitingDouble = true;
      lastShortReleaseAt = millis();
    }
  }

  if (awaitingDouble && (millis() - lastShortReleaseAt) > DOUBLE_WINDOW_MS) {
    awaitingDouble = false;
    return BTN_SHORT;
  }

  return BTN_NONE;
}

static void loadSettings() {
  Settings s;
  EEPROM.get(EEPROM_ADDR, s);
  if (s.magic != EEPROM_MAGIC) return;
  if (s.freq >= FREQ_MIN && s.freq <= FREQ_MAX) currentFreq = s.freq;
  if (s.favCount > MAX_FAVS) return;
  for (uint8_t i = 0; i < s.favCount; i++) {
    if (s.favs[i] < FREQ_MIN || s.favs[i] > FREQ_MAX) return;
  }
  for (uint8_t i = 0; i < s.favCount; i++) favs[i] = s.favs[i];
  favCount = s.favCount;
}

static void saveSettings() {
  Settings s = {};
  s.magic    = EEPROM_MAGIC;
  s.freq     = currentFreq;
  s.favCount = favCount;
  for (uint8_t i = 0; i < favCount; i++) s.favs[i] = favs[i];
  EEPROM.put(EEPROM_ADDR, s);
  EEPROM.commit();
  Serial.println(F("saved"));
}

static void markDirty() {
  dirty = true;
  lastChangeTime = millis();
}

static int8_t findFav(uint16_t freq) {
  for (uint8_t i = 0; i < favCount; i++) {
    if (favs[i] == freq) return (int8_t)i;
  }
  return -1;
}

static void addCurrentToFavs() {
  if (findFav(currentFreq) >= 0) {
    Serial.println(F("fav exists"));
    return;
  }
  if (favCount >= MAX_FAVS) {
    Serial.println(F("fav full"));
    return;
  }
  favs[favCount++] = currentFreq;
  markDirty();
  Serial.print(F("fav added: "));
  Serial.println(currentFreq);
}

static void deleteSelectedFav() {
  if (favCount == 0) return;
  for (uint8_t i = favIndex; i + 1 < favCount; i++) {
    favs[i] = favs[i + 1];
  }
  favCount--;
  if (favCount == 0) {
    favIndex = 0;
    mode = MODE_TUNE;
  } else {
    if (favIndex >= favCount) favIndex = favCount - 1;
    currentFreq = favs[favIndex];
    rx.setFrequency(currentFreq);
  }
  markDirty();
  Serial.println(F("fav deleted"));
}

static void enterFavMode() {
  if (favCount == 0) {
    favIndex = 0;
    return;
  }
  int8_t idx = findFav(currentFreq);
  if (idx >= 0) {
    favIndex = (uint8_t)idx;
    return;
  }
  favIndex = 0;
  currentFreq = favs[0];
  rx.setFrequency(currentFreq);
}

static void cycleMode() {
  if (mode == MODE_TUNE) {
    mode = MODE_FAV;
    enterFavMode();
  } else {
    mode = MODE_TUNE;
  }
}

static const __FlashStringHelper* modeName() {
  if (mode == MODE_TUNE) return F("TUNE");
  return F("FAV");
}

static void radioPowerOff() {
  if (dirty) {
    saveSettings();
    dirty = false;
  }
  rx.setMute(true);
  rx.powerDown();
  u8g2.setPowerSave(1);
  powerOn = false;
  Serial.println(F("power off"));
}

static void radioPowerOn() {
  u8g2.setPowerSave(0);
  rx.powerUp();
  rx.setBand(2);
  rx.setSpace(0);
  rx.setMute(true);
  rx.setFrequency(currentFreq);
  delay(50);
  rx.setVolume(FIXED_VOL);
  rx.setMute(false);
  drawScreen();
  lastEncPos = readEncoderPosition();
  powerOn = true;
  Serial.println(F("power on"));
}

static void drawScreen() {
  cachedRssi = rx.getRssi();

  char freqBuf[16];
  snprintf(freqBuf, sizeof(freqBuf), "%u.%02u MHz", currentFreq / 100, currentFreq % 100);
  char favBuf[16];
  if (mode == MODE_FAV) {
    if (favCount == 0) {
      strcpy(favBuf, "FAV (empty)");
    } else {
      snprintf(favBuf, sizeof(favBuf), "FAV %u/%u", favIndex + 1, favCount);
    }
  }

  int fillW = cachedRssi;
  if (fillW > 126) fillW = 126;

  u8g2.firstPage();
  do {
    u8g2.setFont(u8g2_font_6x10_tr);
    if (mode == MODE_TUNE) {
      u8g2.drawStr(0, 10, "TUNE");
      u8g2.setFont(u8g2_font_logisoso24_tr);
      u8g2.drawStr(0, 50, freqBuf);
    } else {
      u8g2.drawStr(0, 10, favBuf);
      if (favCount > 0) {
        u8g2.setFont(u8g2_font_logisoso24_tr);
        u8g2.drawStr(0, 50, freqBuf);
      }
    }

    u8g2.drawFrame(0, 58, 128, 6);
    if (fillW > 0) u8g2.drawBox(1, 59, fillW, 4);
  } while (u8g2.nextPage());
}

void setup() {
  Serial.begin(9600);
  Wire.begin();

  pinMode(PIN_SW, INPUT_PULLUP);
  attachInterrupt(digitalPinToInterrupt(PIN_ENC_A), encoderISR, CHANGE);
  attachInterrupt(digitalPinToInterrupt(PIN_ENC_B), encoderISR, CHANGE);

  EEPROM.begin(EEPROM_SIZE);
  loadSettings();

  rx.setup();
  rx.setBand(2);        // 76-108 MHz
  rx.setSpace(0);       // 100 kHz step
  rx.setMute(true);
  rx.setFrequency(currentFreq);
  rx.setVolume(FIXED_VOL);
  rx.setMute(false);

  u8g2.begin();
  u8g2.setContrast(0x10);
  drawScreen();

  lastDebounce = millis();
  Serial.println(F("radio ready"));
}

void loop() {
  BtnEvent ev = pollBtn();
  if (ev == BTN_LONG) {
    if (powerOn) radioPowerOff();
    else radioPowerOn();
  } else if (powerOn) {
    if (ev == BTN_SHORT) {
      cycleMode();
      drawScreen();
      Serial.print(F("mode="));
      Serial.println(modeName());
    } else if (ev == BTN_DOUBLE) {
      if (mode == MODE_TUNE) {
        addCurrentToFavs();
        drawScreen();
      } else if (mode == MODE_FAV) {
        deleteSelectedFav();
        drawScreen();
      }
    }
  }

  if (!powerOn) return;

  long pos = readEncoderPosition();
  if (stableBtnState == LOW) {
    lastEncPos = pos;
  } else if (pos != lastEncPos) {
    long delta = pos - lastEncPos;
    lastEncPos = pos;

    if (mode == MODE_TUNE) {
      long range = (long)FREQ_MAX - FREQ_MIN + FREQ_STEP;
      long next = (long)currentFreq + delta * FREQ_STEP;
      while (next < (long)FREQ_MIN) next += range;
      while (next > (long)FREQ_MAX) next -= range;
      currentFreq = (uint16_t)next;
      rx.setFrequency(currentFreq);
      drawScreen();
      markDirty();
      Serial.print(F("freq="));
      Serial.println(currentFreq);
    } else {
      if (favCount > 0) {
        long fc = (long)favCount;
        long newIdx = ((long)favIndex + delta) % fc;
        if (newIdx < 0) newIdx += fc;
        favIndex = (uint8_t)newIdx;
        currentFreq = favs[favIndex];
        rx.setFrequency(currentFreq);
        drawScreen();
        markDirty();
        Serial.print(F("fav="));
        Serial.println(favIndex);
      }
    }
  }

  if (dirty && millis() - lastChangeTime >= SAVE_DELAY_MS) {
    saveSettings();
    dirty = false;
  }
}
