#include "config.h"
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

/// ESP32Servo hands out LEDC timers from a pool that starts empty; without
/// this, attach() finds no timer and fails, and the servo simply never moves.
/// Timer 0 is deliberately left alone: on the round board the display
/// backlight drives LEDC channel 0, and taking its timer kills the backlight.
static void allocateTimersOnce() {
    static bool done = false;
    if (done) return;
    done = true;
#ifdef DISPLAY_LGFX
    // Timer 0 drives the panel backlight on the LCD boards; taking it would
    // black out the screen. Timer 3 has no LEDC channels mapped to it on the
    // 6-channel C6, so 1 and 2 are what is actually usable.
    ESP32PWM::allocateTimer(1);
    ESP32PWM::allocateTimer(2);
#else
    ESP32PWM::allocateTimer(0);
    ESP32PWM::allocateTimer(1);
    ESP32PWM::allocateTimer(2);
    ESP32PWM::allocateTimer(3);
#endif
}

/// Attach one channel, reporting whether it actually worked. attach() returns
/// 0 on failure; the old code ignored that and marked the channel attached
/// regardless, so a servo that was never driven looked perfectly healthy in
/// GET /servos.
static bool attachChannel(int i) {
    allocateTimersOnce();
    servoObjects[i].setPeriodHertz(50);
    // attach() returns the LEDC channel, and channel 0 is a perfectly valid
    // one; failure is -1. Testing for 0 reports a working servo as broken.
    int ch = servoObjects[i].attach(channels[i].pin, 500, 2400);
    if (ch < 0) {
        attached[i] = false;
        Serial.printf("[Servo] Ch%d FAILED to attach on pin %d (no LEDC timer free)\n",
                      i, channels[i].pin);
        return false;
    }
    servoObjects[i].write(channels[i].restAngle);
    attached[i] = true;
    currentSmooth[i] = channels[i].restAngle;
    targetAngle[i] = channels[i].restAngle;
    Serial.printf("[Servo] Ch%d on pin %d (%s) -> %d deg\n",
                  i, channels[i].pin, channels[i].label, channels[i].restAngle);
    return true;
}

void init() {
    loadFromNVS();

    for (int i = 0; i < MAX_SERVOS; i++) {
        if (channels[i].pin >= 0 && channels[i].enabled) {
            attachChannel(i);
        }
    }
    Serial.println("[Servo] Initialized");
}

bool isAttached(uint8_t ch) {
    return ch < MAX_SERVOS && attached[ch];
}

// Sweep state. The HTTP handler runs on the async server task, where a
// blocking delay() stalls the whole web server and drops the caller's
// connection — so the request only records what to do and update() steps
// through it.
static int8_t sweepCh = -1;
static uint8_t sweepPhase = 0;      // index into the waypoints below
static float sweepPos = 90.0f;      // angle currently commanded
static uint32_t sweepNextMs = 0;

// Step toward each waypoint in short hops with a pause between, rather than
// driving continuously. Measured on hardware: a single 5-15 degree move is
// fine, but ~1s of continuous travel browns out a board whose servo shares the
// USB supply and resets the device mid-test — which looks exactly like the
// fault the test exists to find. Hopping lets the rail recover between moves.
// This mitigates; it does not cure. A servo that must run continuously needs
// its own supply, common ground, and a bulk capacitor.
static const uint32_t SWEEP_HOP_MS = 260;   // pause between hops
static const int SWEEP_HOP_DEG = 10;        // travel per hop
static const int SWEEP_ARC_DEG = 35;

bool requestSweep(uint8_t ch) {
    if (ch >= MAX_SERVOS || !attached[ch]) return false;
    sweepCh = (int8_t)ch;
    sweepPhase = 0;
    sweepPos = (float)channels[ch].currentAngle;
    sweepNextMs = 0;  // start on the next update()
    return true;
}

bool isSweeping() { return sweepCh >= 0; }

/// Drive the channel through its range, bypassing the state animation in
/// update(), so the result is unambiguous: if the horn does not move for this,
/// the problem is wiring or power rather than the animation.
static void stepSweep() {
    if (sweepCh < 0 || millis() < sweepNextMs) return;

    const ServoChannel& c = channels[sweepCh];
    // A bounded arc around rest rather than the full min..max travel. Two
    // reasons, both learned the hard way on real hardware: driving to the
    // mechanical limits can push a mounted horn into the enclosure, and the
    // sustained current of a long excursion browns out a board whose servo
    // shares the USB supply — the device resets mid-test, which looks exactly
    // like the failure the test is meant to diagnose.
    const int lo = max((int)c.minAngle, (int)c.restAngle - SWEEP_ARC_DEG);
    const int hi = min((int)c.maxAngle, (int)c.restAngle + SWEEP_ARC_DEG);
    const uint8_t waypoints[3] = { (uint8_t)lo, (uint8_t)hi, c.restAngle };

    if (sweepPhase >= 3) {
        currentSmooth[sweepCh] = c.restAngle;
        targetAngle[sweepCh] = c.restAngle;
        channels[sweepCh].currentAngle = c.restAngle;
        Serial.printf("[Servo] Ch%d sweep done\n", sweepCh);
        sweepCh = -1;
        return;
    }

    const float target = (float)waypoints[sweepPhase];
    const float diff = target - sweepPos;
    if (fabsf(diff) <= (float)SWEEP_HOP_DEG) {
        sweepPos = target;
        sweepPhase++;
    } else {
        sweepPos += (diff > 0 ? (float)SWEEP_HOP_DEG : -(float)SWEEP_HOP_DEG);
    }

    uint8_t angle = (uint8_t)constrain((int)lroundf(sweepPos), c.minAngle, c.maxAngle);
    servoObjects[sweepCh].write(angle);
    channels[sweepCh].currentAngle = angle;
    sweepNextMs = millis() + SWEEP_HOP_MS;
}

void update(uint32_t deltaMs) {
    // A sweep is a manual test; let it own the servo until it finishes rather
    // than fighting the state animation for the same channel.
    if (sweepCh >= 0) {
        stepSweep();
        return;
    }

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
        allocateTimersOnce();
        servoObjects[ch].setPeriodHertz(50);
        if (servoObjects[ch].attach(pin, 500, 2400) < 0) {
            // Still persist below: the operator can fix the wiring or free a
            // timer and reboot, and the configuration should survive that.
            attached[ch] = false;
            Serial.printf("[Servo] Ch%d FAILED to attach on pin %d\n", ch, pin);
        } else {
            servoObjects[ch].write(rest);
            attached[ch] = true;
            currentSmooth[ch] = rest;
            targetAngle[ch] = rest;
        }
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
