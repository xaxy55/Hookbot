#pragma once

#include <Arduino.h>

// Screen saver subsystem - random animations to prevent OLED burn-in
namespace Screensaver {
    void init();
    // Call each frame with time since screensaver activated
    void update(uint32_t deltaMs);
    void draw();
    // Pick a new random animation
    void randomize();
}
