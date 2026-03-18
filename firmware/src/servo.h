#pragma once

#include "avatar.h"
#include <Arduino.h>

#define MAX_SERVOS 4

struct ServoChannel {
    int8_t pin;          // GPIO pin (-1 = disabled)
    uint8_t minAngle;    // Min angle (default 0)
    uint8_t maxAngle;    // Max angle (default 180)
    uint8_t restAngle;   // Rest/neutral position
    uint8_t currentAngle;
    char label[12];      // "head_tilt", "head_pan", "left_hand", "right_hand"
    bool enabled;
};

// State-linked servo positions: what angle each servo goes to per avatar state
struct ServoStateMap {
    uint8_t angles[MAX_SERVOS]; // angle per servo channel
};

// Tool-specific hand gesture overrides (left_hand, right_hand angles)
struct ToolHandPose {
    uint8_t leftHand;
    uint8_t rightHand;
};

namespace Servos {
    void init();
    void update(uint32_t deltaMs);
    void setAngle(uint8_t channel, uint8_t angle);
    void setAllToRest();
    ServoChannel* getChannels();
    void configureChannel(uint8_t channel, int8_t pin, uint8_t minA, uint8_t maxA, uint8_t rest, const char* label);
    void onStateChange(AvatarState state);
    void onToolChange(const char* toolName);  // Tool-specific hand gestures
    void loadFromNVS();
    void saveToNVS();
    ServoStateMap* getStateMaps();
}
