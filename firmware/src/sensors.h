#pragma once
#include <Arduino.h>

#define MAX_SENSOR_CHANNELS 8

enum class SensorType : uint8_t {
    None         = 0,
    Digital      = 1,
    Analog       = 2,
    AmbientLight = 3,
};

struct SensorChannel {
    int8_t   pin            = -1;
    SensorType type         = SensorType::None;
    char     label[16]      = "";
    uint16_t pollIntervalMs = 1000;
    int16_t  threshold      = 0;
    int16_t  lastValue      = 0;
    uint32_t lastReadAt     = 0;
    bool     triggered      = false;
    // Button / debounce support
    uint32_t lastChangeAt   = 0;
    int16_t  prevValue      = 0;
    uint32_t pressStartAt   = 0;
    // Presence detection
    uint32_t lastMotionAt   = 0;
};

namespace Sensors {
    void init();
    void update(uint32_t deltaMs);
    SensorChannel* getChannels();
    int  getChannelCount();
    void configureChannel(uint8_t ch, int8_t pin, SensorType type,
                          const char* label, uint16_t pollMs, int16_t threshold);
    void saveToNVS();
    void loadFromNVS();
    bool isPresenceAway();
    uint32_t getPresenceTimeoutMs();
    void setPresenceTimeoutMs(uint32_t ms);
    int getAmbientLightValue();
    bool isAmbientLightConfigured();
}
