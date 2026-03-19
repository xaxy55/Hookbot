#include "sensors.h"
#include <Preferences.h>

namespace Sensors {

static SensorChannel channels[MAX_SENSOR_CHANNELS];
static uint32_t presenceTimeoutMs = 300000; // 5 minutes default
static uint32_t globalTime = 0;

// ─── Persistence ────────────────────────────────────────────────

void loadFromNVS() {
    Preferences prefs;
    prefs.begin("sensors", true); // read-only
    presenceTimeoutMs = prefs.getUInt("presTimeout", 300000);
    size_t len = prefs.getBytes("chData", channels, sizeof(channels));
    if (len != sizeof(channels)) {
        // No saved data or size mismatch — start with all disabled
        memset(channels, 0, sizeof(channels));
        for (int i = 0; i < MAX_SENSOR_CHANNELS; i++) {
            channels[i].pin  = -1;
            channels[i].type = SensorType::None;
            channels[i].pollIntervalMs = 1000;
        }
    }
    prefs.end();
    Serial.println("[Sensors] Config loaded from NVS");
}

void saveToNVS() {
    Preferences prefs;
    prefs.begin("sensors", false); // read-write
    prefs.putUInt("presTimeout", presenceTimeoutMs);
    prefs.putBytes("chData", channels, sizeof(channels));
    prefs.end();
    Serial.println("[Sensors] Config saved to NVS");
}

// ─── Init ───────────────────────────────────────────────────────

void init() {
    loadFromNVS();
    // Configure pins for enabled channels
    for (int i = 0; i < MAX_SENSOR_CHANNELS; i++) {
        if (channels[i].type != SensorType::None && channels[i].pin >= 0) {
            if (channels[i].type == SensorType::Digital) {
                pinMode(channels[i].pin, INPUT_PULLUP);
            }
            // ANALOG and AMBIENT_LIGHT pins don't need explicit pinMode on ESP32
            Serial.printf("[Sensors] Ch%d: pin=%d type=%d label=%s\n",
                i, channels[i].pin, (int)channels[i].type, channels[i].label);
        }
    }
}

// ─── Update ─────────────────────────────────────────────────────

void update(uint32_t deltaMs) {
    globalTime += deltaMs;

    for (int i = 0; i < MAX_SENSOR_CHANNELS; i++) {
        SensorChannel& ch = channels[i];
        if (ch.type == SensorType::None || ch.pin < 0) continue;

        // Check poll interval
        if (globalTime - ch.lastReadAt < ch.pollIntervalMs) continue;
        ch.lastReadAt = globalTime;

        int16_t raw = 0;

        if (ch.type == SensorType::Digital) {
            raw = digitalRead(ch.pin);

            // Debounce: ignore changes within 50ms
            if (raw != ch.prevValue) {
                if (globalTime - ch.lastChangeAt < 50) {
                    continue; // skip this read, too soon
                }
                ch.lastChangeAt = globalTime;

                // Button press detection (active LOW with INPUT_PULLUP)
                if (raw == LOW && ch.prevValue == HIGH) {
                    // Press started
                    ch.pressStartAt = globalTime;
                }
                if (raw == HIGH && ch.prevValue == LOW) {
                    // Released — check duration
                    uint32_t pressDuration = globalTime - ch.pressStartAt;
                    if (pressDuration >= 500) {
                        // Long press
                        Serial.printf("[Sensors] Ch%d long press (%dms)\n", i, (int)pressDuration);
                    } else {
                        // Short press
                        Serial.printf("[Sensors] Ch%d short press (%dms)\n", i, (int)pressDuration);
                    }
                }

                ch.prevValue = raw;
            }

            // PIR / motion: track last motion time when pin goes HIGH
            if (raw == HIGH) {
                ch.lastMotionAt = globalTime;
            }

            // Threshold crossing for digital
            bool nowTriggered = (raw == HIGH);
            if (nowTriggered != ch.triggered) {
                ch.triggered = nowTriggered;
                Serial.printf("[Sensors] Ch%d triggered=%d\n", i, ch.triggered);
            }
        }
        else if (ch.type == SensorType::Analog) {
            raw = analogRead(ch.pin);

            // Threshold crossing detection
            bool nowTriggered = (raw >= ch.threshold);
            if (nowTriggered != ch.triggered) {
                ch.triggered = nowTriggered;
                Serial.printf("[Sensors] Ch%d analog=%d threshold=%d triggered=%d\n",
                    i, raw, ch.threshold, ch.triggered);
            }
        }
        else if (ch.type == SensorType::AmbientLight) {
            raw = analogRead(ch.pin);
            // No threshold logic needed — value is consumed by Led auto-brightness
        }

        ch.lastValue = raw;
    }
}

// ─── Accessors ──────────────────────────────────────────────────

SensorChannel* getChannels() {
    return channels;
}

int getChannelCount() {
    return MAX_SENSOR_CHANNELS;
}

void configureChannel(uint8_t ch, int8_t pin, SensorType type,
                      const char* label, uint16_t pollMs, int16_t threshold) {
    if (ch >= MAX_SENSOR_CHANNELS) return;

    channels[ch].pin  = pin;
    channels[ch].type = type;
    strncpy(channels[ch].label, label, sizeof(channels[ch].label) - 1);
    channels[ch].label[sizeof(channels[ch].label) - 1] = '\0';
    channels[ch].pollIntervalMs = pollMs;
    channels[ch].threshold = threshold;
    channels[ch].lastValue = 0;
    channels[ch].triggered = false;
    channels[ch].prevValue = 0;
    channels[ch].pressStartAt = 0;
    channels[ch].lastChangeAt = 0;
    channels[ch].lastMotionAt = 0;

    // Configure pin
    if (type == SensorType::Digital && pin >= 0) {
        pinMode(pin, INPUT_PULLUP);
    }

    Serial.printf("[Sensors] Configured ch%d: pin=%d type=%d label=%s poll=%dms thresh=%d\n",
        ch, pin, (int)type, label, pollMs, threshold);
}

// ─── Presence detection ─────────────────────────────────────────

bool isPresenceAway() {
    // Check all digital channels for recent motion
    uint32_t latestMotion = 0;
    bool hasMotionSensor = false;
    for (int i = 0; i < MAX_SENSOR_CHANNELS; i++) {
        if (channels[i].type == SensorType::Digital && channels[i].pin >= 0) {
            hasMotionSensor = true;
            if (channels[i].lastMotionAt > latestMotion) {
                latestMotion = channels[i].lastMotionAt;
            }
        }
    }
    if (!hasMotionSensor) return false; // no motion sensors = not away
    return (globalTime - latestMotion) > presenceTimeoutMs;
}

uint32_t getPresenceTimeoutMs() {
    return presenceTimeoutMs;
}

void setPresenceTimeoutMs(uint32_t ms) {
    presenceTimeoutMs = ms;
}

// ─── Ambient light ──────────────────────────────────────────────

int getAmbientLightValue() {
    for (int i = 0; i < MAX_SENSOR_CHANNELS; i++) {
        if (channels[i].type == SensorType::AmbientLight && channels[i].pin >= 0) {
            return channels[i].lastValue;
        }
    }
    return -1; // no ambient light sensor configured
}

bool isAmbientLightConfigured() {
    for (int i = 0; i < MAX_SENSOR_CHANNELS; i++) {
        if (channels[i].type == SensorType::AmbientLight && channels[i].pin >= 0) {
            return true;
        }
    }
    return false;
}

} // namespace Sensors
