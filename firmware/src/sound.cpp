#ifndef NO_SOUND

#include "sound.h"
#include "config.h"
#include <Arduino.h>
#include <Preferences.h>

namespace Sound {

struct Note {
    uint16_t freq;     // Hz (0 = rest)
    uint16_t duration; // ms
};

static const int MAX_QUEUE = 8;
static Note queue[MAX_QUEUE];
static int queueLen = 0;
static int queuePos = 0;
static uint32_t noteStart = 0;
static bool playing = false;

// Custom melodies per state (6 states: idle=0 through error=5)
static Melody customMelodies[6] = {};
static bool customEnabled = false;

// LEDC channel for tone
static const uint8_t LEDC_CHANNEL = 0;

void setCustomMelodies(bool enabled) {
    customEnabled = enabled;
    if (!enabled) {
        // Clear all custom melodies
        for (int i = 0; i < 6; i++) {
            customMelodies[i].count = 0;
        }
    }
    Serial.printf("[Sound] Custom melodies %s\n", enabled ? "enabled" : "disabled");
}

void setMelody(int stateIndex, const Melody& melody) {
    if (stateIndex < 0 || stateIndex > 5) return;
    customMelodies[stateIndex] = melody;
    Serial.printf("[Sound] Custom melody set for state %d (%d notes)\n", stateIndex, melody.count);
}

void saveMelodiesToNVS() {
    Preferences prefs;
    prefs.begin("sndpack", false);
    prefs.putBool("enabled", customEnabled);
    prefs.putBytes("melodies", customMelodies, sizeof(customMelodies));
    prefs.end();
    Serial.println("[Sound] Melodies saved to NVS");
}

void loadMelodiesFromNVS() {
    Preferences prefs;
    prefs.begin("sndpack", true);
    customEnabled = prefs.getBool("enabled", false);
    if (customEnabled) {
        prefs.getBytes("melodies", customMelodies, sizeof(customMelodies));
    }
    prefs.end();
    Serial.printf("[Sound] Melodies loaded from NVS, custom=%s\n", customEnabled ? "on" : "off");
}

void init() {
    ledcSetup(LEDC_CHANNEL, 1000, 8);
    ledcAttachPin(BUZZER_PIN, LEDC_CHANNEL);
    ledcWrite(LEDC_CHANNEL, 0);
    loadMelodiesFromNVS();
    Serial.println("[Sound] Initialized");
}

static void enqueue(uint16_t freq, uint16_t duration) {
    if (queueLen < MAX_QUEUE) {
        queue[queueLen++] = {freq, duration};
    }
}

static void startNote(const Note& note) {
    if (note.freq > 0) {
        ledcWriteTone(LEDC_CHANNEL, note.freq);
        ledcWrite(LEDC_CHANNEL, 128);  // 50% duty
    } else {
        ledcWrite(LEDC_CHANNEL, 0);
    }
    noteStart = millis();
    playing = true;
}

void playStateSound(AvatarState state) {
    // Clear queue and start fresh
    queueLen = 0;
    queuePos = 0;
    playing = false;
    ledcWrite(LEDC_CHANNEL, 0);

    int stateIdx = (int)state;

    // Use custom melody if enabled and defined for this state
    if (customEnabled && stateIdx >= 0 && stateIdx < 6 && customMelodies[stateIdx].count > 0) {
        const Melody& m = customMelodies[stateIdx];
        for (int i = 0; i < m.count && i < MAX_MELODY_NOTES; i++) {
            enqueue(m.freqs[i], m.durations[i]);
        }
    } else {
        // Default hardcoded melodies
        switch (state) {
            case AvatarState::IDLE:
                // Deep ominous power hum
                enqueue(110, 120);
                enqueue(0, 30);
                enqueue(110, 80);
                break;

            case AvatarState::THINKING:
                // Rapid impatient tapping - the CEO is scheming
                enqueue(330, 50);
                enqueue(0, 20);
                enqueue(370, 50);
                enqueue(0, 20);
                enqueue(415, 60);
                break;

            case AvatarState::WAITING:
                // Ominous low drone - displeasure
                enqueue(100, 200);
                break;

            case AvatarState::SUCCESS:
                // Triumphant fanfare - world conquered
                enqueue(440, 80);
                enqueue(0, 20);
                enqueue(554, 80);
                enqueue(0, 20);
                enqueue(659, 80);
                enqueue(0, 20);
                enqueue(880, 200);
                break;

            case AvatarState::TASKCHECK:
                // Authoritative double gavel strike
                enqueue(660, 60);
                enqueue(0, 60);
                enqueue(880, 80);
                break;

            case AvatarState::ERROR:
                // Rage descending - doom chord
                enqueue(880, 80);
                enqueue(0, 20);
                enqueue(440, 80);
                enqueue(0, 20);
                enqueue(220, 120);
                enqueue(0, 20);
                enqueue(110, 200);
                break;
        }
    }

    // Start first note
    if (queueLen > 0) {
        startNote(queue[0]);
        queuePos = 0;
    }
}

void updateWaitingEscalation(uint32_t stateTimeMs) {
    if (playing) return;  // Don't interrupt current sound

    float secs = (float)stateTimeMs / 1000.0f;
    uint32_t interval;

    if (secs < 3.0f) return;  // No sound in first phase
    else if (secs < 6.0f) interval = 4000;   // Annoyed: beep every 4s
    else if (secs < 10.0f) interval = 2000;  // Angry: beep every 2s
    else interval = 800;                      // FURIOUS: rapid beeping

    // Check if it's time for a beep
    static uint32_t lastBeep = 0;
    if (stateTimeMs - lastBeep < interval) return;
    lastBeep = stateTimeMs;

    queueLen = 0;
    queuePos = 0;

    if (secs < 6.0f) {
        // Annoyed: impatient tap
        enqueue(300, 40);
    } else if (secs < 10.0f) {
        // Angry: harsh double beep
        enqueue(500, 50);
        enqueue(0, 30);
        enqueue(600, 60);
    } else {
        // FURIOUS: alarm klaxon
        enqueue(800, 60);
        enqueue(0, 20);
        enqueue(400, 60);
        enqueue(0, 20);
        enqueue(800, 60);
    }

    if (queueLen > 0) {
        startNote(queue[0]);
        queuePos = 0;
    }
}

void update(uint32_t deltaMs) {
    if (!playing || queueLen == 0) return;

    uint32_t elapsed = millis() - noteStart;
    if (elapsed >= queue[queuePos].duration) {
        queuePos++;
        if (queuePos < queueLen) {
            startNote(queue[queuePos]);
        } else {
            // Done
            ledcWrite(LEDC_CHANNEL, 0);
            playing = false;
            queueLen = 0;
            queuePos = 0;
        }
    }
}

} // namespace Sound

#endif // !NO_SOUND
