#include "animation_player.h"
#include <ArduinoJson.h>

namespace AnimPlayer {

static Animation currentAnim;
static bool animPlaying = false;
static uint32_t elapsedMs = 0;

void init() {
    currentAnim.frameCount = 0;
    currentAnim.loop = false;
    currentAnim.duration_ms = 0;
    animPlaying = false;
    elapsedMs = 0;
    Serial.println("[AnimPlayer] Initialized");
}

bool loadFromJson(const char* json) {
    JsonDocument doc;
    DeserializationError err = deserializeJson(doc, json);
    if (err) {
        Serial.printf("[AnimPlayer] JSON parse error: %s\n", err.c_str());
        return false;
    }

    currentAnim.loop = doc["loop"] | false;
    currentAnim.duration_ms = doc["duration_ms"] | 2000;

    JsonArray frames = doc["frames"];
    if (frames.isNull() || frames.size() == 0) {
        Serial.println("[AnimPlayer] No frames in animation");
        return false;
    }

    currentAnim.frameCount = 0;
    for (size_t i = 0; i < frames.size() && i < MAX_KEYFRAMES; i++) {
        JsonObject f = frames[i];
        Keyframe& kf = currentAnim.frames[i];
        kf.time_ms    = f["time"] | 0;
        kf.eyeX       = f["eyeX"] | 0.0f;
        kf.eyeY       = f["eyeY"] | 0.0f;
        kf.eyeOpen    = f["eyeOpen"] | 1.0f;
        kf.mouthCurve = f["mouthCurve"] | 0.0f;
        kf.mouthOpen  = f["mouthOpen"] | 0.0f;
        kf.bounce     = f["bounce"] | 0.0f;
        kf.shake      = f["shake"] | 0.0f;
        kf.browAngle  = f["browAngle"] | 0.0f;
        kf.browY      = f["browY"] | 0.0f;
        currentAnim.frameCount++;
    }

    Serial.printf("[AnimPlayer] Loaded %d keyframes, duration=%dms, loop=%d\n",
        currentAnim.frameCount, currentAnim.duration_ms, currentAnim.loop);
    return true;
}

void play() {
    if (currentAnim.frameCount == 0) return;
    elapsedMs = 0;
    animPlaying = true;
    Serial.println("[AnimPlayer] Play");
}

void stop() {
    animPlaying = false;
    Serial.println("[AnimPlayer] Stop");
}

bool isPlaying() {
    return animPlaying;
}

static float lerpf(float a, float b, float t) {
    return a + (b - a) * t;
}

static void interpolateKeyframes(const Keyframe& a, const Keyframe& b, float t, AvatarParams& out) {
    out.eyeX       = lerpf(a.eyeX, b.eyeX, t);
    out.eyeY       = lerpf(a.eyeY, b.eyeY, t);
    out.eyeOpen    = lerpf(a.eyeOpen, b.eyeOpen, t);
    out.mouthCurve = lerpf(a.mouthCurve, b.mouthCurve, t);
    out.mouthOpen  = lerpf(a.mouthOpen, b.mouthOpen, t);
    out.bounce     = lerpf(a.bounce, b.bounce, t);
    out.shake      = lerpf(a.shake, b.shake, t);
    out.browAngle  = lerpf(a.browAngle, b.browAngle, t);
    out.browY      = lerpf(a.browY, b.browY, t);
}

bool update(uint32_t deltaMs, AvatarParams& outParams) {
    if (!animPlaying || currentAnim.frameCount == 0) return false;

    elapsedMs += deltaMs;

    // Check if animation finished
    if (elapsedMs >= currentAnim.duration_ms) {
        if (currentAnim.loop) {
            elapsedMs = elapsedMs % currentAnim.duration_ms;
        } else {
            animPlaying = false;
            // Output the last keyframe params
            const Keyframe& last = currentAnim.frames[currentAnim.frameCount - 1];
            outParams.eyeX       = last.eyeX;
            outParams.eyeY       = last.eyeY;
            outParams.eyeOpen    = last.eyeOpen;
            outParams.mouthCurve = last.mouthCurve;
            outParams.mouthOpen  = last.mouthOpen;
            outParams.bounce     = last.bounce;
            outParams.shake      = last.shake;
            outParams.browAngle  = last.browAngle;
            outParams.browY      = last.browY;
            return false;
        }
    }

    // Find the two keyframes to interpolate between
    uint8_t frameIdx = 0;
    for (uint8_t i = 0; i < currentAnim.frameCount - 1; i++) {
        if (currentAnim.frames[i + 1].time_ms <= elapsedMs) {
            frameIdx = i + 1;
        } else {
            break;
        }
    }

    // If we're at or past the last keyframe
    if (frameIdx >= currentAnim.frameCount - 1) {
        const Keyframe& last = currentAnim.frames[currentAnim.frameCount - 1];
        outParams.eyeX       = last.eyeX;
        outParams.eyeY       = last.eyeY;
        outParams.eyeOpen    = last.eyeOpen;
        outParams.mouthCurve = last.mouthCurve;
        outParams.mouthOpen  = last.mouthOpen;
        outParams.bounce     = last.bounce;
        outParams.shake      = last.shake;
        outParams.browAngle  = last.browAngle;
        outParams.browY      = last.browY;
        return true;
    }

    const Keyframe& a = currentAnim.frames[frameIdx];
    const Keyframe& b = currentAnim.frames[frameIdx + 1];

    uint16_t segDuration = b.time_ms - a.time_ms;
    float t = (segDuration > 0) ? (float)(elapsedMs - a.time_ms) / (float)segDuration : 1.0f;
    t = constrain(t, 0.0f, 1.0f);

    interpolateKeyframes(a, b, t, outParams);
    return true;
}

} // namespace AnimPlayer
