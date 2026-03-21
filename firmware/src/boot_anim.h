#pragma once
#include <Arduino.h>

namespace BootAnim {
    // Play the configured boot animation. Blocks until complete.
    // type: 0=none, 1=classic, 2=matrix, 3=glitch
    void play(int type);
}
