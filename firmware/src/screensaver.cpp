#include "screensaver.h"
#include "display.h"
#include "config.h"

namespace Screensaver {

// ─── Animation types ────────────────────────────────────────────
enum class Anim : uint8_t {
    BOUNCING_SKULL,
    STARFIELD,
    MATRIX_RAIN,
    ORBITING_PARTICLES,
    WAVEFORM,
    NUM_ANIMS
};

static Anim currentAnim = Anim::BOUNCING_SKULL;
static uint32_t animTime = 0;
static uint32_t animSwitchTime = 0;
static const uint32_t ANIM_CYCLE_MS = 30000;  // Switch animation every 30s

// ─── Bouncing Skull ─────────────────────────────────────────────
// Classic DVD-logo style bouncing face

static float skullX, skullY, skullDx, skullDy;

static void initBouncingSkull() {
    skullX = esp_random() % (SCREEN_WIDTH - 20) + 10;
    skullY = esp_random() % (SCREEN_HEIGHT - 16) + 8;
    skullDx = 0.8f + (esp_random() % 100) / 200.0f;
    skullDy = 0.6f + (esp_random() % 100) / 200.0f;
    if (esp_random() % 2) skullDx = -skullDx;
    if (esp_random() % 2) skullDy = -skullDy;
}

static void drawBouncingSkull(DisplayCanvas* d) {
    // Move
    skullX += skullDx;
    skullY += skullDy;

    // Bounce off edges
    if (skullX < 8 || skullX > SCREEN_WIDTH - 8) skullDx = -skullDx;
    if (skullY < 8 || skullY > SCREEN_HEIGHT - 8) skullDy = -skullDy;
    skullX = constrain(skullX, 8, SCREEN_WIDTH - 8);
    skullY = constrain(skullY, 8, SCREEN_HEIGHT - 8);

    int16_t x = (int16_t)skullX;
    int16_t y = (int16_t)skullY;

    // Mini CEO face
    // Eyes
    d->fillRoundRect(x - 6, y - 3, 5, 5, 1, COLOR_WHITE);
    d->fillRoundRect(x + 2, y - 3, 5, 5, 1, COLOR_WHITE);
    d->fillCircle(x - 4, y - 1, 1, COLOR_BLACK);  // Pupils
    d->fillCircle(x + 4, y - 1, 1, COLOR_BLACK);
    // Evil smirk
    for (int i = -4; i <= 4; i++) {
        float t = (float)i / 4.0f;
        int16_t dy = (int16_t)(2.0f * (1.0f - t * t));
        d->drawPixel(x + i, y + 5 + dy, COLOR_WHITE);
    }
    // Brows
    d->drawLine(x - 6, y - 6, x - 2, y - 5, COLOR_WHITE);
    d->drawLine(x + 2, y - 5, x + 6, y - 6, COLOR_WHITE);
}

// ─── Starfield ──────────────────────────────────────────────────
// Flying through stars

#define NUM_STARS 32
static float starX[NUM_STARS], starY[NUM_STARS], starZ[NUM_STARS];

static void initStarfield() {
    for (int i = 0; i < NUM_STARS; i++) {
        starX[i] = (float)(esp_random() % 200) - 100.0f;
        starY[i] = (float)(esp_random() % 200) - 100.0f;
        starZ[i] = (float)(esp_random() % 100) + 1.0f;
    }
}

static void drawStarfield(DisplayCanvas* d) {
    int16_t cx = SCREEN_WIDTH / 2;
    int16_t cy = SCREEN_HEIGHT / 2;

    for (int i = 0; i < NUM_STARS; i++) {
        // Move star closer
        starZ[i] -= 0.8f;
        if (starZ[i] <= 0) {
            starX[i] = (float)(esp_random() % 200) - 100.0f;
            starY[i] = (float)(esp_random() % 200) - 100.0f;
            starZ[i] = 100.0f;
        }

        // Project to 2D
        float px = starX[i] / starZ[i] * 60.0f + cx;
        float py = starY[i] / starZ[i] * 60.0f + cy;
        int16_t sx = (int16_t)px;
        int16_t sy = (int16_t)py;

        if (sx >= 0 && sx < SCREEN_WIDTH && sy >= 0 && sy < SCREEN_HEIGHT) {
            d->drawPixel(sx, sy, COLOR_WHITE);
            // Brighter (bigger) when closer
            if (starZ[i] < 30) {
                d->drawPixel(sx + 1, sy, COLOR_WHITE);
                d->drawPixel(sx, sy + 1, COLOR_WHITE);
            }
            // Trail for very close stars
            if (starZ[i] < 15) {
                float prevPx = starX[i] / (starZ[i] + 2.0f) * 60.0f + cx;
                float prevPy = starY[i] / (starZ[i] + 2.0f) * 60.0f + cy;
                d->drawLine(sx, sy, (int16_t)prevPx, (int16_t)prevPy, COLOR_WHITE);
            }
        }
    }
}

// ─── Matrix Rain ────────────────────────────────────────────────
// Falling characters

#define MATRIX_COLS 16
static float matrixY[MATRIX_COLS];
static float matrixSpeed[MATRIX_COLS];
static uint8_t matrixLen[MATRIX_COLS];

static void initMatrixRain() {
    for (int i = 0; i < MATRIX_COLS; i++) {
        matrixY[i] = -(float)(esp_random() % SCREEN_HEIGHT);
        matrixSpeed[i] = 0.5f + (esp_random() % 100) / 80.0f;
        matrixLen[i] = 3 + esp_random() % 5;
    }
}

static void drawMatrixRain(DisplayCanvas* d) {
    d->setTextSize(1);

    for (int col = 0; col < MATRIX_COLS; col++) {
        matrixY[col] += matrixSpeed[col];
        if (matrixY[col] > SCREEN_HEIGHT + matrixLen[col] * 8) {
            matrixY[col] = -(float)(esp_random() % 20);
            matrixSpeed[col] = 0.5f + (esp_random() % 100) / 80.0f;
            matrixLen[col] = 3 + esp_random() % 5;
        }

        int16_t x = col * 8;
        for (int j = 0; j < matrixLen[col]; j++) {
            int16_t y = (int16_t)matrixY[col] - j * 8;
            if (y >= 0 && y < SCREEN_HEIGHT && x < SCREEN_WIDTH) {
                // Random character - mix of digits and symbols
                char c = '!' + (esp_random() % 90);
                d->setTextColor(COLOR_WHITE);
                d->setCursor(x, y);
                d->print(c);
            }
        }
    }
}

// ─── Orbiting Particles ────────────────────────────────────────
// Particles orbiting a center point in various elliptical paths

#define NUM_PARTICLES 12
static float particleAngle[NUM_PARTICLES];
static float particleRadX[NUM_PARTICLES];
static float particleRadY[NUM_PARTICLES];
static float particleSpeed[NUM_PARTICLES];
static float particlePhase[NUM_PARTICLES];

static void initOrbitingParticles() {
    for (int i = 0; i < NUM_PARTICLES; i++) {
        particleAngle[i] = (float)(esp_random() % 628) / 100.0f;
        particleRadX[i] = 10.0f + (esp_random() % 40);
        particleRadY[i] = 6.0f + (esp_random() % 20);
        particleSpeed[i] = 0.02f + (esp_random() % 100) / 2000.0f;
        particlePhase[i] = (float)i / NUM_PARTICLES * 3.14159f;
    }
}

static void drawOrbitingParticles(DisplayCanvas* d) {
    int16_t cx = SCREEN_WIDTH / 2;
    int16_t cy = SCREEN_HEIGHT / 2;

    // Center dot
    d->fillCircle(cx, cy, 2, COLOR_WHITE);

    for (int i = 0; i < NUM_PARTICLES; i++) {
        particleAngle[i] += particleSpeed[i];

        // Tilted elliptical orbit
        float tilt = particlePhase[i];
        float a = particleAngle[i];
        float rx = particleRadX[i];
        float ry = particleRadY[i];

        float px = cosf(a) * rx;
        float py = sinf(a) * ry;

        // Rotate by tilt
        float x = px * cosf(tilt) - py * sinf(tilt);
        float y = px * sinf(tilt) + py * cosf(tilt);

        int16_t sx = cx + (int16_t)x;
        int16_t sy = cy + (int16_t)y;

        if (sx >= 0 && sx < SCREEN_WIDTH && sy >= 0 && sy < SCREEN_HEIGHT) {
            d->drawPixel(sx, sy, COLOR_WHITE);
            // Larger particles in front (y > 0 in orbit)
            if (sinf(a) > 0) {
                d->drawPixel(sx + 1, sy, COLOR_WHITE);
                d->drawPixel(sx, sy + 1, COLOR_WHITE);
            }
        }

        // Trail - draw previous few positions as fading dots
        for (int t = 1; t <= 3; t++) {
            float ta = a - particleSpeed[i] * t * 5;
            float tpx = cosf(ta) * rx;
            float tpy = sinf(ta) * ry;
            float tx = tpx * cosf(tilt) - tpy * sinf(tilt);
            float ty = tpx * sinf(tilt) + tpy * cosf(tilt);
            int16_t tsx = cx + (int16_t)tx;
            int16_t tsy = cy + (int16_t)ty;
            if (tsx >= 0 && tsx < SCREEN_WIDTH && tsy >= 0 && tsy < SCREEN_HEIGHT) {
                d->drawPixel(tsx, tsy, COLOR_WHITE);
            }
        }
    }
}

// ─── Waveform ───────────────────────────────────────────────────
// Layered sine waves scrolling across the screen

static void drawWaveform(DisplayCanvas* d) {
    float phase = (float)animTime / 1000.0f;

    // Three overlapping sine waves at different frequencies
    for (int wave = 0; wave < 3; wave++) {
        float freq = 0.05f + wave * 0.03f;
        float amp = 8.0f + wave * 4.0f;
        float speed = 1.5f + wave * 0.7f;
        int16_t baseY = SCREEN_HEIGHT / 2 + (wave - 1) * 2;

        int16_t prevY = -1;
        for (int16_t x = 0; x < SCREEN_WIDTH; x++) {
            float val = sinf(x * freq + phase * speed)
                      + sinf(x * freq * 1.7f - phase * speed * 0.6f) * 0.5f;
            int16_t y = baseY + (int16_t)(val * amp);
            y = constrain(y, 0, SCREEN_HEIGHT - 1);

            d->drawPixel(x, y, COLOR_WHITE);
            if (prevY >= 0 && abs(y - prevY) > 1) {
                // Connect discontinuities
                d->drawLine(x - 1, prevY, x, y, COLOR_WHITE);
            }
            prevY = y;
        }
    }
}

// ─── Public API ─────────────────────────────────────────────────

void init() {
    randomize();
}

void randomize() {
    Anim newAnim;
    do {
        newAnim = (Anim)(esp_random() % (uint8_t)Anim::NUM_ANIMS);
    } while (newAnim == currentAnim);
    currentAnim = newAnim;
    animTime = 0;
    animSwitchTime = 0;

    switch (currentAnim) {
        case Anim::BOUNCING_SKULL:     initBouncingSkull(); break;
        case Anim::STARFIELD:          initStarfield(); break;
        case Anim::MATRIX_RAIN:        initMatrixRain(); break;
        case Anim::ORBITING_PARTICLES: initOrbitingParticles(); break;
        case Anim::WAVEFORM:           break;  // No state to init
        default: break;
    }

    Serial.printf("[Screensaver] Animation -> %d\n", (int)currentAnim);
}

void update(uint32_t deltaMs) {
    animTime += deltaMs;
    animSwitchTime += deltaMs;

    if (animSwitchTime >= ANIM_CYCLE_MS) {
        randomize();
    }
}

void draw() {
    DisplayCanvas* d = Display::getCanvas();

    switch (currentAnim) {
        case Anim::BOUNCING_SKULL:     drawBouncingSkull(d); break;
        case Anim::STARFIELD:          drawStarfield(d); break;
        case Anim::MATRIX_RAIN:        drawMatrixRain(d); break;
        case Anim::ORBITING_PARTICLES: drawOrbitingParticles(d); break;
        case Anim::WAVEFORM:           drawWaveform(d); break;
        default: break;
    }
}

} // namespace Screensaver
