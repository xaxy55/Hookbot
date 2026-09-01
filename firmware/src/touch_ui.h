#pragma once

#include "config.h"

#ifdef DISPLAY_TOUCH

#include <Arduino.h>

namespace TouchUI {
    void init();
    void update(uint32_t deltaMs, int16_t touchX, int16_t touchY, bool touching);
    void draw();         // Draw overlay on virtual canvas (120x120)
    bool isOverlayActive();  // True if overlay is consuming touches
}

#endif
