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

// Per-element colours, RGB565. Every element defaults to white, which is what
// the avatar has always been, so an unconfigured device looks unchanged.
//
// On the monochrome OLED there is only one ink colour, so setPalette() is a
// no-op there and every field stays COLOR_WHITE — the drawing code does not
// need to care which panel it is on.
struct AvatarPalette {
    uint16_t face;        // outline, eyebrows, thought bubbles, Zzz
    uint16_t eyes;
    uint16_t mouth;
    uint16_t headphones;  // shown while music plays
    uint16_t crown;
    uint16_t hat;
    uint16_t glasses;     // glasses and monocle
    uint16_t accessory;   // horns, halo, cigar, bow tie
    uint16_t music;       // now-playing track text
    uint16_t text;        // project, branch, tool and task text
    uint16_t accent;      // XP bar, wifi, state markers
};

// Avatar drawing and animation subsystem
namespace Avatar {
    const AvatarPalette& palette();
    void setPalette(const AvatarPalette& p);
    /// Parse a "#RRGGBB" string into RGB565. Returns `fallback` if malformed.
    uint16_t colorFromHex(const char* hex, uint16_t fallback);

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
