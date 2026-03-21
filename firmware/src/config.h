#pragma once

// ─── Firmware Version ─────────────────────────────────────────────
#define FIRMWARE_VERSION "0.7.0"

// ─── WiFi Configuration ───────────────────────────────────────────
// WiFi is provisioned via BLE at first boot. The device advertises as
// "Hookbot-XXYY" and accepts credentials over Bluetooth.
//
// Optional: create firmware/src/secrets.h with compile-time credentials:
//   #define WIFI_SSID "YourSSID"
//   #define WIFI_PASS "YourPassword"
//
#if __has_include("secrets.h")
#include "secrets.h"
#endif

// Max networks configurable at runtime via NVS (on top of compile-time ones)
#define MAX_WIFI_NETWORKS 6

// mDNS hostname (hookbot.local) - overridden by NVS if set
#define MDNS_HOSTNAME "hookbot"

// Management server URL (empty = disabled, set for hosted/cloud mode)
// For production devices: "https://bot.mr-ai.no"
// For self-hosted: "" (disabled) — set via BLE provisioning or POST /config
#ifndef DEFAULT_MGMT_SERVER
#define DEFAULT_MGMT_SERVER ""
#endif

// ─── Board-specific display configuration ─────────────────────────

#ifdef BOARD_ESP32_4848S040C
  // ESP32-4848S040C: 4.0" 480x480 IPS LCD (ST7701S + GT911 touch)
  // Virtual canvas resolution (scaled 4x to 480x480)
  #define SCREEN_WIDTH   120
  #define SCREEN_HEIGHT  120
  #define LCD_PHYS_WIDTH  480
  #define LCD_PHYS_HEIGHT 480
  #define LCD_SCALE       4
#else
  // Default: SSD1306 OLED 128x64 I2C
  #define SCREEN_WIDTH   128
  #define SCREEN_HEIGHT   64
  #define OLED_SDA        21
  #define OLED_SCL        22
  #define OLED_ADDR     0x3C
#endif

// ─── WS2812B LED ─────────────────────────────────────────────────
#ifndef NO_LED
  #define LED_PIN    16
  #define NUM_LEDS    1
#endif

// ─── Passive Buzzer ──────────────────────────────────────────────
#ifndef NO_SOUND
  #define BUZZER_PIN 25
#endif

// ─── I2S Audio (Voice Control & TTS) ────────────────────────────
// Define NO_AUDIO to disable I2S audio support (mic + speaker)
// Requires INMP441 microphone and MAX98357A DAC amplifier
// Pin assignments in audio.h

// ─── Animation Settings ──────────────────────────────────────────
#define TARGET_FPS        30
#define FRAME_TIME_MS     (1000 / TARGET_FPS)
#define AUTO_RETURN_MS    3000  // Success/taskcheck auto-return to idle
