#pragma once

#include "config.h"

#ifdef BOARD_ESP32_4848S040C
  #include <LovyanGFX.hpp>
  typedef lgfx::LGFX_Sprite DisplayCanvas;
  #define COLOR_WHITE 0xFFFFU
  #define COLOR_BLACK 0x0000U
#else
  #include <Adafruit_SSD1306.h>
  typedef Adafruit_SSD1306 DisplayCanvas;
  #define COLOR_WHITE SSD1306_WHITE
  #define COLOR_BLACK SSD1306_BLACK
#endif

// Display subsystem - abstracts SSD1306 OLED and ESP32-4848S040C LCD
namespace Display {
    void init();
    void clear();
    void flush();

    DisplayCanvas* getCanvas();
    int16_t width();
    int16_t height();
    int16_t centerX();
    int16_t centerY();

#ifdef BOARD_ESP32_4848S040C
    bool getTouchPoint(int16_t& x, int16_t& y);
    void touchTest();
    void setBrightness(uint8_t level);
#endif
}
