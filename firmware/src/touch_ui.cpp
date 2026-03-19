#ifdef BOARD_ESP32_4848S040C

#include "touch_ui.h"
#include "display.h"
#include "server.h"
#include "avatar.h"

extern String _hookbot_get_ip();

namespace TouchUI {

// ─── Overlay state ──────────────────────────────────────────────

enum class Panel : uint8_t {
    NONE,
    SETTINGS,
    ACCESSORIES,
    STATE_SELECT,
};

static Panel activePanel = Panel::NONE;
static float slideProgress = 0.0f;   // 0 = hidden, 1 = fully visible
static bool slideIn = false;
static bool slideOut = false;
static const float SLIDE_SPEED = 6.0f;  // units per second

// Swipe detection
static int16_t swipeStartY = -1;
static int16_t swipeStartX = -1;
static bool swipeTracking = false;
static uint32_t touchStartTime = 0;
static bool wasTouching = false;
static uint32_t lastTapTime = 0;

// Touch region tracking for buttons
static int16_t lastTouchX = -1;
static int16_t lastTouchY = -1;
static bool justReleased = false;

// ─── Helpers ────────────────────────────────────────────────────

static void openPanel(Panel p) {
    activePanel = p;
    slideIn = true;
    slideOut = false;
    Serial.printf("[TouchUI] Open panel %d\n", (int)p);
}

static void closePanel() {
    slideOut = true;
    slideIn = false;
    Serial.println("[TouchUI] Close panel");
}

// ─── Init ───────────────────────────────────────────────────────

void init() {
    Serial.println("[TouchUI] Touch UI initialized");
}

// ─── Update ─────────────────────────────────────────────────────

void update(uint32_t deltaMs, int16_t touchX, int16_t touchY, bool touching) {
    float dt = (float)deltaMs / 1000.0f;
    justReleased = false;

    // Slide animation
    if (slideIn) {
        slideProgress += SLIDE_SPEED * dt;
        if (slideProgress >= 1.0f) {
            slideProgress = 1.0f;
            slideIn = false;
        }
    }
    if (slideOut) {
        slideProgress -= SLIDE_SPEED * dt;
        if (slideProgress <= 0.0f) {
            slideProgress = 0.0f;
            slideOut = false;
            activePanel = Panel::NONE;
        }
    }

    // Touch handling
    if (touching && !wasTouching) {
        // Touch start
        swipeStartX = touchX;
        swipeStartY = touchY;
        swipeTracking = true;
        touchStartTime = millis();
        lastTouchX = touchX;
        lastTouchY = touchY;
    } else if (touching && wasTouching) {
        // Touch move
        lastTouchX = touchX;
        lastTouchY = touchY;
    } else if (!touching && wasTouching) {
        // Touch release
        justReleased = true;
        uint32_t touchDuration = millis() - touchStartTime;

        if (swipeTracking && activePanel == Panel::NONE) {
            int16_t dy = lastTouchY - swipeStartY;
            int16_t dx = lastTouchX - swipeStartX;

            // Swipe up from bottom -> settings
            if (dy < -8 && swipeStartY > 85 && abs(dx) < abs(dy)) {
                openPanel(Panel::SETTINGS);
            }
            // Swipe from left edge -> accessories
            else if (dx > 8 && swipeStartX < 25 && abs(dx) > abs(dy)) {
                openPanel(Panel::ACCESSORIES);
            }
            // Swipe from right edge -> state selector
            else if (dx < -8 && swipeStartX > 95 && abs(dx) > abs(dy)) {
                openPanel(Panel::STATE_SELECT);
            }
        } else if (activePanel != Panel::NONE) {
            int16_t dy = lastTouchY - swipeStartY;
            int16_t dx = lastTouchX - swipeStartX;
            // Swipe away to close panel (down for settings, right-to-left for accessories, etc.)
            if (abs(dy) > 10 || abs(dx) > 10) {
                closePanel();
            }
        }

        swipeTracking = false;
    }

    wasTouching = touching;
}

// ─── Drawing helpers ────────────────────────────────────────────

static void drawSettingsPanel(DisplayCanvas* d) {
    int16_t panelH = 80;
    int16_t panelY = 120 - (int16_t)(panelH * slideProgress);

    // Panel background
    d->fillRect(0, panelY, 120, panelH, COLOR_BLACK);
    d->drawFastHLine(0, panelY, 120, COLOR_WHITE);

    // Drag handle
    d->fillRoundRect(52, panelY + 2, 16, 3, 1, COLOR_WHITE);

    // Title
    d->setTextSize(1);
    d->setTextColor(COLOR_WHITE);
    d->setCursor(4, panelY + 8);
    d->print("Settings");

    RuntimeConfig& cfg = HookbotServer::getConfig();

    // Sound toggle
    int16_t rowY = panelY + 20;
    d->setCursor(4, rowY);
    d->print("Sound");
    // Toggle indicator
    if (cfg.soundEnabled) {
        d->fillRoundRect(90, rowY - 1, 20, 9, 4, COLOR_WHITE);
        d->fillCircle(104, rowY + 3, 3, COLOR_BLACK);
    } else {
        d->drawRoundRect(90, rowY - 1, 20, 9, 4, COLOR_WHITE);
        d->fillCircle(96, rowY + 3, 3, COLOR_WHITE);
    }
    // Tap zone for sound toggle
    if (justReleased && lastTouchY >= rowY - 2 && lastTouchY <= rowY + 10
        && lastTouchX >= 85 && lastTouchX <= 115) {
        cfg.soundEnabled = !cfg.soundEnabled;
        HookbotServer::saveConfigToNVS();
        Serial.printf("[TouchUI] Sound: %s\n", cfg.soundEnabled ? "ON" : "OFF");
    }

    // LED brightness
    rowY += 14;
    d->setCursor(4, rowY);
    d->print("LED");
    // Brightness bar
    int16_t barX = 40;
    int16_t barW = 70;
    d->drawRect(barX, rowY, barW, 7, COLOR_WHITE);
    int16_t fillW = (int16_t)((float)cfg.ledBrightness / 255.0f * (barW - 2));
    d->fillRect(barX + 1, rowY + 1, fillW, 5, COLOR_WHITE);
    // Tap to adjust brightness
    if (justReleased && lastTouchY >= rowY - 2 && lastTouchY <= rowY + 10
        && lastTouchX >= barX && lastTouchX <= barX + barW) {
        float pct = (float)(lastTouchX - barX) / (float)barW;
        cfg.ledBrightness = (int)(pct * 255);
        HookbotServer::saveConfigToNVS();
        Serial.printf("[TouchUI] LED brightness: %d\n", cfg.ledBrightness);
    }

    // Device info
    rowY += 14;
    d->setTextColor(COLOR_WHITE);
    d->setCursor(4, rowY);
    d->print("IP:");
    String ip = ::_hookbot_get_ip();
    d->setCursor(28, rowY);
    d->print(ip.c_str());

    rowY += 10;
    d->setCursor(4, rowY);
    d->print("FW:");
    d->setCursor(28, rowY);
    d->print(FIRMWARE_VERSION);
}

static void drawAccessoriesPanel(DisplayCanvas* d) {
    int16_t panelW = 70;
    int16_t panelX = -(int16_t)(panelW * (1.0f - slideProgress));

    // Panel background
    d->fillRect(panelX, 0, panelW, 120, COLOR_BLACK);
    d->drawFastVLine(panelX + panelW, 0, 120, COLOR_WHITE);

    // Title
    d->setTextSize(1);
    d->setTextColor(COLOR_WHITE);
    d->setCursor(panelX + 4, 4);
    d->print("Style");

    RuntimeConfig& cfg = HookbotServer::getConfig();

    struct AccItem {
        const char* name;
        bool* value;
    };
    AccItem items[] = {
        { "Hat",     &cfg.topHat },
        { "Cigar",   &cfg.cigar },
        { "Glasses", &cfg.glasses },
        { "Monocle", &cfg.monocle },
        { "Bowtie",  &cfg.bowtie },
        { "Crown",   &cfg.crown },
        { "Horns",   &cfg.horns },
        { "Halo",    &cfg.halo },
    };

    for (int i = 0; i < 8; i++) {
        int16_t rowY = 16 + i * 13;
        int16_t rowX = panelX + 4;

        d->setCursor(rowX, rowY);
        d->print(items[i].name);

        // Checkbox
        int16_t cbX = panelX + panelW - 14;
        if (*items[i].value) {
            d->fillRect(cbX, rowY - 1, 9, 9, COLOR_WHITE);
            // Checkmark
            d->drawPixel(cbX + 2, rowY + 4, COLOR_BLACK);
            d->drawPixel(cbX + 3, rowY + 5, COLOR_BLACK);
            d->drawPixel(cbX + 4, rowY + 4, COLOR_BLACK);
            d->drawPixel(cbX + 5, rowY + 3, COLOR_BLACK);
            d->drawPixel(cbX + 6, rowY + 2, COLOR_BLACK);
        } else {
            d->drawRect(cbX, rowY - 1, 9, 9, COLOR_WHITE);
        }

        // Tap to toggle
        if (justReleased && lastTouchY >= rowY - 3 && lastTouchY <= rowY + 10
            && lastTouchX >= panelX && lastTouchX <= panelX + panelW) {
            *items[i].value = !(*items[i].value);
            HookbotServer::saveConfigToNVS();
            Serial.printf("[TouchUI] %s: %s\n", items[i].name, *items[i].value ? "ON" : "OFF");
        }
    }
}

static void drawStatePanel(DisplayCanvas* d) {
    int16_t panelW = 55;
    int16_t panelX = 120 + (int16_t)(panelW * (1.0f - slideProgress)) - panelW;

    // Panel background
    d->fillRect(panelX, 0, panelW, 120, COLOR_BLACK);
    d->drawFastVLine(panelX, 0, 120, COLOR_WHITE);

    // Title
    d->setTextSize(1);
    d->setTextColor(COLOR_WHITE);
    d->setCursor(panelX + 4, 4);
    d->print("Mood");

    const char* stateNames[] = {"idle", "think", "wait", "win!", "check", "rage!"};

    for (int i = 0; i < 6; i++) {
        int16_t rowY = 16 + i * 17;
        int16_t rowX = panelX + 4;

        bool isActive = ((int)Avatar::getState() == i);

        if (isActive) {
            d->fillRoundRect(rowX - 2, rowY - 2, panelW - 4, 13, 2, COLOR_WHITE);
            d->setTextColor(COLOR_BLACK);
        } else {
            d->drawRoundRect(rowX - 2, rowY - 2, panelW - 4, 13, 2, COLOR_WHITE);
            d->setTextColor(COLOR_WHITE);
        }

        d->setCursor(rowX + 2, rowY);
        d->print(stateNames[i]);
    }
}

// ─── Swipe hint indicators ─────────────────────────────────────

static void drawSwipeHints(DisplayCanvas* d) {
    if (activePanel != Panel::NONE) return;

    uint32_t t = millis();
    float pulse = sinf((float)t / 800.0f) * 0.5f + 0.5f;

    // Bottom center: up arrow hint (settings)
    if (pulse > 0.3f) {
        int16_t cx = 60;
        int16_t y = 117;
        d->drawPixel(cx, y - 2, COLOR_WHITE);
        d->drawPixel(cx - 1, y - 1, COLOR_WHITE);
        d->drawPixel(cx + 1, y - 1, COLOR_WHITE);
        d->drawPixel(cx - 2, y, COLOR_WHITE);
        d->drawPixel(cx + 2, y, COLOR_WHITE);
    }

    // Left edge: right arrow hint (accessories)
    if (pulse > 0.5f) {
        int16_t y = 60;
        d->drawPixel(1, y, COLOR_WHITE);
        d->drawPixel(2, y - 1, COLOR_WHITE);
        d->drawPixel(2, y + 1, COLOR_WHITE);
    }

    // Right edge: left arrow hint (states)
    if (pulse > 0.5f) {
        int16_t y = 60;
        d->drawPixel(118, y, COLOR_WHITE);
        d->drawPixel(117, y - 1, COLOR_WHITE);
        d->drawPixel(117, y + 1, COLOR_WHITE);
    }
}

// ─── Public API ─────────────────────────────────────────────────

void draw() {
    DisplayCanvas* d = Display::getCanvas();

    drawSwipeHints(d);

    if (activePanel == Panel::NONE && slideProgress <= 0.0f) return;

    switch (activePanel) {
        case Panel::SETTINGS:     drawSettingsPanel(d); break;
        case Panel::ACCESSORIES:  drawAccessoriesPanel(d); break;
        case Panel::STATE_SELECT: drawStatePanel(d); break;
        default: break;
    }
}

bool isOverlayActive() {
    return activePanel != Panel::NONE || slideProgress > 0.0f;
}

} // namespace TouchUI

#endif // BOARD_ESP32_4848S040C
