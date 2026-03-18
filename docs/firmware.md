# Firmware Guide

## Supported Boards

| Board | Display | Features |
|-------|---------|----------|
| ESP32 (default) | SSD1306 128x64 OLED | Avatar, LED, buzzer, servos, BLE provisioning |
| ESP32-4848S040C | ST7701S 480x480 LCD | All above + touch UI, 4x scaled graphics |

## Building

### PlatformIO CLI

```bash
cd firmware

# OLED board (default)
pio run -e esp32

# LCD board
pio run -e esp32-4848s040c
```

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
