#pragma once

#include <Arduino.h>

// Avatar states
enum class AvatarState : uint8_t {
    IDLE,
    THINKING,
    WAITING,
    SUCCESS,
    TASKCHECK,
    ERROR
};

// Smoothly interpolated face parameters (simplified for 128x64)
struct AvatarParams {
    float eyeX      = 0.0f;   // Eye horizontal offset (-1 to 1)
    float eyeY      = 0.0f;   // Eye vertical offset (-1 to 1)
    float eyeOpen   = 1.0f;   // Eye openness (0=closed, 1=open)
    float mouthCurve = 0.0f;  // Mouth curve (-1=frown, 0=neutral, 1=smile)
    float mouthOpen  = 0.0f;  // Mouth openness (0=closed, 1=open)
    float bounce     = 0.0f;  // Vertical bounce offset
    float shake      = 0.0f;  // Horizontal shake offset
    float browAngle  = 0.0f;  // Eyebrow angle (-1=angry V, 0=neutral, 1=raised)
    float browY      = 0.0f;  // Eyebrow vertical offset
};

// Avatar drawing and animation subsystem
namespace Avatar {
    void init();
    void setState(AvatarState state);
    AvatarState getState();
    void update(uint32_t deltaMs);
    void draw();
    // Override face parameters for one frame (used by animation player)
    void overrideParams(const AvatarParams& params);
    // QR code display (full-screen overlay when unclaimed)
    bool isShowingQR();
    void showQR(bool show);
}
