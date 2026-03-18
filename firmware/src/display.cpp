#ifndef BOARD_ESP32_4848S040C

#include "display.h"
#include <Wire.h>

namespace Display {

static Adafruit_SSD1306* oled = nullptr;

void init() {
    Wire.begin(OLED_SDA, OLED_SCL);

    oled = new Adafruit_SSD1306(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, -1);

    if (!oled->begin(SSD1306_SWITCHCAPVCC, OLED_ADDR)) {
        Serial.println("[Display] SSD1306 init FAILED");
        return;
    }

    oled->clearDisplay();
    oled->display();

    Serial.println("[Display] SSD1306 128x64 initialized");
}

void clear() {
    oled->clearDisplay();
}

void flush() {
    oled->display();
}

DisplayCanvas* getCanvas() {
    return oled;
}

int16_t width()   { return SCREEN_WIDTH; }
int16_t height()  { return SCREEN_HEIGHT; }
int16_t centerX() { return SCREEN_WIDTH / 2; }
int16_t centerY() { return SCREEN_HEIGHT / 2; }

} // namespace Display

#endif // !BOARD_ESP32_4848S040C
