#pragma once
#include <Arduino.h>
#include "avatar.h"

#define MAX_KEYFRAMES 16

struct Keyframe {
    uint16_t time_ms;     // Time offset from start
    float eyeX, eyeY;
    float eyeOpen;
    float mouthCurve, mouthOpen;
    float bounce, shake;
    float browAngle, browY;
};

struct Animation {
    Keyframe frames[MAX_KEYFRAMES];
    uint8_t frameCount;
    bool loop;
    uint16_t duration_ms;
};

namespace AnimPlayer {
    void init();
    // Load animation from JSON (received from server)
    bool loadFromJson(const char* json);
    // Start/stop playback
    void play();
    void stop();
    bool isPlaying();
    // Call each frame - returns interpolated params, or false if not playing
    bool update(uint32_t deltaMs, AvatarParams& outParams);
}
