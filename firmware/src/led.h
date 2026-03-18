#pragma once

#ifndef NO_LED

#include "avatar.h"

// WS2812B LED subsystem
namespace Led {
    void init();
    void setState(AvatarState state);
    void update(uint32_t deltaMs);
}

#endif // !NO_LED
