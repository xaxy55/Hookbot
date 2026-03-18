#pragma once

#include "avatar.h"

// Non-blocking tone queue for passive buzzer
namespace Sound {
    void init();
    void playStateSound(AvatarState state);
    void updateWaitingEscalation(uint32_t stateTimeMs);
    void update(uint32_t deltaMs);
}
