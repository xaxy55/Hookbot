#pragma once

#include "config.h"

#ifdef BOARD_ESP32_4848S040C

#include <Arduino.h>

namespace TouchUI {
    void init();
    void update(uint32_t deltaMs, int16_t touchX, int16_t touchY, bool touching);
    void draw();         // Draw overlay on virtual canvas (120x120)
    bool isOverlayActive();  // True if overlay is consuming touches
}

#endif
