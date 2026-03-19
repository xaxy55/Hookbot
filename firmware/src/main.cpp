#include <Arduino.h>
#include <ArduinoOTA.h>
#include "config.h"
#ifndef NO_DISPLAY
#include "display.h"
#include "avatar.h"
#include "screensaver.h"
#include "animation_player.h"
#endif
#ifndef NO_LED
#include "led.h"
#endif
#ifndef NO_SOUND
#include "sound.h"
#endif
#include "server.h"
#include "servo.h"
#include "sensors.h"
#include "ble_prov.h"
#ifdef BOARD_ESP32_4848S040C
#include "touch_ui.h"
#endif

// ─── Screensaver ────────────────────────────────────────────────

#define SCREENSAVER_TIMEOUT_MS 300000  // 5 minutes of idle before screensaver

#ifndef NO_DISPLAY
static bool screensaverActive = false;
#endif

// ─── State machine ──────────────────────────────────────────────

static AvatarState currentState = AvatarState::IDLE;
static uint32_t stateEnteredAt = 0;

static void setState(AvatarState state) {
    currentState = state;
    stateEnteredAt = millis();
#ifndef NO_DISPLAY
    if (screensaverActive) {
        screensaverActive = false;
        Serial.println("[Main] Screensaver deactivated");
    }
    Avatar::setState(state);
#endif
#ifndef NO_LED
    Led::setState(state);
#endif
    Servos::onStateChange(state);
#ifndef NO_SOUND
    if (HookbotServer::getConfig().soundEnabled) {
        Sound::playStateSound(state);
    }
#endif
}

// ─── Touch input (LCD board only) ───────────────────────────────

#if defined(BOARD_ESP32_4848S040C) && !defined(NO_DISPLAY)
static uint32_t lastTouchTime = 0;
static bool wasTouchingMain = false;

static uint32_t lastTouchDebug = 0;

static void handleTouch(uint32_t deltaMs) {
    int16_t tx, ty;
    bool touching = Display::getTouchPoint(tx, ty);

    // Debug: log touch events (throttled to once per second)
    if (touching && (millis() - lastTouchDebug > 1000)) {
        Serial.printf("[Touch] DETECTED x=%d y=%d\n", tx, ty);
        lastTouchDebug = millis();
    }

    // Any touch dismisses the screensaver (wake on touch)
    if (touching && screensaverActive) {
        screensaverActive = false;
        stateEnteredAt = millis();  // Reset idle timer so it doesn't reactivate
        wasTouchingMain = touching;
        Serial.println("[Touch] Screensaver dismissed");
        return;  // Consume this touch as a wake gesture
    }

    // Feed touch data to the overlay UI
    TouchUI::update(deltaMs, tx, ty, touching);

    // If overlay is active, it consumes all touches
    if (TouchUI::isOverlayActive()) {
        wasTouchingMain = touching;
        return;
    }

    // Default touch behavior when no overlay
    if (touching && !wasTouchingMain && (millis() - lastTouchTime > 500)) {
        lastTouchTime = millis();

        // Tap zones on the 120x120 virtual canvas:
        // Top half: cycle forward through states
        // Bottom half: back to idle
        if (ty < 60) {
            int next = ((int)currentState + 1) % 6;
            setState((AvatarState)next);
            Serial.printf("[Touch] State -> %d\n", next);
        } else {
            setState(AvatarState::IDLE);
            Serial.println("[Touch] State -> IDLE");
        }
    }
    wasTouchingMain = touching;
}
#endif

// ─── Setup ──────────────────────────────────────────────────────

