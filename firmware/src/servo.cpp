#include "servo.h"
#include <ESP32Servo.h>
#include <Preferences.h>

namespace Servos {

static ServoChannel channels[MAX_SERVOS] = {
    { -1, 0, 180, 90, 90, "head_tilt", false },
    { -1, 0, 180, 90, 90, "head_pan",  false },
    { -1, 0, 180, 90, 90, "left_hand", false },
    { -1, 0, 180, 90, 90, "right_hand", false },
};

static Servo servoObjects[MAX_SERVOS];
static bool attached[MAX_SERVOS] = {};

// Smooth movement
static float targetAngle[MAX_SERVOS] = { 90, 90, 90, 90 };
static float currentSmooth[MAX_SERVOS] = { 90, 90, 90, 90 };
static const float SMOOTH_SPEED = 5.0f; // degrees per ms * speed factor

// State-linked positions (default: all rest)
static ServoStateMap stateMaps[6] = {
    // IDLE
    { { 90, 90, 90, 90 } },
    // THINKING: slight head tilt, hands fidget
    { { 75, 90, 70, 110 } },
    // WAITING: head turned, arms crossed feel
    { { 90, 70, 45, 135 } },
    // SUCCESS: head up, arms raised
    { { 110, 90, 150, 30 } },
    // TASKCHECK: nod down
    { { 70, 90, 90, 90 } },
    // ERROR: head shake position, hands defensive
    { { 90, 110, 60, 120 } },
};

void init() {
    loadFromNVS();

    for (int i = 0; i < MAX_SERVOS; i++) {
        if (channels[i].pin >= 0 && channels[i].enabled) {
            servoObjects[i].setPeriodHertz(50);
            servoObjects[i].attach(channels[i].pin, 500, 2400);
            servoObjects[i].write(channels[i].restAngle);
            attached[i] = true;
            currentSmooth[i] = channels[i].restAngle;
            targetAngle[i] = channels[i].restAngle;
            Serial.printf("[Servo] Ch%d on pin %d (%s) -> %d°\n",
                i, channels[i].pin, channels[i].label, channels[i].restAngle);
        }
    }
    Serial.println("[Servo] Initialized");
}

void update(uint32_t deltaMs) {
    float dt = (float)deltaMs / 1000.0f;

    for (int i = 0; i < MAX_SERVOS; i++) {
        if (!attached[i]) continue;

        // Smooth interpolation toward target
        float diff = targetAngle[i] - currentSmooth[i];
        if (fabsf(diff) > 0.5f) {
            currentSmooth[i] += diff * min(1.0f, SMOOTH_SPEED * dt);
            uint8_t angle = (uint8_t)constrain((int)currentSmooth[i], channels[i].minAngle, channels[i].maxAngle);
            if (angle != channels[i].currentAngle) {
                channels[i].currentAngle = angle;
                servoObjects[i].write(angle);
            }
        }
    }
}

void setAngle(uint8_t channel, uint8_t angle) {
    if (channel >= MAX_SERVOS) return;
    angle = constrain(angle, channels[channel].minAngle, channels[channel].maxAngle);
    targetAngle[channel] = angle;
}

void setAllToRest() {
    for (int i = 0; i < MAX_SERVOS; i++) {
        targetAngle[i] = channels[i].restAngle;
    }
}

ServoChannel* getChannels() {
    return channels;
}

ServoStateMap* getStateMaps() {
    return stateMaps;
}

void configureChannel(uint8_t ch, int8_t pin, uint8_t minA, uint8_t maxA, uint8_t rest, const char* label) {
    if (ch >= MAX_SERVOS) return;

    // Detach old if changing pin
    if (attached[ch]) {
        servoObjects[ch].detach();
        attached[ch] = false;
    }

    channels[ch].pin = pin;
    channels[ch].minAngle = minA;
    channels[ch].maxAngle = maxA;
    channels[ch].restAngle = rest;
    channels[ch].enabled = (pin >= 0);
    strncpy(channels[ch].label, label, sizeof(channels[ch].label) - 1);

    // Attach new
    if (pin >= 0 && channels[ch].enabled) {
        servoObjects[ch].setPeriodHertz(50);
        servoObjects[ch].attach(pin, 500, 2400);
        servoObjects[ch].write(rest);
        attached[ch] = true;
        currentSmooth[ch] = rest;
        targetAngle[ch] = rest;
    }

    saveToNVS();
    Serial.printf("[Servo] Ch%d configured: pin=%d range=%d-%d rest=%d label=%s\n",
        ch, pin, minA, maxA, rest, label);
}

// Tool-specific hand poses: map tool names to left_hand (ch2) and right_hand (ch3) angles
// These override state map channels 2 & 3 during THINKING state
static const struct { const char* tool; uint8_t left; uint8_t right; } toolPoses[] = {
    // Read/search: one hand raised to "eye" level (reading), other relaxed
    { "Read",    120, 70  },
    { "Grep",    120, 70  },
    { "Glob",    120, 70  },
    // Write/Edit: both hands forward, typing gesture
    { "Write",   100, 80  },
    { "Edit",    105, 75  },
    // Bash: commanding point - one hand outstretched
    { "Bash",    60,  30  },
    // Agent: both hands out wide, delegating
    { "Agent",   150, 30  },
    // LSP: both hands hovering, analyzing
    { "LSP",     100, 80  },
};
static const int NUM_TOOL_POSES = sizeof(toolPoses) / sizeof(toolPoses[0]);

static bool toolOverrideActive = false;

void onStateChange(AvatarState state) {
    int idx = (int)state;
    if (idx < 0 || idx >= 6) return;

    // Clear tool override when leaving THINKING
    if (state != AvatarState::THINKING) {
        toolOverrideActive = false;
    }

    for (int i = 0; i < MAX_SERVOS; i++) {
        if (attached[i]) {
            targetAngle[i] = stateMaps[idx].angles[i];
        }
    }
}

void onToolChange(const char* toolName) {
    if (!toolName || strlen(toolName) == 0) return;

    for (int t = 0; t < NUM_TOOL_POSES; t++) {
        if (strcmp(toolName, toolPoses[t].tool) == 0) {
            // Override hand channels (2=left, 3=right) with tool-specific pose
            if (attached[2]) targetAngle[2] = toolPoses[t].left;
            if (attached[3]) targetAngle[3] = toolPoses[t].right;
            toolOverrideActive = true;
            Serial.printf("[Servo] Tool pose: %s -> L=%d R=%d\n",
                toolName, toolPoses[t].left, toolPoses[t].right);
            return;
        }
    }
    // Unknown tool: use default thinking pose
    toolOverrideActive = false;
}

void loadFromNVS() {
    Preferences prefs;
    prefs.begin("servos", true);
    for (int i = 0; i < MAX_SERVOS; i++) {
        char key[16];
        snprintf(key, sizeof(key), "pin%d", i);
        channels[i].pin = prefs.getChar(key, -1);
        snprintf(key, sizeof(key), "min%d", i);
        channels[i].minAngle = prefs.getUChar(key, 0);
        snprintf(key, sizeof(key), "max%d", i);
        channels[i].maxAngle = prefs.getUChar(key, 180);
        snprintf(key, sizeof(key), "rest%d", i);
        channels[i].restAngle = prefs.getUChar(key, 90);
        snprintf(key, sizeof(key), "en%d", i);
        channels[i].enabled = prefs.getBool(key, false);
        snprintf(key, sizeof(key), "lbl%d", i);
        String lbl = prefs.getString(key, channels[i].label);
        strncpy(channels[i].label, lbl.c_str(), sizeof(channels[i].label) - 1);
    }
    // State maps
    for (int s = 0; s < 6; s++) {
        char key[16];
        snprintf(key, sizeof(key), "sm%d", s);
        size_t len = prefs.getBytesLength(key);
        if (len == sizeof(ServoStateMap)) {
            prefs.getBytes(key, &stateMaps[s], sizeof(ServoStateMap));
        }
    }
    prefs.end();
}

void saveToNVS() {
    Preferences prefs;
    prefs.begin("servos", false);
    for (int i = 0; i < MAX_SERVOS; i++) {
        char key[16];
        snprintf(key, sizeof(key), "pin%d", i);
        prefs.putChar(key, channels[i].pin);
        snprintf(key, sizeof(key), "min%d", i);
        prefs.putUChar(key, channels[i].minAngle);
        snprintf(key, sizeof(key), "max%d", i);
        prefs.putUChar(key, channels[i].maxAngle);
        snprintf(key, sizeof(key), "rest%d", i);
        prefs.putUChar(key, channels[i].restAngle);
        snprintf(key, sizeof(key), "en%d", i);
        prefs.putBool(key, channels[i].enabled);
        snprintf(key, sizeof(key), "lbl%d", i);
        prefs.putString(key, channels[i].label);
    }
    for (int s = 0; s < 6; s++) {
        char key[16];
        snprintf(key, sizeof(key), "sm%d", s);
        prefs.putBytes(key, &stateMaps[s], sizeof(ServoStateMap));
    }
    prefs.end();
}

} // namespace Servos
