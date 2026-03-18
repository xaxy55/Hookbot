#ifndef NO_LED

#include "led.h"
#include "config.h"
#include "server.h"
#include <FastLED.h>

namespace Led {

static CRGB leds[NUM_LEDS];
static AvatarState currentState = AvatarState::IDLE;
static uint32_t stateTime = 0;

void init() {
    FastLED.addLeds<WS2812B, LED_PIN, GRB>(leds, NUM_LEDS);
    FastLED.setBrightness(HookbotServer::getConfig().ledBrightness);
    leds[0] = CRGB::Black;
    FastLED.show();
    Serial.println("[LED] Initialized");
}

void setState(AvatarState state) {
    currentState = state;
    stateTime = 0;
}

void update(uint32_t deltaMs) {
    stateTime += deltaMs;
    float phase = (float)stateTime / 1000.0f;

    switch (currentState) {
        case AvatarState::IDLE: {
            // Soft blue breathing
            uint8_t val = 30 + (uint8_t)(25.0f * (sinf(phase * 1.2f) * 0.5f + 0.5f));
            leds[0] = CRGB(0, 0, val);
            break;
        }

        case AvatarState::THINKING: {
            // Purple chase/pulse
            uint8_t r = 60 + (uint8_t)(40.0f * (sinf(phase * 3.0f) * 0.5f + 0.5f));
            uint8_t b = 80 + (uint8_t)(40.0f * (cosf(phase * 3.0f) * 0.5f + 0.5f));
            leds[0] = CRGB(r, 0, b);
            break;
        }

        case AvatarState::WAITING: {
            // Warm white pulse
            uint8_t val = 40 + (uint8_t)(30.0f * (sinf(phase * 1.5f) * 0.5f + 0.5f));
            leds[0] = CRGB(val, val - 10, val - 20);
            break;
        }

        case AvatarState::SUCCESS: {
            // Green flash then fade
            if (stateTime < 200) {
                leds[0] = CRGB(0, 200, 0);
            } else {
                float fade = max(0.0f, 1.0f - (float)(stateTime - 200) / 2000.0f);
                leds[0] = CRGB(0, (uint8_t)(120 * fade), 0);
            }
            break;
        }

        case AvatarState::TASKCHECK: {
            // Teal fill
            if (stateTime < 300) {
                uint8_t val = (uint8_t)(80.0f * (float)stateTime / 300.0f);
                leds[0] = CRGB(0, val, val);
            } else {
                float fade = max(0.0f, 1.0f - (float)(stateTime - 300) / 2000.0f);
                leds[0] = CRGB(0, (uint8_t)(80 * fade), (uint8_t)(80 * fade));
            }
            break;
        }

        case AvatarState::ERROR: {
            // Red double flash
            bool flash = (stateTime % 400) < 200;
            if (stateTime < 800 && flash) {
                leds[0] = CRGB(200, 0, 0);
            } else if (stateTime >= 800) {
                float fade = max(0.0f, 1.0f - (float)(stateTime - 800) / 1500.0f);
                leds[0] = CRGB((uint8_t)(100 * fade), 0, 0);
            } else {
                leds[0] = CRGB::Black;
            }
            break;
        }
    }

    // Apply runtime brightness
    FastLED.setBrightness(HookbotServer::getConfig().ledBrightness);
    FastLED.show();
}

} // namespace Led

#endif // !NO_LED
