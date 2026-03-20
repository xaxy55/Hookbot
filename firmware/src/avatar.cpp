#include "avatar.h"
#include "display.h"
#include "config.h"
#include "server.h"

extern "C" bool _bleProv_isAdvertising();

extern String _hookbot_get_ip();

namespace Avatar {

static AvatarState currentState = AvatarState::IDLE;
static AvatarParams current;
static AvatarParams target;
static uint32_t stateTime = 0;  // ms since state entered
static uint32_t totalTime = 0;

// ─── Easing ──────────────────────────────────────────────────────

static float lerp(float a, float b, float t) {
    return a + (b - a) * t;
}

static float smoothStep(float a, float b, float speed, float dt) {
    float t = 1.0f - expf(-speed * dt / 1000.0f);
    return lerp(a, b, t);
}

// ─── Target param generators per state ──────────────────────────
// CEO "Destroyer of Worlds" personality:
//   IDLE     = confident, scheming smirk, stern brows
//   THINKING = impatient, plotting world domination
//   WAITING  = displeased, tapping foot energy
//   SUCCESS  = maniacal triumph
//   TASKCHECK = approving nod, still intimidating
//   ERROR    = FURIOUS, heads will roll

static void setIdleTarget() {
    target.eyeX = 0;
    target.eyeY = 0;
    target.eyeOpen = 0.95f;      // Relaxed, open
    target.mouthCurve = 0.1f;    // Hint of a smirk
    target.mouthOpen = 0.0f;
    target.bounce = 0;
    target.shake = 0;
    target.browAngle = -0.15f;   // Mostly neutral, relaxed confidence
    target.browY = 0;
}

static void setThinkingTarget() {
    target.eyeOpen = 0.7f;       // Narrowed - impatient
    target.mouthCurve = -0.2f;   // Slight frown - displeased at having to think
    target.mouthOpen = 0.0f;
    target.bounce = 0;
    target.shake = 0;
    target.browAngle = -0.7f;    // Angry brows - plotting
    target.browY = -1.0f;        // Brows pushed down
}

static void setWaitingTarget() {
    // Base waiting - will be overridden by escalation in updateAnimations
    target.eyeX = 0;
    target.eyeY = 0;
    target.eyeOpen = 0.85f;
    target.mouthCurve = -0.2f;
    target.mouthOpen = 0.0f;
    target.bounce = 0;
    target.shake = 0;
    target.browAngle = -0.4f;
    target.browY = 0;
}

static void setSuccessTarget() {
    target.eyeX = 0;
    target.eyeY = 0;
    target.eyeOpen = 0.7f;       // Happy squint
    target.mouthCurve = 1.0f;    // Big grin
    target.mouthOpen = 0.3f;     // Laughing
    target.bounce = 0;
    target.shake = 0;
    target.browAngle = -0.1f;    // Relaxed happy brows
    target.browY = 0.5f;         // Raised - delighted
}

static void setTaskcheckTarget() {
    target.eyeX = 0;
    target.eyeY = 0;
    target.eyeOpen = 0.8f;       // Focused
    target.mouthCurve = 0.3f;    // Pleased smirk
    target.mouthOpen = 0.0f;
    target.bounce = 0;
    target.shake = 0;
    target.browAngle = -0.1f;    // Neutral, attentive
    target.browY = 0.3f;         // One brow slightly raised
}

static void setErrorTarget() {
    target.eyeX = 0;
    target.eyeY = 0;
    target.eyeOpen = 1.2f;       // Wide with RAGE
    target.mouthCurve = -1.0f;   // Maximum frown
    target.mouthOpen = 0.4f;     // Yelling
    target.bounce = 0;
    target.shake = 0;
    target.browAngle = -1.0f;    // Maximum angry V brows
    target.browY = -2.0f;        // Slammed down
}

// ─── Animation updates ──────────────────────────────────────────

static void updateAnimations(uint32_t deltaMs) {
    float dt = (float)deltaMs;
    float phase = (float)stateTime / 1000.0f;

    // Blink logic (all states) - CEOs blink less, they stare
    static uint32_t nextBlink = 0;
    static uint32_t blinkStart = 0;
    static bool blinking = false;

    if (!blinking && stateTime > nextBlink) {
        blinking = true;
        blinkStart = stateTime;
        nextBlink = stateTime + 4000 + (esp_random() % 3000);  // Blinks less often
    }
    if (blinking) {
        uint32_t blinkAge = stateTime - blinkStart;
        if (blinkAge < 60) {       // Faster blink - decisive
            target.eyeOpen = 0.0f;
        } else if (blinkAge < 120) {
            blinking = false;
        }
    }

    switch (currentState) {
        case AvatarState::IDLE: {
            if (!blinking) setIdleTarget();
            float secs = (float)stateTime / 1000.0f;

            if (secs < 30.0f) {
                // Phase 1: Content — calm breathing, occasional curious glance
                float breath = sinf(phase * 0.5f) * 0.8f;
                target.bounce = breath;
                // Occasional glance around
                float glance = sinf(phase * 0.25f);
                if (glance > 0.85f) {
                    target.eyeX = sinf(phase * 1.2f) * 0.4f;
                }
                // Subtle smile that comes and goes
                target.mouthCurve = 0.1f + sinf(phase * 0.15f) * 0.1f;
            } else if (secs < 90.0f) {
                // Phase 2: Getting restless — looking around more, sighing
                float restless = (secs - 30.0f) / 60.0f;  // 0→1 over 60s
                float breath = sinf(phase * 0.4f) * (1.0f + restless * 0.5f);
                target.bounce = breath;
                // More frequent eye movement
                target.eyeX = sinf(phase * 0.8f) * (0.3f + restless * 0.5f);
                target.eyeY = sinf(phase * 0.5f) * 0.2f;
                // Losing the smirk
                target.mouthCurve = 0.1f - restless * 0.15f;
                // Eyebrows slowly raising (questioning, "is that it?")
                target.browAngle = -0.15f + restless * 0.2f;
                // Occasional sigh: deep bounce + mouth open
                float sighCycle = fmodf(phase * 0.08f, 1.0f);
                if (sighCycle > 0.85f && sighCycle < 0.95f) {
                    target.bounce = -1.5f;
                    target.mouthOpen = 0.15f;
                    target.eyeOpen = 0.7f;
                }
            } else if (secs < 180.0f) {
                // Phase 3: Bored — droopy eyes, yawning, fidgeting
                float boredom = min(1.0f, (secs - 90.0f) / 90.0f);
                target.eyeOpen = 0.7f - boredom * 0.2f;  // Drooping eyelids
                target.browAngle = 0.1f + boredom * 0.15f;  // Raised, disinterested
                target.browY = 0.3f;
                target.mouthCurve = -0.05f;  // Slightly pouty
                // Lazy looking around
                target.eyeX = sinf(phase * 0.3f) * 0.6f;
                target.eyeY = sinf(phase * 0.2f) * 0.3f;
                // Slow breathing
                target.bounce = sinf(phase * 0.3f) * 1.5f;
                // Yawn every ~20 seconds
                float yawnCycle = fmodf(secs / 20.0f, 1.0f);
                if (yawnCycle > 0.8f) {
                    float yawnT = (yawnCycle - 0.8f) / 0.2f;  // 0→1
                    if (yawnT < 0.5f) {
                        // Opening yawn
                        target.mouthOpen = yawnT * 0.8f;
                        target.mouthCurve = 0.2f;
                        target.eyeOpen = 0.3f;
                        target.browY = 0.5f;
                    } else {
                        // Closing yawn
                        float close = (yawnT - 0.5f) * 2.0f;
                        target.mouthOpen = 0.4f * (1.0f - close);
                        target.eyeOpen = 0.3f + close * 0.3f;
                    }
                }
            } else {
                // Phase 4: Fell asleep — Zzz
                float sleepDepth = min(1.0f, (secs - 180.0f) / 15.0f);
                target.eyeOpen = 0.1f * (1.0f - sleepDepth);
                target.mouthCurve = 0.0f;
                target.mouthOpen = 0.0f;
                target.browAngle = 0.1f;
                target.browY = 0.5f;
                target.eyeX = 0;
                target.eyeY = 0;
                // Snoring rhythm
                float snore = sinf(phase * 0.3f);
                target.bounce = snore * 2.0f;
                if (snore > 0.7f) {
                    target.mouthOpen = 0.15f;
                }
            }
            break;
        }

        case AvatarState::THINKING: {
            if (!blinking) setThinkingTarget();
            // Eyes darting - scheming rapidly
            target.eyeX = sinf(phase * 3.5f) * 0.9f;
            target.eyeY = sinf(phase * 1.8f) * 0.4f;
            // Impatient micro-shake
            target.shake = sinf(phase * 8.0f) * 0.3f;
            break;
        }

        case AvatarState::WAITING: {
            if (!blinking) setWaitingTarget();

            // Escalating rage: 0-3s mild, 3-6s annoyed, 6-10s angry, 10s+ FURIOUS
            float rage = min(1.0f, (float)stateTime / 10000.0f);  // 0 to 1 over 10s
            float secs = (float)stateTime / 1000.0f;

            if (secs < 3.0f) {
                // Phase 1: Impatient tapping, slight frown
                float tap = sinf(phase * 3.0f);
                if (tap > 0.7f) target.bounce = -1.5f;
                target.mouthCurve = -0.2f;
                target.browAngle = -0.4f;
                // Looking around impatiently
                target.eyeX = sinf(phase * 1.0f) * 0.4f;
            } else if (secs < 6.0f) {
                // Phase 2: Annoyed - faster tapping, deeper frown
                float tap = sinf(phase * 5.0f);
                if (tap > 0.5f) target.bounce = -2.0f;
                target.mouthCurve = -0.5f;
                target.browAngle = -0.6f;
                target.browY = -1.0f;
                target.eyeOpen = 0.75f;
                // Glaring left and right
                target.eyeX = sinf(phase * 2.0f) * 0.7f;
            } else if (secs < 10.0f) {
                // Phase 3: Angry - shaking, yelling
                target.shake = sinf(phase * 6.0f) * 2.0f;
                float stomp = sinf(phase * 7.0f);
                if (stomp > 0.3f) target.bounce = -3.0f * stomp;
                target.mouthCurve = -0.8f;
                target.mouthOpen = 0.2f;
                target.browAngle = -0.8f;
                target.browY = -1.5f;
                target.eyeOpen = 0.65f;  // Seething squint
                target.eyeX = sinf(phase * 3.0f) * 0.5f;
            } else {
                // Phase 4: FULL TANTRUM - violent shaking, wide eyes, screaming
                target.shake = sinf(phase * 12.0f) * 4.0f;
                target.bounce = sinf(phase * 9.0f) * 4.0f;
                target.mouthCurve = -1.0f;
                target.mouthOpen = 0.5f;  // SCREAMING
                target.browAngle = -1.0f;
                target.browY = -2.0f;
                // Alternating between wide rage eyes and furious squint
                float eyePulse = sinf(phase * 4.0f);
                target.eyeOpen = eyePulse > 0 ? 1.2f : 0.5f;
                target.eyeX = sinf(phase * 5.0f) * 1.0f;
                target.eyeY = sinf(phase * 3.5f) * 0.5f;
            }
            break;
        }

        case AvatarState::SUCCESS: {
            if (!blinking) setSuccessTarget();
            // Maniacal victory bounce
            if (stateTime < 1000) {
                float t = (float)stateTime / 1000.0f;
                target.bounce = sinf(t * PI * 5) * (1.0f - t) * 5.0f;
            } else {
                target.bounce = 0;
                // Settle into evil satisfied look
                target.mouthOpen = 0.1f;
            }
            break;
        }

        case AvatarState::TASKCHECK: {
            if (!blinking) setTaskcheckTarget();
            // Single authoritative nod
            if (stateTime < 400) {
                float t = (float)stateTime / 400.0f;
                target.bounce = sinf(t * PI) * 3.0f;
            } else {
                target.bounce = 0;
            }
            break;
        }

        case AvatarState::ERROR: {
            if (!blinking) setErrorTarget();
            // Violent shaking rage
            if (stateTime < 800) {
                float t = (float)stateTime / 800.0f;
                target.shake = sinf(t * PI * 10) * (1.0f - t * 0.5f) * 5.0f;
            } else {
                // Settle into seething fury
                target.shake = sinf(phase * 6.0f) * 0.5f;
                target.eyeOpen = 0.6f;  // Narrow to seething
                target.mouthOpen = 0.0f;
                target.mouthCurve = -0.9f;
            }
            break;
        }
    }

    // Smooth interpolation toward target
    float speed = 8.0f;
    current.eyeX       = smoothStep(current.eyeX,       target.eyeX,       speed, dt);
    current.eyeY       = smoothStep(current.eyeY,       target.eyeY,       speed, dt);
    current.eyeOpen    = smoothStep(current.eyeOpen,     target.eyeOpen,    12.0f, dt);
    current.mouthCurve = smoothStep(current.mouthCurve,  target.mouthCurve, speed, dt);
    current.mouthOpen  = smoothStep(current.mouthOpen,   target.mouthOpen,  speed, dt);
    current.bounce     = smoothStep(current.bounce,      target.bounce,     10.0f, dt);
    current.shake      = smoothStep(current.shake,       target.shake,      12.0f, dt);
    current.browAngle  = smoothStep(current.browAngle,   target.browAngle,  speed, dt);
    current.browY      = smoothStep(current.browY,       target.browY,      speed, dt);
}

// ─── Drawing ────────────────────────────────────────────────────

static void drawTaskList(DisplayCanvas* d) {
    const TaskList& tasks = HookbotServer::getTasks();
    if (tasks.count == 0) return;

    int16_t startX = 72;  // Right side of screen
    int16_t startY = 2;
    int16_t lineH = 9;    // 8px font + 1px gap
    int16_t maxVisible = min((uint8_t)6, tasks.count);

    // Scroll window: keep active item visible
    int16_t scrollOffset = 0;
    if (tasks.activeIndex >= maxVisible) {
        scrollOffset = tasks.activeIndex - maxVisible + 1;
    }

    d->setTextSize(1);  // 6x8 font

    for (uint8_t i = 0; i < maxVisible; i++) {
        uint8_t idx = i + scrollOffset;
        if (idx >= tasks.count) break;

        int16_t y = startY + i * lineH;
        const TaskItem& item = tasks.items[idx];

        // Checkbox
        if (item.status == 2) {
            // Done: filled box with checkmark
            d->fillRect(startX, y + 1, 7, 7, COLOR_WHITE);
            d->drawPixel(startX + 2, y + 5, COLOR_BLACK);
            d->drawPixel(startX + 3, y + 6, COLOR_BLACK);
            d->drawPixel(startX + 4, y + 5, COLOR_BLACK);
            d->drawPixel(startX + 5, y + 4, COLOR_BLACK);
        } else if (item.status == 3) {
            // Failed: X box
            d->drawRect(startX, y + 1, 7, 7, COLOR_WHITE);
            d->drawLine(startX + 1, y + 2, startX + 5, y + 6, COLOR_WHITE);
            d->drawLine(startX + 5, y + 2, startX + 1, y + 6, COLOR_WHITE);
        } else if (item.status == 1 || idx == tasks.activeIndex) {
            // Active: animated dot
            uint32_t t = millis();
            bool blink = (t % 600) < 400;
            if (blink) {
                d->fillCircle(startX + 3, y + 4, 3, COLOR_WHITE);
            } else {
                d->drawCircle(startX + 3, y + 4, 3, COLOR_WHITE);
            }
        } else {
            // Pending: empty box
            d->drawRect(startX, y + 1, 7, 7, COLOR_WHITE);
        }

        // Label (truncated to fit)
        d->setTextColor(COLOR_WHITE);
        d->setCursor(startX + 10, y + 1);

        // Calculate max chars that fit (screen width - label start) / 6px per char
        int16_t maxChars = (SCREEN_WIDTH - startX - 10) / 6;
        char truncated[MAX_TASK_LEN];
        strncpy(truncated, item.label, maxChars);
        truncated[maxChars] = '\0';
        d->print(truncated);

        // Strikethrough for done items
        if (item.status == 2) {
            int16_t textW = min((int16_t)(strlen(truncated) * 6), (int16_t)(SCREEN_WIDTH - startX - 10));
            d->drawFastHLine(startX + 10, y + 4, textW, COLOR_WHITE);
        }
    }

    // Scroll indicator if there are more items
    if (tasks.count > maxVisible) {
        int16_t indicatorY = startY + maxVisible * lineH;
        int16_t totalH = maxVisible * lineH;
        int16_t thumbH = max((int16_t)4, (int16_t)(totalH * maxVisible / tasks.count));
        int16_t thumbY = startY + (int16_t)((float)scrollOffset / (tasks.count - maxVisible) * (totalH - thumbH));
        d->drawFastVLine(SCREEN_WIDTH - 2, startY, totalH, COLOR_WHITE);
        d->fillRect(SCREEN_WIDTH - 3, thumbY, 3, thumbH, COLOR_WHITE);
    }
}

static void drawFace(DisplayCanvas* d) {
    // Shift face left when tasks are showing
    const TaskList& tasks = HookbotServer::getTasks();
    bool hasTasks = tasks.count > 0;
    int16_t faceOffset = hasTasks ? -20 : 0;

    int16_t cx = Display::centerX() + (int16_t)current.shake + faceOffset;
    int16_t cy = Display::centerY() + (int16_t)current.bounce;

    const RuntimeConfig& cfg = HookbotServer::getConfig();

    // ─── Top Hat (CEO standard issue) ─────────
    if (cfg.topHat) {
        int16_t hatBrimY = cy - 22;
        int16_t hatTopY  = cy - 38;
        int16_t hatW     = 14;   // Half-width of hat body
        int16_t brimW    = 20;   // Half-width of brim

        // Brim
        d->drawFastHLine(cx - brimW, hatBrimY, brimW * 2 + 1, COLOR_WHITE);
        d->drawFastHLine(cx - brimW, hatBrimY + 1, brimW * 2 + 1, COLOR_WHITE);

        // Hat body (tall rectangle)
        d->drawFastVLine(cx - hatW, hatTopY, hatBrimY - hatTopY, COLOR_WHITE);
        d->drawFastVLine(cx + hatW, hatTopY, hatBrimY - hatTopY, COLOR_WHITE);

        // Top
        d->drawFastHLine(cx - hatW, hatTopY, hatW * 2 + 1, COLOR_WHITE);

        // Hat band (decorative stripe near bottom of hat body)
        d->drawFastHLine(cx - hatW + 1, hatBrimY - 4, hatW * 2 - 1, COLOR_WHITE);
        d->drawFastHLine(cx - hatW + 1, hatBrimY - 3, hatW * 2 - 1, COLOR_WHITE);
    }

    // ─── Crown ─────────────────────────────────
    if (cfg.crown) {
        int16_t crownY = cy - 24;
        // Crown base
        d->drawFastHLine(cx - 16, crownY, 33, COLOR_WHITE);
        // Spikes
        d->drawLine(cx - 16, crownY, cx - 16, crownY - 10, COLOR_WHITE);
        d->drawLine(cx - 16, crownY - 10, cx - 10, crownY - 5, COLOR_WHITE);
        d->drawLine(cx - 10, crownY - 5, cx - 4, crownY - 14, COLOR_WHITE);
        d->drawLine(cx - 4, crownY - 14, cx, crownY - 8, COLOR_WHITE);
        d->drawLine(cx, crownY - 8, cx + 4, crownY - 14, COLOR_WHITE);
        d->drawLine(cx + 4, crownY - 14, cx + 10, crownY - 5, COLOR_WHITE);
        d->drawLine(cx + 10, crownY - 5, cx + 16, crownY - 10, COLOR_WHITE);
        d->drawLine(cx + 16, crownY - 10, cx + 16, crownY, COLOR_WHITE);
        // Jewel dots on spike tips
        d->fillCircle(cx, crownY - 8, 1, COLOR_WHITE);
        d->fillCircle(cx - 4, crownY - 14, 1, COLOR_WHITE);
        d->fillCircle(cx + 4, crownY - 14, 1, COLOR_WHITE);
    }

    // ─── Devil Horns ───────────────────────────
    if (cfg.horns) {
        int16_t hornY = cy - 22;
        for (int side = -1; side <= 1; side += 2) {
            int16_t hx = cx + side * 12;
            // Curved horn pointing up and out
            d->drawLine(hx, hornY, hx + side * 4, hornY - 8, COLOR_WHITE);
            d->drawLine(hx + 1, hornY, hx + side * 4 + 1, hornY - 8, COLOR_WHITE);
            d->drawLine(hx + side * 4, hornY - 8, hx + side * 2, hornY - 14, COLOR_WHITE);
            d->drawLine(hx + side * 4 + 1, hornY - 8, hx + side * 2 + 1, hornY - 14, COLOR_WHITE);
        }
    }

    // ─── Halo ──────────────────────────────────
    if (cfg.halo) {
        int16_t haloY = cy - 28;
        // Ellipse approximation - draw two arcs
        d->drawCircle(cx, haloY, 12, COLOR_WHITE);
        // Make it look like an ellipse by clearing top/bottom
        d->drawFastHLine(cx - 10, haloY - 4, 21, COLOR_BLACK);
        d->drawFastHLine(cx - 10, haloY + 4, 21, COLOR_BLACK);
        // Redraw as flat ellipse with horizontal lines
        for (int16_t i = -12; i <= 12; i++) {
            float t = (float)i / 12.0f;
            int16_t dy = (int16_t)(3.0f * sqrtf(1.0f - t * t));
            d->drawPixel(cx + i, haloY - dy, COLOR_WHITE);
            d->drawPixel(cx + i, haloY + dy, COLOR_WHITE);
        }
    }

    // ─── Eyebrows (the source of all authority) ──────
    int16_t eyeSpacing = 18;
    int16_t browBaseY  = cy - 18;

    for (int side = -1; side <= 1; side += 2) {
        int16_t bx = cx + side * eyeSpacing;
        int16_t by = browBaseY + (int16_t)current.browY;

        // Angry V shape: inner end lower, outer end higher
        float angle = current.browAngle;
        int16_t innerY = by + (int16_t)(angle * -3.0f);  // Inner goes down when angry
        int16_t outerY = by + (int16_t)(angle * 3.0f);   // Outer goes up when angry

        int16_t innerX = bx - side * 2;  // Toward center
        int16_t outerX = bx + side * 8;  // Away from center

        // Draw thick brow (2 lines for boldness - CEO brows are THICC)
        d->drawLine(innerX, innerY, outerX, outerY, COLOR_WHITE);
        d->drawLine(innerX, innerY + 1, outerX, outerY + 1, COLOR_WHITE);
    }

    // ─── Eyes ─────────────────────────────────
    int16_t eyeBaseY   = cy - 8;
    int16_t eyeW       = 10;
    int16_t eyeMaxH    = 12;

    float openness = max(0.0f, min(current.eyeOpen, 1.2f));
    int16_t eyeH = (int16_t)(eyeMaxH * openness);
    if (eyeH < 1) eyeH = 1;

    int16_t pupilOffX = (int16_t)(current.eyeX * 3.0f);
    int16_t pupilOffY = (int16_t)(current.eyeY * 2.0f);

    for (int side = -1; side <= 1; side += 2) {
        int16_t ex = cx + side * eyeSpacing;
        int16_t ey = eyeBaseY;

        if (eyeH <= 2) {
            // Closed eye - horizontal line
            d->drawFastHLine(ex - eyeW / 2, ey, eyeW, COLOR_WHITE);
        } else {
            // Open eye - filled rounded rect
            int16_t r = min((int16_t)(eyeW / 2), (int16_t)(eyeH / 2));
            d->fillRoundRect(ex - eyeW / 2, ey - eyeH / 2, eyeW, eyeH, r, COLOR_WHITE);

            // Pupil (dark dot inside white eye)
            if (eyeH > 5) {
                int16_t pupilR = 2;
                int16_t px = ex + pupilOffX;
                int16_t py = ey + pupilOffY;
                d->fillCircle(px, py, pupilR, COLOR_BLACK);
            }
        }
    }

    // ─── Glasses ────────────────────────────────
    if (cfg.glasses) {
        for (int side = -1; side <= 1; side += 2) {
            int16_t ex = cx + side * eyeSpacing;
            // Lens frame
            d->drawRoundRect(ex - eyeW / 2 - 2, eyeBaseY - 7, eyeW + 4, 14, 3, COLOR_WHITE);
        }
        // Bridge
        d->drawLine(cx - eyeSpacing + eyeW / 2 + 2, eyeBaseY, cx + eyeSpacing - eyeW / 2 - 2, eyeBaseY, COLOR_WHITE);
        // Arms
        d->drawFastHLine(cx - eyeSpacing - eyeW / 2 - 2, eyeBaseY - 3, -6, COLOR_WHITE);
        d->drawFastHLine(cx + eyeSpacing + eyeW / 2 + 2, eyeBaseY - 3, 6, COLOR_WHITE);
    }

    // ─── Monocle ───────────────────────────────
    if (cfg.monocle) {
        int16_t ex = cx + eyeSpacing;  // Right eye
        d->drawCircle(ex, eyeBaseY, eyeW / 2 + 3, COLOR_WHITE);
        d->drawCircle(ex, eyeBaseY, eyeW / 2 + 4, COLOR_WHITE);
        // Chain hanging down
        int16_t chainX = ex + eyeW / 2 + 2;
        int16_t chainY = eyeBaseY + eyeW / 2 + 2;
        for (int i = 0; i < 12; i++) {
            int16_t py = chainY + i * 2;
            int16_t px = chainX + (int16_t)(sinf((float)i * 0.8f) * 2.0f);
            if (py < SCREEN_HEIGHT) {
                d->drawPixel(px, py, COLOR_WHITE);
            }
        }
    }

    // ─── Mouth ───────────────────────────────
    int16_t mouthY = cy + 12;
    int16_t mouthW = 16;

    if (current.mouthCurve > 0.1f) {
        // Evil grin
        int16_t curveH = (int16_t)(current.mouthCurve * 6.0f);
        int16_t openH  = (int16_t)(current.mouthOpen * 6.0f);

        if (openH > 1) {
            // Open mouth - maniacal laugh
            d->fillRoundRect(cx - mouthW / 2, mouthY, mouthW, openH + 2, 2, COLOR_WHITE);
        }
        // Grin curve
        for (int i = -mouthW / 2; i <= mouthW / 2; i++) {
            float t = (float)i / (float)(mouthW / 2);
            int16_t dy = (int16_t)(curveH * (1.0f - t * t));
            d->drawPixel(cx + i, mouthY + dy, COLOR_WHITE);
        }
    } else if (current.mouthCurve < -0.1f) {
        // Frown of displeasure
        int16_t curveH = (int16_t)(-current.mouthCurve * 5.0f);
        int16_t openH  = (int16_t)(current.mouthOpen * 5.0f);

        if (openH > 1) {
            // Open frown - yelling at subordinates
            d->fillRoundRect(cx - mouthW / 2, mouthY - 2, mouthW, openH + 2, 2, COLOR_WHITE);
        }
        for (int i = -mouthW / 2; i <= mouthW / 2; i++) {
            float t = (float)i / (float)(mouthW / 2);
            int16_t dy = (int16_t)(curveH * (1.0f - t * t));
            d->drawPixel(cx + i, mouthY - dy, COLOR_WHITE);
        }
    } else {
        // Neutral: straight line (unimpressed)
        d->drawFastHLine(cx - mouthW / 3, mouthY, mouthW * 2 / 3, COLOR_WHITE);
    }

    // ─── Cigar (CEO power move) ───────────────
    if (cfg.cigar) {
        int16_t cigarX = cx + mouthW / 2 + 1;  // Sticks out right side of mouth
        int16_t cigarY = mouthY + 1;

        // Cigar body - angled slightly upward
        d->drawLine(cigarX, cigarY, cigarX + 10, cigarY - 3, COLOR_WHITE);
        d->drawLine(cigarX, cigarY + 1, cigarX + 10, cigarY - 2, COLOR_WHITE);
        d->drawLine(cigarX, cigarY + 2, cigarX + 10, cigarY - 1, COLOR_WHITE);

        // Ember tip - flickering glow
        float flicker = sinf((float)totalTime / 150.0f);
        if (flicker > -0.3f) {
            d->drawPixel(cigarX + 10, cigarY - 3, COLOR_WHITE);
            d->drawPixel(cigarX + 10, cigarY - 2, COLOR_WHITE);
            d->drawPixel(cigarX + 11, cigarY - 3, COLOR_WHITE);
        }
        if (flicker > 0.3f) {
            d->drawPixel(cigarX + 11, cigarY - 2, COLOR_WHITE);
        }

        // Smoke particles - rising and drifting
        float smokePhase = (float)totalTime / 1000.0f;
        int16_t smokeSrcX = cigarX + 11;
        int16_t smokeSrcY = cigarY - 4;

        for (int i = 0; i < 5; i++) {
            // Each particle has its own phase offset and drift
            float pLife = fmodf(smokePhase * 1.2f + i * 0.7f, 3.0f);  // 0-3 cycle
            if (pLife > 2.5f) continue;  // Gap between puffs

            float rise = pLife * 5.0f;     // Float upward
            float drift = sinf(pLife * 2.0f + i * 1.5f) * (2.0f + pLife);  // Wavy drift

            int16_t sx = smokeSrcX + (int16_t)drift;
            int16_t sy = smokeSrcY - (int16_t)rise;

            // Smoke gets wispier as it rises
            if (sy >= 0 && sy < SCREEN_HEIGHT && sx >= 0 && sx < SCREEN_WIDTH) {
                d->drawPixel(sx, sy, COLOR_WHITE);
                if (pLife < 1.5f) {
                    // Thicker smoke near the cigar
                    d->drawPixel(sx + 1, sy, COLOR_WHITE);
                }
                if (pLife < 0.8f) {
                    d->drawPixel(sx, sy - 1, COLOR_WHITE);
                }
            }
        }
    }

    // ─── Bow Tie ────────────────────────────────
    if (cfg.bowtie) {
        int16_t tieY = cy + 20;
        // Left triangle
        d->drawLine(cx, tieY, cx - 8, tieY - 4, COLOR_WHITE);
        d->drawLine(cx, tieY, cx - 8, tieY + 4, COLOR_WHITE);
        d->drawLine(cx - 8, tieY - 4, cx - 8, tieY + 4, COLOR_WHITE);
        // Right triangle
        d->drawLine(cx, tieY, cx + 8, tieY - 4, COLOR_WHITE);
        d->drawLine(cx, tieY, cx + 8, tieY + 4, COLOR_WHITE);
        d->drawLine(cx + 8, tieY - 4, cx + 8, tieY + 4, COLOR_WHITE);
        // Center knot
        d->fillCircle(cx, tieY, 2, COLOR_WHITE);
    }

    // ─── Project name (top-left during non-idle states) ──────
    if (currentState != AvatarState::IDLE) {
        const ProjectInfo& proj = HookbotServer::getProject();
        if (strlen(proj.name) > 0 && (millis() - proj.lastUpdatedAt) < 600000) {
            d->setTextSize(1);
            d->setTextColor(COLOR_WHITE);
            d->setCursor(2, 2);
            char truncName[14];
            strncpy(truncName, proj.name, 13);
            truncName[13] = '\0';
            d->print(truncName);
        }
    }

    // ─── Tool display (bottom of screen) ──────
    if ((currentState == AvatarState::THINKING || currentState == AvatarState::TASKCHECK)
        && strlen(HookbotServer::getCurrentTool().name) > 0) {
        const ToolInfo& tool = HookbotServer::getCurrentTool();

        int16_t toolY = SCREEN_HEIGHT - 8;  // Bottom area
        int16_t iconX = 2;
        int16_t textX = 14;

        // Draw tool-specific icon (8x8 area)
        if (strcmp(tool.name, "Read") == 0) {
            // Eye icon - reading
            d->drawCircle(iconX + 4, toolY + 3, 3, COLOR_WHITE);
            d->fillCircle(iconX + 4, toolY + 3, 1, COLOR_WHITE);
            d->drawLine(iconX, toolY + 3, iconX + 1, toolY + 3, COLOR_WHITE);
            d->drawLine(iconX + 7, toolY + 3, iconX + 8, toolY + 3, COLOR_WHITE);
        } else if (strcmp(tool.name, "Write") == 0 || strcmp(tool.name, "Edit") == 0) {
            // Pencil icon
            d->drawLine(iconX + 1, toolY + 6, iconX + 7, toolY, COLOR_WHITE);
            d->drawLine(iconX + 2, toolY + 6, iconX + 8, toolY, COLOR_WHITE);
            d->drawPixel(iconX, toolY + 7, COLOR_WHITE);
        } else if (strcmp(tool.name, "Bash") == 0) {
            // Terminal icon: >_
            d->drawLine(iconX, toolY + 1, iconX + 3, toolY + 3, COLOR_WHITE);
            d->drawLine(iconX, toolY + 5, iconX + 3, toolY + 3, COLOR_WHITE);
            d->drawFastHLine(iconX + 4, toolY + 6, 4, COLOR_WHITE);
        } else if (strcmp(tool.name, "Grep") == 0 || strcmp(tool.name, "Glob") == 0) {
            // Magnifying glass
            d->drawCircle(iconX + 3, toolY + 3, 3, COLOR_WHITE);
            d->drawLine(iconX + 5, toolY + 5, iconX + 8, toolY + 7, COLOR_WHITE);
        } else if (strcmp(tool.name, "Agent") == 0) {
            // Robot head
            d->drawRect(iconX + 1, toolY + 2, 7, 5, COLOR_WHITE);
            d->drawPixel(iconX + 3, toolY + 4, COLOR_WHITE);
            d->drawPixel(iconX + 5, toolY + 4, COLOR_WHITE);
            d->drawFastHLine(iconX + 2, toolY, 5, COLOR_WHITE);
        } else {
            // Generic gear icon
            d->drawCircle(iconX + 4, toolY + 3, 2, COLOR_WHITE);
            d->drawPixel(iconX + 4, toolY, COLOR_WHITE);
            d->drawPixel(iconX + 4, toolY + 6, COLOR_WHITE);
            d->drawPixel(iconX + 1, toolY + 3, COLOR_WHITE);
            d->drawPixel(iconX + 7, toolY + 3, COLOR_WHITE);
        }

        // Tool name text
        d->setTextSize(1);
        d->setTextColor(COLOR_WHITE);
        d->setCursor(textX, toolY);
        d->print(tool.name);

        // Detail (filename etc) - show after tool name if present
        if (strlen(tool.detail) > 0) {
            int16_t nameLen = strlen(tool.name) * 6;  // 6px per char at size 1
            d->setCursor(textX + nameLen + 3, toolY);
            d->print(tool.detail);
        }

        // Animated progress indicator
        if (currentState == AvatarState::THINKING) {
            float phase = (float)stateTime / 300.0f;
            int16_t barX = 2;
            int16_t barY = SCREEN_HEIGHT - 10;
            int16_t barW = SCREEN_WIDTH - 4;
            // Scanning line effect
            int16_t scanPos = barX + (int16_t)(fmodf(phase, 1.0f) * barW);
            d->drawPixel(scanPos, barY, COLOR_WHITE);
            if (scanPos > barX) d->drawPixel(scanPos - 1, barY, COLOR_WHITE);
            if (scanPos > barX + 1) d->drawPixel(scanPos - 2, barY, COLOR_WHITE);
        }
    } else if (currentState == AvatarState::THINKING) {
        // Fallback: plotting dots when no tool info
        float phase = (float)stateTime / 400.0f;
        for (int i = 0; i < 3; i++) {
            int16_t dotX = cx - 6 + i * 6;
            int16_t dotY = cy + 24;
            float anim = sinf(phase * PI + i * 1.0f);
            if (anim > 0.3f) {
                d->fillCircle(dotX, dotY - (int16_t)(anim * 2), 1, COLOR_WHITE);
            }
        }
    }

    // ─── Taskcheck: authoritative checkmark ──
    if (currentState == AvatarState::TASKCHECK && stateTime < 800) {
        float progress = min(1.0f, (float)stateTime / 600.0f);
        int16_t checkX = cx - 6;
        int16_t checkY = cy + 24;

        if (progress < 0.4f) {
            float t = progress / 0.4f;
            int16_t endX = checkX + (int16_t)(4 * t);
            int16_t endY = checkY + (int16_t)(4 * t);
            d->drawLine(checkX, checkY, endX, endY, COLOR_WHITE);
        } else {
            float t = (progress - 0.4f) / 0.6f;
            d->drawLine(checkX, checkY, checkX + 4, checkY + 4, COLOR_WHITE);
            int16_t endX = checkX + 4 + (int16_t)(8 * t);
            int16_t endY = checkY + 4 - (int16_t)(8 * t);
            d->drawLine(checkX + 4, checkY + 4, endX, endY, COLOR_WHITE);
        }
    }

    // ─── Error: skull crossbones (X marks) ───
    if (currentState == AvatarState::ERROR && stateTime > 300) {
        // Double X - worlds are being destroyed
        for (int s = -1; s <= 1; s += 2) {
            int16_t xc = cx + s * 8;
            int16_t yc = cy + 24;
            d->drawLine(xc - 3, yc - 3, xc + 3, yc + 3, COLOR_WHITE);
            d->drawLine(xc + 3, yc - 3, xc - 3, yc + 3, COLOR_WHITE);
        }
    }

    // ─── Bored: thought bubble with "..." ─────
    if (currentState == AvatarState::IDLE && stateTime > 90000 && stateTime < 180000) {
        // Floating "..." thought bubble
        float boredPhase = (float)totalTime / 600.0f;
        int16_t bubbleX = cx + 24;
        int16_t bubbleY = cy - 18 + (int16_t)(sinf(boredPhase) * 2.0f);

        // Three dots with staggered animation
        for (int i = 0; i < 3; i++) {
            float dotPhase = sinf(boredPhase * 2.0f + i * 1.0f);
            int16_t dotY = bubbleY + (dotPhase > 0.5f ? -1 : 0);
            d->fillCircle(bubbleX + i * 4, dotY, 1, COLOR_WHITE);
        }
    }

    // ─── Sleeping: Zzz ────────────────────────
    if (currentState == AvatarState::IDLE && stateTime > 180000) {
        float zPhase = (float)totalTime / 1500.0f;
        // Three Z's floating up at different sizes
        for (int i = 0; i < 3; i++) {
            float zLife = fmodf(zPhase + i * 1.2f, 3.5f);
            if (zLife > 3.0f) continue;
            float rise = zLife * 6.0f;
            float drift = zLife * 3.0f;
            int16_t zx = cx + 22 + (int16_t)drift;
            int16_t zy = cy - 10 - (int16_t)rise;
            int16_t zSize = 2 + i;  // Gets bigger
            if (zy >= 0 && zx < SCREEN_WIDTH - zSize) {
                // Draw a Z
                d->drawFastHLine(zx, zy, zSize, COLOR_WHITE);
                d->drawLine(zx + zSize - 1, zy, zx, zy + zSize - 1, COLOR_WHITE);
                d->drawFastHLine(zx, zy + zSize - 1, zSize, COLOR_WHITE);
            }
        }
    }

    // ─── Waiting: exclamation marks (PAY ATTENTION!) ──
    if (currentState == AvatarState::WAITING && stateTime > 3000) {
        float rage = min(1.0f, (float)stateTime / 10000.0f);
        float pulse = sinf((float)totalTime / 200.0f);
        // Bouncing exclamation marks on either side - more appear with rage
        int marks = 1 + (int)(rage * 2);  // 1 to 3 marks per side
        for (int m = 0; m < marks; m++) {
            float mPhase = pulse + m * 0.8f;
            int16_t mBounce = (int16_t)(sinf(mPhase * 3.0f) * 2.0f);
            for (int side = -1; side <= 1; side += 2) {
                int16_t mx = cx + side * (30 + m * 7);
                int16_t my = cy - 8 + mBounce;
                if (mx > 2 && mx < SCREEN_WIDTH - 2) {
                    // ! mark: line + dot
                    d->drawFastVLine(mx, my - 4, 6, COLOR_WHITE);
                    d->drawPixel(mx, my + 4, COLOR_WHITE);
                }
            }
        }
    }
}

// ─── Public API ─────────────────────────────────────────────────

void init() {
    setIdleTarget();
    current = target;
    Serial.println("[Avatar] The Destroyer awakens");
}

void setState(AvatarState state) {
    if (state == currentState) return;
    currentState = state;
    stateTime = 0;

    switch (state) {
        case AvatarState::IDLE:      setIdleTarget(); break;
        case AvatarState::THINKING:  setThinkingTarget(); break;
        case AvatarState::WAITING:   setWaitingTarget(); break;
        case AvatarState::SUCCESS:   setSuccessTarget(); break;
        case AvatarState::TASKCHECK: setTaskcheckTarget(); break;
        case AvatarState::ERROR:     setErrorTarget(); break;
    }

    Serial.printf("[Avatar] State -> %d\n", (int)state);
}

AvatarState getState() {
    return currentState;
}

void update(uint32_t deltaMs) {
    stateTime += deltaMs;
    totalTime += deltaMs;
    updateAnimations(deltaMs);
}

void overrideParams(const AvatarParams& params) {
    current = params;
}

static void drawNotifications(DisplayCanvas* d) {
    NotificationData* notifs = HookbotServer::getNotifications();
    int count = HookbotServer::getNotificationCount();

    int16_t badgeX = SCREEN_WIDTH - 2;  // Right-aligned
    int16_t badgeY = 1;

    for (int i = 0; i < count; i++) {
        if (!notifs[i].active || notifs[i].unread <= 0) continue;

        // Calculate badge width based on number of digits
        char countStr[8];
        snprintf(countStr, sizeof(countStr), "%d", notifs[i].unread);
        int16_t textW = strlen(countStr) * 6;  // 6px per char at size 1
        int16_t pillW = textW + 8;  // padding
        int16_t pillH = 10;
        int16_t pillX = badgeX - pillW;

        // Draw source icon (tiny, left of badge)
        int16_t iconX = pillX - 10;

        if (strcmp(notifs[i].source, "teams") == 0) {
            // Teams icon: "T" in a box
            d->drawRect(iconX, badgeY, 8, 8, COLOR_WHITE);
            d->setCursor(iconX + 2, badgeY + 1);
            d->setTextSize(1);
            d->setTextColor(COLOR_WHITE);
            d->print("T");
        } else if (strcmp(notifs[i].source, "slack") == 0) {
            // Slack icon: #
            d->setCursor(iconX, badgeY);
            d->setTextSize(1);
            d->setTextColor(COLOR_WHITE);
            d->print("#");
        } else {
            // Generic bell
            d->drawCircle(iconX + 3, badgeY + 3, 3, COLOR_WHITE);
            d->drawPixel(iconX + 3, badgeY + 7, COLOR_WHITE);
        }

        // Notification badge pill (filled rounded rect)
        d->fillRoundRect(pillX, badgeY, pillW, pillH, 4, COLOR_WHITE);

        // Unread count (black text on white pill)
        d->setTextSize(1);
        d->setTextColor(COLOR_BLACK);
        d->setCursor(pillX + 4, badgeY + 1);
        d->print(countStr);

        // Pulsing animation - brief flash effect
        uint32_t t = millis();
        if ((t % 2000) < 200) {
            // Quick pulse: invert the badge
            d->drawRoundRect(pillX - 1, badgeY - 1, pillW + 2, pillH + 2, 5, COLOR_WHITE);
        }

        badgeY += pillH + 3;  // Stack multiple notifications
    }

    // Reset text color
    d->setTextColor(COLOR_WHITE);
}

static void drawIdleInfo(DisplayCanvas* d) {
    // Only show in IDLE state so it doesn't clutter other states
    if (currentState != AvatarState::IDLE) return;

    d->setTextSize(1);
    d->setTextColor(COLOR_WHITE);

    int16_t x = 2;
    int16_t y = 2;
    int16_t line = 0;

    // Line 1: Active project (if set recently — fade after 10 minutes)
    const ProjectInfo& proj = HookbotServer::getProject();
    bool hasProject = strlen(proj.name) > 0
        && (millis() - proj.lastUpdatedAt) < 600000;  // 10 min timeout
    if (hasProject) {
        d->setCursor(x, y + line * 10);
        // Folder icon: small open bracket shape
        d->drawPixel(x, y + line * 10 + 1, COLOR_WHITE);
        d->drawPixel(x, y + line * 10 + 2, COLOR_WHITE);
        d->drawPixel(x, y + line * 10 + 3, COLOR_WHITE);
        d->drawPixel(x, y + line * 10 + 4, COLOR_WHITE);
        d->drawPixel(x, y + line * 10 + 5, COLOR_WHITE);
        d->drawPixel(x + 1, y + line * 10, COLOR_WHITE);
        d->drawPixel(x + 2, y + line * 10, COLOR_WHITE);
        d->drawPixel(x + 1, y + line * 10 + 6, COLOR_WHITE);
        d->drawPixel(x + 2, y + line * 10 + 6, COLOR_WHITE);
        // Project name (truncated to fit left side of screen)
        d->setCursor(x + 5, y + line * 10);
        // Truncate to ~10 chars on 128px OLED (leave room for face)
        char truncName[12];
        strncpy(truncName, proj.name, 11);
        truncName[11] = '\0';
        d->print(truncName);
        line++;
    }

    // IP address (when connected, no mgmt server)
    if (HookbotServer::isConnected()) {
        const RuntimeConfig& cfg = HookbotServer::getConfig();
        if (strlen(cfg.mgmtServer) == 0) {
            String ip = ::_hookbot_get_ip();
            d->setCursor(x, y + line * 10);
            d->print(ip.c_str());
            line++;
        }
    }

    // Firmware version
    d->setCursor(x, y + line * 10);
    d->print("FW v" FIRMWARE_VERSION);
    line++;

    // Uptime
    unsigned long ms = millis();
    unsigned long totalSec = ms / 1000;
    unsigned long minutes = (totalSec / 60) % 60;
    unsigned long hours   = (totalSec / 3600) % 24;
    unsigned long days    = totalSec / 86400;

    char uptimeStr[20];
    if (days > 0) {
        snprintf(uptimeStr, sizeof(uptimeStr), "Up %lud %luh", days, hours);
    } else if (hours > 0) {
        snprintf(uptimeStr, sizeof(uptimeStr), "Up %luh %lum", hours, minutes);
    } else {
        snprintf(uptimeStr, sizeof(uptimeStr), "Up %lum", minutes);
    }
    d->setCursor(x, y + line * 10);
    d->print(uptimeStr);
}

static void drawWifiStatus(DisplayCanvas* d) {
    if (HookbotServer::isConnected()) return;  // Only show when disconnected

    int16_t x = 2;
    int16_t y = 2;

    // WiFi icon: three arcs
    d->drawPixel(x + 4, y + 8, COLOR_WHITE);  // Center dot

    // Inner arc
    d->drawPixel(x + 3, y + 6, COLOR_WHITE);
    d->drawPixel(x + 4, y + 5, COLOR_WHITE);
    d->drawPixel(x + 5, y + 6, COLOR_WHITE);

    // Middle arc
    d->drawPixel(x + 2, y + 4, COLOR_WHITE);
    d->drawPixel(x + 3, y + 3, COLOR_WHITE);
    d->drawPixel(x + 4, y + 2, COLOR_WHITE);
    d->drawPixel(x + 5, y + 3, COLOR_WHITE);
    d->drawPixel(x + 6, y + 4, COLOR_WHITE);

    // Outer arc
    d->drawPixel(x + 1, y + 2, COLOR_WHITE);
    d->drawPixel(x + 2, y + 1, COLOR_WHITE);
    d->drawPixel(x + 3, y + 0, COLOR_WHITE);
    d->drawPixel(x + 4, y + 0, COLOR_WHITE);
    d->drawPixel(x + 5, y + 0, COLOR_WHITE);
    d->drawPixel(x + 6, y + 1, COLOR_WHITE);
    d->drawPixel(x + 7, y + 2, COLOR_WHITE);

    // Strike-through X (no wifi)
    d->drawLine(x + 1, y + 1, x + 7, y + 7, COLOR_WHITE);
    d->drawLine(x + 7, y + 1, x + 1, y + 7, COLOR_WHITE);

    // BLE icon next to WiFi icon when advertising
    if (_bleProv_isAdvertising()) {
        int16_t bx = x + 12;
        int16_t by = y;
        // Bluetooth "B" rune shape
        d->drawFastVLine(bx + 2, by, 9, COLOR_WHITE);
        d->drawPixel(bx + 3, by + 1, COLOR_WHITE);
        d->drawPixel(bx + 4, by + 2, COLOR_WHITE);
        d->drawPixel(bx + 3, by + 3, COLOR_WHITE);
        d->drawPixel(bx + 3, by + 5, COLOR_WHITE);
        d->drawPixel(bx + 4, by + 6, COLOR_WHITE);
        d->drawPixel(bx + 3, by + 7, COLOR_WHITE);
        // Arrow tips
        d->drawPixel(bx, by + 2, COLOR_WHITE);
        d->drawPixel(bx + 1, by + 3, COLOR_WHITE);
        d->drawPixel(bx, by + 6, COLOR_WHITE);
        d->drawPixel(bx + 1, by + 5, COLOR_WHITE);
        // Blink the icon
        if ((millis() % 1000) < 500) {
            d->drawPixel(bx + 5, by + 4, COLOR_WHITE);
        }
    }
}

static void drawXpBar(DisplayCanvas* d) {
    const XpData& xp = HookbotServer::getXpData();
    if (xp.level == 0 && xp.xp == 0) return;  // No data yet

    // Only show on idle/success screen
    if (currentState != AvatarState::IDLE && currentState != AvatarState::SUCCESS) return;

    // Draw at bottom of screen
    int16_t barY = SCREEN_HEIGHT - 9;
    int16_t barX = 2;
    int16_t barW = SCREEN_WIDTH - 4;
    int16_t barH = 4;

    // Level text: "Lv5" on the left
    d->setTextSize(1);
    d->setTextColor(COLOR_WHITE);
    char lvlStr[12];
    snprintf(lvlStr, sizeof(lvlStr), "Lv%d", xp.level);
    d->setCursor(barX, barY - 1);
    d->print(lvlStr);

    // XP bar starts after level text
    int16_t textW = strlen(lvlStr) * 6 + 2;
    int16_t xpBarX = barX + textW;
    int16_t xpBarW = barW - textW;

    // Bar outline
    d->drawRect(xpBarX, barY, xpBarW, barH, COLOR_WHITE);

    // Filled progress
    int16_t fillW = (int16_t)((float)xp.progress / 100.0f * (xpBarW - 2));
    if (fillW > 0) {
        d->fillRect(xpBarX + 1, barY + 1, fillW, barH - 2, COLOR_WHITE);
    }
}

void draw() {
    DisplayCanvas* d = Display::getCanvas();
    Display::clear();
    drawFace(d);
    drawTaskList(d);
    drawNotifications(d);
    drawWifiStatus(d);
    drawIdleInfo(d);
    drawXpBar(d);
}

} // namespace Avatar
