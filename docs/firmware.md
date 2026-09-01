# Firmware Guide

## Supported Boards

| Board | PlatformIO env | Display | Features |
|-------|----------------|---------|----------|
| ESP32 (default) | `esp32` | SSD1306 128x64 OLED | Avatar, LED, buzzer, servos, BLE provisioning |
| ESP32-4848S040C | `esp32-4848s040c` | ST7701S 480x480 LCD | All above + touch UI, 4x scaled graphics |
| Seeed XIAO ESP32-C6 | `xiao-c6-gc9a01` | GC9A01 240x240 round LCD | Avatar, servos, BLE provisioning, 2x scaled graphics |

All colour LCD boards render to the same 120x120 virtual canvas and scale it up
on flush, so avatar and screensaver code is shared across them.

### Seeed XIAO ESP32-C6 + GC9A01 wiring

| Display pin | XIAO pin | ESP32-C6 GPIO |
|-------------|----------|---------------|
| BL  | D0  | 0  |
| CS  | D1  | 1  |
| RST | D2  | 2  |
| DC  | D3  | 21 |
| SCK | D8  | 19 |
| MOSI (SDA) | D10 | 18 |
| VCC | 3V3 | — |
| GND | GND | — |

Notes:

- The panel is **round**: the 120x120 virtual canvas is drawn as a 240x240
  square, so the canvas corners fall behind the bezel. Edge-anchored UI is
  positioned against `SAFE_LEFT` / `SAFE_TOP` / `SAFE_RIGHT` / `SAFE_BOTTOM`
  (`firmware/src/config.h`) rather than the raw canvas edge — that rectangle is
  the largest square inscribed in the circle (an 18px inset per side). On the
  rectangular boards the inset is 0, so those macros are the canvas bounds and
  their layout is unchanged. Use them for any new HUD element.
- There is **no touch controller**, so the touch UI overlay is compiled out.
  LED, buzzer, and I2S audio are disabled too (`NO_LED`, `NO_SOUND`, `NO_AUDIO`).
- Flash is tight: the image is ~1.93MB against a 1.98MB OTA slot. The stock
  `min_spiffs.csv` app slot (0x1E0000) is too small, so this env uses
  `partitions_c6_ota.csv`, which drops the unused filesystem partition to give
  both OTA slots 0x1F0000. It also builds at `CORE_DEBUG_LEVEL=0`. If you add
  much more code here, expect `Image length ... doesn't fit in partition length`
  at boot — the build itself will still succeed.
- ESP32-C6 requires Arduino-ESP32 3.x, which the official `espressif32`
  PlatformIO platform does not ship. The `xiao-c6-gc9a01` env pins the
  [pioarduino](https://github.com/pioarduino/platform-espressif32) fork
  instead. The first build downloads a separate RISC-V toolchain.

## Building

### PlatformIO CLI

```bash
cd firmware

# OLED board (default)
pio run -e esp32

# LCD board
pio run -e esp32-4848s040c

# XIAO ESP32-C6 round LCD board
pio run -e xiao-c6-gc9a01
```

### Why the C6 board needs its own PlatformIO store

Use the make targets rather than a bare `pio run`:

```bash
make firmware            # esp32 + esp32-4848s040c
make firmware-c6         # XIAO ESP32-C6 round LCD
make firmware-c6-upload  # flash it over USB
```

`xiao-c6-gc9a01` is the only env on the pioarduino platform. That platform is
*also* named `espressif32`, and both it and the official platform install a
package called `framework-arduinoespressif32` — at incompatible versions
(2.0.17 vs 3.3.11) into the same directory. Sharing one PlatformIO core dir
means whichever env built last wins and the other fails with
`TypeError: expected str, bytes or os.PathLike object, not NoneType`.
`make firmware-c6` points `PLATFORMIO_CORE_DIR` at a separate store
(`~/.platformio-pioarduino`, override with `PIO_C6_CORE_DIR`), so the two
coexist. The first build there re-downloads the toolchain.

For the same reason the `esp32` env pins `espressif32@^6.0.0`. Left unpinned it
resolves to the pioarduino platform (version 55.x beats 6.x) and then overflows
IRAM on the classic ESP32.

### Upload

```bash
# USB
pio run -e esp32 --target upload

# OTA (after first flash)
# Use the web dashboard: OTA page → select firmware → deploy
```

## WiFi Setup

### BLE Provisioning (recommended)

1. Flash the firmware via USB
2. Device boots and advertises as `DeskBot-XXYY` over Bluetooth
3. Connect with any BLE app (e.g. nRF Connect, LightBlue)
4. Find service `4fafc201-1fb5-459e-8fcc-c5c9c331914b`
5. Write to characteristic `beb5483e-36e1-4688-b7f5-ea07361b26a8`:
   ```
   YourSSID\nYourPassword
   ```
6. Device saves credentials to flash and reboots
7. On successful WiFi connection, BLE stops automatically

The device stores up to 6 WiFi networks. If WiFi drops, BLE restarts automatically.

### Compile-Time Credentials (optional)

For development, you can create `firmware/src/secrets.h`:

```cpp
#define WIFI_SSID "YourSSID"
#define WIFI_PASS "YourPassword"
```

This file is auto-detected via `__has_include` and never committed to git.

## Pin Configuration

### OLED Board (ESP32)

| Pin | Function |
|-----|----------|
| 21 | SDA (I2C) |
| 22 | SCL (I2C) |
| 16 | WS2812B LED |
| 25 | Passive buzzer |

### LCD Board (ESP32-4848S040C)

Display and touch are driven via SPI/I2C by LovyanGFX. See `display_lcd.cpp` for pin assignments.

## Configuration via NVS

The device stores runtime configuration in non-volatile storage:

- WiFi networks (up to 6)
- LED brightness
- Sound enabled/volume
- mDNS hostname
- Avatar preset and accessories
- Management server URL

Configuration can be changed via the web dashboard or the device's HTTP API.

## Avatar States

| State | Trigger | Animation |
|-------|---------|-----------|
| IDLE | Default / Stop | Blinking, breathing |
| THINKING | Tool use / prompt | Eyes moving, processing |
| SUCCESS | Task complete / build pass | Celebration |
| ERROR | Build fail / error | Shaking, red |
| WAITING | User input needed | Escalating beeps |
| TASKCHECK | Task list update | Checklist overlay |

## Conditional Compilation

| Flag | Effect |
|------|--------|
| `NO_DISPLAY` | Disable OLED/LCD (headless mode) |
| `NO_LED` | Disable WS2812B LED |
| `NO_SOUND` | Disable buzzer |
| `BOARD_ESP32_4848S040C` | LCD board variant |
