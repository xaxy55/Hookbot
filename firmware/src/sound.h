#pragma once

#include "avatar.h"

// Custom melody per state: array of {freq, duration} pairs
#define MAX_MELODY_NOTES 8
struct Melody {
    uint16_t freqs[MAX_MELODY_NOTES];
    uint16_t durations[MAX_MELODY_NOTES];
    uint8_t count;
};

// Non-blocking tone queue for passive buzzer
namespace Sound {
    void init();
    void playStateSound(AvatarState state);
    void updateWaitingEscalation(uint32_t stateTimeMs);
    void update(uint32_t deltaMs);
    void setCustomMelodies(bool enabled);
    void setMelody(int stateIndex, const Melody& melody);
    void saveMelodiesToNVS();
    void loadMelodiesFromNVS();
}