void setup() {
    Serial.begin(115200);
    delay(500);
    Serial.println("\n=== THE DESTROYER OF WORLDS IS BOOTING ===");
    Serial.printf("[Main] Free heap: %d\n", ESP.getFreeHeap());
#if BOARD_HAS_PSRAM
    Serial.printf("[Main] PSRAM: %d bytes\n", ESP.getFreePsram());
#endif

#ifndef NO_DISPLAY
    Serial.println("[Main] Display init...");
    Display::init();
    Serial.println("[Main] Avatar init...");
    Avatar::init();
    AnimPlayer::init();
#endif
#ifndef NO_LED
    Serial.println("[Main] LED init...");
    Led::init();
#endif
#ifndef NO_SOUND
    Sound::init();
#endif
    Serial.println("[Main] Servo init...");
    Servos::init();
    Serial.println("[Main] Sensors init...");
    Sensors::init();

#ifndef NO_DISPLAY
    Screensaver::init();
    Avatar::draw();
    Display::flush();
#endif

    Serial.println("[Main] Server init...");
    HookbotServer::init([](AvatarState newState) {
        setState(newState);
        Serial.printf("[Main] State changed to %d via HTTP\n", (int)newState);
    });

    // OTA updates - no more USB cables for the CEO
    ArduinoOTA.setHostname(HookbotServer::getConfig().hostname);
    ArduinoOTA.onStart([]() {
        Serial.println("[OTA] Firmware update incoming...");
    });
    ArduinoOTA.onEnd([]() {
        Serial.println("[OTA] Update complete. Rebooting the Destroyer...");
    });
    ArduinoOTA.onError([](ota_error_t error) {
        Serial.printf("[OTA] Error[%u]\n", error);
    });
    ArduinoOTA.begin();

    Serial.printf("[Main] Free heap: %d bytes\n", ESP.getFreeHeap());
#ifdef BOARD_HAS_PSRAM
    if (BOARD_HAS_PSRAM) {
        Serial.printf("[Main] Free PSRAM: %d bytes\n", ESP.getFreePsram());
    }
#endif
    Serial.println("=== THE CEO HAS ARRIVED. TREMBLE. ===");

    BleProv::init();
#ifdef BOARD_ESP32_4848S040C
    TouchUI::init();
    // Apply saved display brightness from config
    {
        RuntimeConfig& cfg = HookbotServer::getConfig();
        if (cfg.ledBrightness > 0) {
            Display::setBrightness(cfg.ledBrightness);
            Serial.printf("[Main] Display brightness set to %d from config\n", cfg.ledBrightness);
        }
    }
    Display::touchTest();
#endif
}

// ─── Loop ───────────────────────────────────────────────────────

static uint32_t lastFrame = 0;

void loop() {
    uint32_t now = millis();
    uint32_t delta = now - lastFrame;

    if (delta < FRAME_TIME_MS) {
        return;  // Frame rate limiter
    }
    lastFrame = now;

    // Auto-return from transient states
    if ((currentState == AvatarState::SUCCESS || currentState == AvatarState::TASKCHECK)
        && (now - stateEnteredAt >= AUTO_RETURN_MS)) {
        setState(AvatarState::IDLE);
    }

    // Update all subsystems
#ifndef NO_DISPLAY
    // When custom animation is playing, override avatar params
    if (AnimPlayer::isPlaying()) {
        AvatarParams animParams;
        if (AnimPlayer::update(delta, animParams)) {
            Avatar::overrideParams(animParams);
        }
    } else {
        Avatar::update(delta);
    }
#endif
#ifndef NO_LED
    Led::update(delta);
#endif
    Servos::update(delta);
    Sensors::update(delta);
#ifndef NO_SOUND
    Sound::update(delta);
    // Escalating angry beeps when waiting for user input
    if (currentState == AvatarState::WAITING) {
        Sound::updateWaitingEscalation(now - stateEnteredAt);
    }
#endif
    HookbotServer::update();
    ArduinoOTA.handle();
    BleProv::update();

#if defined(BOARD_ESP32_4848S040C) && !defined(NO_DISPLAY)
    handleTouch(delta);
#endif

#ifndef NO_DISPLAY
    // Screensaver: activate after extended idle, prevent OLED burn-in
    if (currentState == AvatarState::IDLE
        && (now - stateEnteredAt >= SCREENSAVER_TIMEOUT_MS)
        && !screensaverActive) {
        screensaverActive = true;
        Screensaver::randomize();
        Serial.println("[Main] Screensaver activated");
    }

    if (screensaverActive) {
        Display::clear();
        Screensaver::update(delta);
        Screensaver::draw();
    } else {
        Avatar::draw();
    }
#ifdef BOARD_ESP32_4848S040C
    TouchUI::draw();  // Draw touch overlay on top of avatar
#endif
    Display::flush();
#endif
}
