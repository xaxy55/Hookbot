#ifdef BOARD_ESP32_4848S040C

#include "touch_ui.h"
#include "display.h"
#include "server.h"
#include "avatar.h"
#include "cloud_client.h"

extern String _hookbot_get_ip();

namespace TouchUI {

// ─── Overlay state ──────────────────────────────────────────────

enum class Panel : uint8_t {
    NONE,
    SETTINGS,
    ACCESSORIES,
    STATE_SELECT,
    POMODORO,
    PET_FULLSCREEN,
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

// ─── Pomodoro completion flash ──────────────────────────────────
static bool pomoFlashActive = false;
static uint32_t pomoFlashStart = 0;
static const uint32_t POMO_FLASH_DURATION = 5000; // 5 seconds max

// Forward declarations for pet fullscreen mode
static void updatePetAnim(float dt);
static void drawPetFullscreen(DisplayCanvas* d);
static void resetPetFullscreen();

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

    // Pet animation update
    if (activePanel == Panel::PET_FULLSCREEN) {
        updatePetAnim(dt);
    }

    // Check for pomodoro completion -> start flash
    if (HookbotServer::pomodoroJustCompleted()) {
        HookbotServer::clearPomodoroCompleted();
        pomoFlashActive = true;
        pomoFlashStart = millis();
    }

    // Auto-dismiss flash after timeout
    if (pomoFlashActive && (millis() - pomoFlashStart >= POMO_FLASH_DURATION)) {
        pomoFlashActive = false;
    }

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
        // Dismiss flash on any touch
        if (pomoFlashActive) {
            pomoFlashActive = false;
        }
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
            // Swipe down from top -> pomodoro
            else if (dy > 8 && swipeStartY < 25 && abs(dy) > abs(dx)) {
                openPanel(Panel::POMODORO);
            }
            // Tap on pet sprite (bottom-right area) -> fullscreen pet
            else if (abs(dy) < 5 && abs(dx) < 5 && touchDuration < 400
                     && swipeStartX >= 80 && swipeStartY >= 78) {
                openPanel(Panel::PET_FULLSCREEN);
            }
        } else if (activePanel == Panel::PET_FULLSCREEN) {
            int16_t dy = lastTouchY - swipeStartY;
            // Swipe down to exit fullscreen pet
            if (dy > 15) {
                closePanel();
                resetPetFullscreen();
            }
            // Other taps handled by drawPetFullscreen
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

// Reclaim confirmation state
static bool showReclaimConfirm = false;

static void drawSettingsPanel(DisplayCanvas* d) {
    int16_t panelH = 120;  // Full height to fit cloud info
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
    int16_t rowY = panelY + 18;
    d->setCursor(4, rowY);
    d->print("Sound");
    if (cfg.soundEnabled) {
        d->fillRoundRect(90, rowY - 1, 20, 9, 4, COLOR_WHITE);
        d->fillCircle(104, rowY + 3, 3, COLOR_BLACK);
    } else {
        d->drawRoundRect(90, rowY - 1, 20, 9, 4, COLOR_WHITE);
        d->fillCircle(96, rowY + 3, 3, COLOR_WHITE);
    }
    if (justReleased && lastTouchY >= rowY - 2 && lastTouchY <= rowY + 10
        && lastTouchX >= 85 && lastTouchX <= 115) {
        cfg.soundEnabled = !cfg.soundEnabled;
        HookbotServer::saveConfigToNVS();
        Serial.printf("[TouchUI] Sound: %s\n", cfg.soundEnabled ? "ON" : "OFF");
    }

    // Display brightness
    rowY += 12;
    d->setCursor(4, rowY);
    d->print("Bright");
    int16_t barX = 44;
    int16_t barW = 66;
    d->drawRect(barX, rowY, barW, 7, COLOR_WHITE);
    int16_t fillW = (int16_t)((float)cfg.ledBrightness / 255.0f * (barW - 2));
    d->fillRect(barX + 1, rowY + 1, fillW, 5, COLOR_WHITE);
    if (justReleased && lastTouchY >= rowY - 2 && lastTouchY <= rowY + 10
        && lastTouchX >= barX && lastTouchX <= barX + barW) {
        float pct = (float)(lastTouchX - barX) / (float)barW;
        cfg.ledBrightness = (int)(pct * 255);
        if (cfg.ledBrightness < 10) cfg.ledBrightness = 10;
        Display::setBrightness(cfg.ledBrightness);
        HookbotServer::saveConfigToNVS();
        Serial.printf("[TouchUI] Display brightness: %d\n", cfg.ledBrightness);
    }

    // Screensaver timeout
    rowY += 12;
    d->setCursor(4, rowY);
    d->print("Sleep");
    char ssBuf[12];
    if (cfg.screensaverMins == 0) {
        snprintf(ssBuf, sizeof(ssBuf), "OFF");
    } else {
        snprintf(ssBuf, sizeof(ssBuf), "%dm", cfg.screensaverMins);
    }
    d->setCursor(90, rowY);
    d->print(ssBuf);
    if (justReleased && lastTouchY >= rowY - 2 && lastTouchY <= rowY + 10) {
        if (lastTouchX >= 70 && lastTouchX < 95) {
            int vals[] = {0, 5, 10, 15, 30, 60};
            int n = sizeof(vals)/sizeof(vals[0]);
            for (int i = n - 1; i >= 0; i--) {
                if (vals[i] < cfg.screensaverMins) { cfg.screensaverMins = vals[i]; break; }
            }
            HookbotServer::saveConfigToNVS();
        } else if (lastTouchX >= 95) {
            int vals[] = {0, 5, 10, 15, 30, 60};
            int n = sizeof(vals)/sizeof(vals[0]);
            for (int i = 0; i < n; i++) {
                if (vals[i] > cfg.screensaverMins) { cfg.screensaverMins = vals[i]; break; }
            }
            HookbotServer::saveConfigToNVS();
        }
    }

    // ─── Divider ─────────────────────────────────────────────
    rowY += 12;
    d->drawFastHLine(4, rowY, 112, 0x4208); // dim gray line

    // ─── Server / Cloud Info ─────────────────────────────────
    rowY += 5;
    d->setTextColor(0x4208); // dim label
    d->setCursor(4, rowY);
    d->print("SERVER");

    rowY += 10;
    d->setTextColor(COLOR_WHITE);
    d->setCursor(4, rowY);
    if (strlen(cfg.mgmtServer) > 0) {
        // Truncate long URLs to fit display
        char truncUrl[20];
        const char* url = cfg.mgmtServer;
        // Skip "http://" or "https://"
        if (strncmp(url, "https://", 8) == 0) url += 8;
        else if (strncmp(url, "http://", 7) == 0) url += 7;
        strncpy(truncUrl, url, sizeof(truncUrl) - 1);
        truncUrl[sizeof(truncUrl) - 1] = '\0';
        d->print(truncUrl);
    } else {
        d->print("(none - local)");
    }

    // Cloud status
    rowY += 10;
    d->setCursor(4, rowY);
    if (CloudClient::isEnabled()) {
        if (CloudClient::isClaimed()) {
            d->setTextColor(0x07E0); // green
            d->print("Claimed");
        } else {
            d->setTextColor(0xFFE0); // yellow
            d->print("Code: ");
            d->print(CloudClient::getClaimCode());
        }
    } else {
        d->setTextColor(0x4208);
        d->print("Local mode");
    }

    // Reclaim button (only when cloud is enabled)
    if (CloudClient::isEnabled()) {
        rowY += 12;
        if (showReclaimConfirm) {
            // Confirmation prompt
            d->setTextColor(0xF800); // red
            d->setCursor(4, rowY);
            d->print("Reset cloud?");
            // Yes button
            d->fillRoundRect(80, rowY - 2, 16, 11, 3, 0xF800);
            d->setTextColor(COLOR_WHITE);
            d->setCursor(83, rowY);
            d->print("Y");
            // No button
            d->fillRoundRect(100, rowY - 2, 16, 11, 3, 0x4208);
            d->setCursor(104, rowY);
            d->print("N");

            if (justReleased && lastTouchY >= rowY - 4 && lastTouchY <= rowY + 12) {
                if (lastTouchX >= 78 && lastTouchX < 98) {
                    // Yes - reset cloud
                    CloudClient::resetCloud();
                    cfg.mgmtServer[0] = '\0';
                    HookbotServer::saveConfigToNVS();
                    showReclaimConfirm = false;
                    Serial.println("[TouchUI] Cloud reset — device unclaimed");
                } else if (lastTouchX >= 98) {
                    // No - cancel
                    showReclaimConfirm = false;
                }
            }
        } else {
            // Reclaim button
            d->fillRoundRect(4, rowY - 2, 60, 11, 3, 0x4208);
            d->setTextColor(COLOR_WHITE);
            d->setCursor(8, rowY);
            d->print("Reclaim");
            if (justReleased && lastTouchY >= rowY - 4 && lastTouchY <= rowY + 12
                && lastTouchX >= 2 && lastTouchX <= 66) {
                showReclaimConfirm = true;
            }
        }
    }

    // IP & FW (bottom)
    rowY += 14;
    d->setTextColor(0x4208);
    d->setCursor(4, rowY);
    d->print("IP:");
    d->setTextColor(COLOR_WHITE);
    String ip = ::_hookbot_get_ip();
    d->setCursor(28, rowY);
    d->print(ip.c_str());

    rowY += 9;
    d->setTextColor(0x4208);
    d->setCursor(4, rowY);
    d->print("FW:");
    d->setTextColor(COLOR_WHITE);
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

// ─── Full-screen Pet Mode ──────────────────────────────────────

enum class PetAnim : uint8_t {
    IDLE,
    EAT,
    DRINK,
    BACKFLIP,
    SPIN,
    HEARTS,
    SLEEP,
};

static PetAnim petAnimState = PetAnim::IDLE;
static float petAnimTimer = 0;
static float petAnimDuration = 0;
static float petFsX = 44;  // fullscreen pet position
static float petFsY = 50;
static float petFsRot = 0; // rotation for tricks
static float petFsScale = 1.0f;
static int heartCount = 0;
static float heartPhase[5] = {};
static float petFsVelY = 0; // for bounce after backflip

static void resetPetFullscreen() {
    petAnimState = PetAnim::IDLE;
    petFsX = 44; petFsY = 50; petFsRot = 0; petFsScale = 1.0f;
}

static void startPetAnim(PetAnim anim, float dur) {
    petAnimState = anim;
    petAnimTimer = 0;
    petAnimDuration = dur;
    petFsRot = 0;
    petFsScale = 1.0f;
    petFsVelY = 0;
    if (anim == PetAnim::HEARTS) {
        heartCount = 5;
        for (int i = 0; i < 5; i++) heartPhase[i] = (float)i * 0.3f;
    }
}

static void updatePetAnim(float dt) {
    if (petAnimState == PetAnim::IDLE) {
        // Gentle breathing bob
        petFsY = 50 + sinf(petAnimTimer * 1.5f) * 2.0f;
        petAnimTimer += dt;
        return;
    }

    petAnimTimer += dt;
    float t = petAnimDuration > 0 ? petAnimTimer / petAnimDuration : 1.0f;
    if (t > 1.0f) t = 1.0f;

    switch (petAnimState) {
        case PetAnim::EAT:
            // Bob down to "eat", then back up
            if (t < 0.3f) {
                petFsY = 50 + (t / 0.3f) * 12;  // lean down
            } else if (t < 0.7f) {
                // Chomp animation - bob up and down rapidly
                float chomp = sinf((t - 0.3f) * 40.0f) * 3;
                petFsY = 62 + chomp;
            } else {
                petFsY = 62 - ((t - 0.7f) / 0.3f) * 12;  // come back up
            }
            break;

        case PetAnim::DRINK:
            // Lean to the side toward bowl
            if (t < 0.2f) {
                petFsX = 44 + (t / 0.2f) * 10;
                petFsY = 50 + (t / 0.2f) * 8;
            } else if (t < 0.8f) {
                // Lapping animation
                float lap = sinf((t - 0.2f) * 25.0f) * 2;
                petFsX = 54;
                petFsY = 58 + lap;
            } else {
                petFsX = 54 - ((t - 0.8f) / 0.2f) * 10;
                petFsY = 58 - ((t - 0.8f) / 0.2f) * 8;
            }
            break;

        case PetAnim::BACKFLIP:
            // Jump up, rotate 360, land with bounce
            if (t < 0.15f) {
                // Crouch
                petFsY = 50 + (t / 0.15f) * 5;
                petFsScale = 1.0f - (t / 0.15f) * 0.2f;
            } else if (t < 0.7f) {
                // In the air, rotating
                float airT = (t - 0.15f) / 0.55f;
                petFsY = 55 - sinf(airT * 3.14159f) * 40;  // arc
                petFsRot = airT * 360.0f;
                petFsScale = 1.0f;
            } else if (t < 0.85f) {
                // Landing impact
                float landT = (t - 0.7f) / 0.15f;
                petFsY = 50 + sinf(landT * 3.14159f) * 4;
                petFsRot = 360;
                petFsScale = 1.0f + sinf(landT * 3.14159f) * 0.15f; // squash
            } else {
                petFsY = 50;
                petFsRot = 0;
                petFsScale = 1.0f;
            }
            break;

        case PetAnim::SPIN:
            // Spin in place with a little hop
            petFsY = 50 - sinf(t * 3.14159f) * 8;
            petFsRot = t * 720.0f;  // 2 full spins
            break;

        case PetAnim::HEARTS:
            // Stay in place, hearts float up
            petFsY = 50 + sinf(petAnimTimer * 3.0f) * 1.5f;
            break;

        case PetAnim::SLEEP:
            // Slow breathing, lower position
            petFsY = 58 + sinf(petAnimTimer * 0.8f) * 1.0f;
            break;

        default:
            break;
    }

    if (t >= 1.0f) {
        petAnimState = PetAnim::IDLE;
        petFsX = 44;
        petFsY = 50;
        petFsRot = 0;
        petFsScale = 1.0f;
    }
}

// forward declare - defined after sprite data
static void drawSprite3x(DisplayCanvas* d, int petIdx, int16_t cx, int16_t cy, float rot, float scale);

static void drawPetFullscreen(DisplayCanvas* d) {
    PetData& pet = HookbotServer::getPetData();
    int petIdx = (int)pet.activePet;
    if (petIdx < 0 || petIdx >= 4) petIdx = 0;

    // Full black background
    d->fillRect(0, 0, 120, 120, COLOR_BLACK);

    // Ground line
    d->drawFastHLine(0, 105, 120, 0x2104);

    // ── Draw food bowl (left side, small) ──
    uint16_t bowlClr = 0xC618;
    d->fillRoundRect(10, 97, 12, 7, 2, bowlClr);
    d->fillRect(11, 95, 10, 3, 0xFCA0); // food

    // ── Draw water bowl (right side, small) ──
    d->fillRoundRect(98, 97, 12, 7, 2, bowlClr);
    d->fillRect(99, 95, 10, 2, 0x065F); // water
    float ripple = sinf(millis() / 300.0f);
    d->drawPixel(102 + (int)ripple, 95, 0x0EBF);

    // ── Draw pet sprite (3x, centered) ──
    drawSprite3x(d, petIdx, (int16_t)petFsX, (int16_t)petFsY, petFsRot, petFsScale);

    // ── Heart particles ──
    if (petAnimState == PetAnim::HEARTS) {
        for (int i = 0; i < heartCount; i++) {
            float phase = heartPhase[i] + petAnimTimer * 2.0f;
            float hx = petFsX + sinf(phase * 2.0f + i) * 15;
            float hy = petFsY - 10 - fmodf(phase * 20.0f, 40.0f);
            float alpha = 1.0f - fmodf(phase * 20.0f, 40.0f) / 40.0f;
            if (alpha > 0 && hy > 0) {
                uint16_t hclr = 0xF8B2; // pink
                int sz = (alpha > 0.5f) ? 2 : 1;
                d->fillRect((int)hx - 1, (int)hy, sz, sz, hclr);
                d->fillRect((int)hx + 1, (int)hy, sz, sz, hclr);
                d->fillRect((int)hx, (int)hy + 1, sz, sz, hclr);
            }
        }
    }

    // ── Zzz for sleep ──
    if (petAnimState == PetAnim::SLEEP) {
        float zPhase = petAnimTimer * 0.7f;
        for (int i = 0; i < 3; i++) {
            float zx = petFsX + 15 + i * 6 + sinf(zPhase + i) * 3;
            float zy = petFsY - 15 - i * 8 - fmodf(zPhase * 5, 10);
            d->setTextSize(1);
            d->setTextColor(0x8410);
            d->setCursor((int)zx, (int)zy);
            d->print("z");
        }
    }

    // ── Stat bars (top) ──
    int16_t barW = 50;
    int16_t barH = 5;

    // Hunger
    d->setTextSize(1);
    d->setTextColor(0xFCA0);
    d->setCursor(4, 4);
    d->print("Hunger");
    d->drawRect(44, 4, barW, barH, 0x4208);
    int16_t hFill = (int16_t)((float)pet.hunger / 100.0f * (barW - 2));
    if (hFill > 0) d->fillRect(45, 5, hFill, barH - 2, 0xFCA0);

    // Happiness
    d->setTextColor(0xF8B2);
    d->setCursor(4, 12);
    d->print("Happy");
    d->drawRect(44, 12, barW, barH, 0x4208);
    int16_t jFill = (int16_t)((float)pet.happiness / 100.0f * (barW - 2));
    if (jFill > 0) d->fillRect(45, 13, jFill, barH - 2, 0xF8B2);

    // Mood emoji text
    d->setTextColor(COLOR_WHITE);
    d->setCursor(100, 4);
    const char* moodStr;
    int avg = (pet.hunger + pet.happiness) / 2;
    if (avg >= 80) moodStr = ":D";
    else if (avg >= 60) moodStr = ":)";
    else if (avg >= 40) moodStr = ":|";
    else if (avg >= 20) moodStr = ":(";
    else moodStr = ";(";
    d->print(moodStr);

    // ── Action buttons (bottom) ──
    int16_t btnY = 108;
    int16_t btnH = 11;

    // Trick button (center-left)
    d->fillRoundRect(30, btnY, 28, btnH, 3, 0x631F); // indigo
    d->setTextSize(1);
    d->setTextColor(COLOR_WHITE);
    d->setCursor(34, btnY + 2);
    d->print("Trick");

    // Pet button (center-right)
    d->fillRoundRect(62, btnY, 28, btnH, 3, 0xF8B2); // pink
    d->setTextColor(COLOR_BLACK);
    d->setCursor(70, btnY + 2);
    d->print("Pet");

    // Swipe-down hint (top-right)
    d->setTextSize(1);
    d->setTextColor(0x4208);
    d->drawPixel(110, 4, 0x4208);
    d->drawPixel(109, 5, 0x4208);
    d->drawPixel(111, 5, 0x4208);
    d->drawPixel(108, 6, 0x4208);
    d->drawPixel(112, 6, 0x4208);

    // ── Touch handling ──
    if (justReleased) {
        // Feed button (tap on food bowl)
        if (lastTouchX >= 5 && lastTouchX <= 28 && lastTouchY >= 90 && lastTouchY <= 105) {
            if (petAnimState == PetAnim::IDLE) {
                startPetAnim(PetAnim::EAT, 1.5f);
                pet.hunger = min(100, pet.hunger + 15);
                pet.totalFeeds++;
                pet.lastFedAt = millis();
                Serial.println("[Pet] Fed via touch screen");
            }
        }
        // Water button (tap on water bowl)
        else if (lastTouchX >= 92 && lastTouchX <= 115 && lastTouchY >= 90 && lastTouchY <= 105) {
            if (petAnimState == PetAnim::IDLE) {
                startPetAnim(PetAnim::DRINK, 2.0f);
                pet.hunger = min(100, pet.hunger + 8);
                pet.happiness = min(100, pet.happiness + 5);
                Serial.println("[Pet] Drinking via touch screen");
            }
        }
        // Trick button
        else if (lastTouchX >= 30 && lastTouchX <= 58 && lastTouchY >= btnY && lastTouchY <= btnY + btnH) {
            if (petAnimState == PetAnim::IDLE) {
                // Alternate between backflip and spin
                static bool doFlip = true;
                if (doFlip) {
                    startPetAnim(PetAnim::BACKFLIP, 1.2f);
                } else {
                    startPetAnim(PetAnim::SPIN, 0.8f);
                }
                doFlip = !doFlip;
                pet.happiness = min(100, pet.happiness + 10);
                Serial.println("[Pet] Trick via touch screen");
            }
        }
        // Pet button
        else if (lastTouchX >= 62 && lastTouchX <= 90 && lastTouchY >= btnY && lastTouchY <= btnY + btnH) {
            if (petAnimState == PetAnim::IDLE) {
                startPetAnim(PetAnim::HEARTS, 2.0f);
                pet.happiness = min(100, pet.happiness + 20);
                pet.totalPets++;
                pet.lastPetAt = millis();
                Serial.println("[Pet] Petted via touch screen");
            }
        }
        // Tap on pet itself (center area) -> random trick
        else if (lastTouchX >= 25 && lastTouchX <= 75 && lastTouchY >= 30 && lastTouchY <= 80) {
            if (petAnimState == PetAnim::IDLE) {
                int r = random(0, 4);
                switch (r) {
                    case 0: startPetAnim(PetAnim::BACKFLIP, 1.2f); break;
                    case 1: startPetAnim(PetAnim::SPIN, 0.8f); break;
                    case 2: startPetAnim(PetAnim::HEARTS, 2.0f); break;
                    case 3: startPetAnim(PetAnim::SLEEP, 3.0f); break;
                }
                pet.happiness = min(100, pet.happiness + 5);
                Serial.println("[Pet] Tapped -> random trick");
            }
        }
        // (swipe down to exit - handled in update)
    }
}

static void drawPomodoroPanel(DisplayCanvas* d) {
    PomodoroData& pomo = HookbotServer::getPomodoro();

    int16_t panelH = 95;
    int16_t panelY = -(int16_t)(panelH * (1.0f - slideProgress));

    // Panel background
    d->fillRect(0, panelY, 120, panelH, COLOR_BLACK);
    d->drawFastHLine(0, panelY + panelH, 120, COLOR_WHITE);

    // Drag handle
    d->fillRoundRect(52, panelY + panelH - 4, 16, 3, 1, COLOR_WHITE);

    // Session colors
    uint16_t focusClr = 0x631F;   // indigo
    uint16_t shortClr = 0x4726;   // green
    uint16_t longClr  = 0x0EBE;   // cyan
    uint16_t sessionClr = focusClr;
    if (pomo.session == PomodoroSession::SHORT_BREAK) sessionClr = shortClr;
    if (pomo.session == PomodoroSession::LONG_BREAK) sessionClr = longClr;

    // ── Session tabs (Focus / Short / Long) ──
    int16_t tabY = panelY + 3;
    struct TabDef { const char* label; PomodoroSession s; uint16_t clr; int16_t x; int16_t w; };
    TabDef tabs[] = {
        { "Focus",  PomodoroSession::FOCUS,       focusClr, 4,  36 },
        { "Short",  PomodoroSession::SHORT_BREAK,  shortClr, 42, 36 },
        { "Long",   PomodoroSession::LONG_BREAK,   longClr,  80, 36 },
    };
    for (int i = 0; i < 3; i++) {
        bool active = (pomo.session == tabs[i].s);
        if (active) {
            d->fillRoundRect(tabs[i].x, tabY, tabs[i].w, 11, 3, tabs[i].clr);
            d->setTextColor(COLOR_BLACK);
        } else {
            d->drawRoundRect(tabs[i].x, tabY, tabs[i].w, 11, 3, 0x4208);
            d->setTextColor(0x8410);
        }
        d->setTextSize(1);
        d->setCursor(tabs[i].x + 4, tabY + 2);
        d->print(tabs[i].label);

        // Tap to switch session (only when idle/paused)
        if (justReleased && pomo.status != PomodoroStatus::RUNNING
            && lastTouchY >= tabY && lastTouchY <= tabY + 11
            && lastTouchX >= tabs[i].x && lastTouchX <= tabs[i].x + tabs[i].w) {
            pomo.session = tabs[i].s;
            pomo.status = PomodoroStatus::IDLE;
            int dur = (i == 0) ? pomo.focusMins : (i == 1) ? pomo.shortBreakMins : pomo.longBreakMins;
            pomo.timeLeftSec = dur * 60;
            pomo.totalDurationSec = dur * 60;
            Serial.printf("[Pomodoro] Switched to %s\n", tabs[i].label);
        }
    }

    // ── Timer display (large) ──
    int mins = pomo.timeLeftSec / 60;
    int secs = pomo.timeLeftSec % 60;
    char timeBuf[8];
    snprintf(timeBuf, sizeof(timeBuf), "%02d:%02d", mins, secs);

    d->setTextSize(2);
    d->setTextColor(COLOR_WHITE);
    d->setCursor(25, panelY + 20);
    d->print(timeBuf);

    // Status text
    d->setTextSize(1);
    d->setTextColor(sessionClr);
    if (pomo.status == PomodoroStatus::RUNNING) {
        d->setCursor(90, panelY + 22);
        d->print("RUN");
    } else if (pomo.status == PomodoroStatus::PAUSED) {
        d->setTextColor(0xFD20);
        d->setCursor(84, panelY + 22);
        d->print("PAUS");
    }

    // ── Progress bar ──
    int16_t barY = panelY + 40;
    int16_t barW = 112;
    float progress = pomo.totalDurationSec > 0
        ? 1.0f - ((float)pomo.timeLeftSec / (float)pomo.totalDurationSec) : 0;
    int16_t fillW = (int16_t)(progress * (barW - 2));
    d->drawRect(4, barY, barW, 5, 0x4208);
    if (fillW > 0) d->fillRect(5, barY + 1, fillW, 3, sessionClr);

    // ── Cycle dots ──
    int16_t dotY = panelY + 50;
    d->setTextSize(1);
    d->setTextColor(0x8410);
    d->setCursor(4, dotY);
    d->print("Cycle:");
    for (int i = 0; i < 4; i++) {
        int16_t dx = 50 + i * 12;
        bool done = (pomo.focusCount % 4) > i;
        bool current = (pomo.focusCount % 4) == i && pomo.session == PomodoroSession::FOCUS;
        if (done) {
            d->fillCircle(dx, dotY + 3, 3, focusClr);
        } else if (current && pomo.status == PomodoroStatus::RUNNING) {
            d->drawCircle(dx, dotY + 3, 3, focusClr);
            d->fillCircle(dx, dotY + 3, 1, focusClr);
        } else {
            d->drawCircle(dx, dotY + 3, 3, 0x4208);
        }
    }

    // ── Stats row ──
    int16_t statY = panelY + 61;
    d->setTextSize(1);
    d->setTextColor(0x8410);
    d->setCursor(4, statY);
    char statBuf[24];
    snprintf(statBuf, sizeof(statBuf), "%d done  %dm focus", pomo.todaySessions, pomo.todayMinutes);
    d->print(statBuf);

    // ── Controls ──
    int16_t btnY = panelY + 73;
    bool isRunning = pomo.status == PomodoroStatus::RUNNING;
    uint16_t btnClr = isRunning ? 0xFD20 : sessionClr;

    // Start/Pause button
    d->fillRoundRect(4, btnY, 55, 14, 3, btnClr);
    d->setTextSize(1);
    d->setTextColor(COLOR_BLACK);
    if (isRunning) {
        d->setCursor(16, btnY + 4);
        d->print("Pause");
    } else if (pomo.status == PomodoroStatus::PAUSED) {
        d->setCursor(12, btnY + 4);
        d->print("Resume");
    } else {
        d->setCursor(16, btnY + 4);
        d->print("Start");
    }
    if (justReleased && lastTouchY >= btnY && lastTouchY <= btnY + 14
        && lastTouchX >= 4 && lastTouchX <= 59) {
        if (pomo.status == PomodoroStatus::RUNNING) {
            pomo.status = PomodoroStatus::PAUSED;
        } else {
            pomo.status = PomodoroStatus::RUNNING;
            pomo.lastTickAt = millis();
        }
    }

    // Reset button
    d->fillRoundRect(63, btnY, 53, 14, 3, 0x4208);
    d->setTextColor(COLOR_WHITE);
    d->setCursor(76, btnY + 4);
    d->print("Reset");
    if (justReleased && lastTouchY >= btnY && lastTouchY <= btnY + 14
        && lastTouchX >= 63 && lastTouchX <= 116) {
        pomo.status = PomodoroStatus::IDLE;
        pomo.session = PomodoroSession::FOCUS;
        pomo.timeLeftSec = pomo.focusMins * 60;
        pomo.totalDurationSec = pomo.focusMins * 60;
    }
}

// ─── Swipe hint indicators ─────────────────────────────────────

// ─── Pet pixel art sprites (16x16, 1-bit) ─────────────────────
// Each row is a 16-bit value, MSB = leftmost pixel

// Dog: sitting golden retriever, floppy ears, tail up
static const uint16_t SPRITE_DOG[16] = {
    0b0000000000000000,
    0b0000110000110000,  // floppy ears
    0b0011111111110000,  // head
    0b0010011001110000,  // eyes (dark spots)
    0b0011111111110000,  // face
    0b0000110110100000,  // nose + mouth
    0b0000001110000000,  // chin
    0b0000001110000010,  // neck + tail tip
    0b0000111111100110,  // body + tail
    0b0001111111101100,  // body + tail
    0b0001111111111000,  // body
    0b0001111111110000,  // body
    0b0001111111100000,  // lower body
    0b0001100001100000,  // legs
    0b0001100001100000,  // legs
    0b0001100001100000,  // paws
};

// Cat: sitting, pointy ears, curled tail
static const uint16_t SPRITE_CAT[16] = {
    0b0010000000100000,  // ear tips
    0b0011000001100000,  // ears
    0b0011111111100000,  // head
    0b0010011001100000,  // eyes
    0b0011111011100000,  // face + nose
    0b0000111111000000,  // chin
    0b0000011110000000,  // neck
    0b0000011110000000,  // neck
    0b0000111111000000,  // body
    0b0001111111000000,  // body
    0b0001111111000010,  // body + tail
    0b0001111111000100,  // body + tail curl
    0b0001111111001000,  // body + tail
    0b0000110011000000,  // legs
    0b0000110011000000,  // legs
    0b0000110011000000,  // paws
};

// Robot: cute bot with antenna and screen face
static const uint16_t SPRITE_ROBOT[16] = {
    0b0000001000000000,  // antenna tip
    0b0000001110000000,  // antenna
    0b0000001100000000,  // antenna stem
    0b0011111111100000,  // head top
    0b0011000000100000,  // screen top
    0b0011010010100000,  // eyes
    0b0011000000100000,  // face
    0b0011011110100000,  // mouth
    0b0011111111100000,  // head bottom
    0b0000011110000000,  // neck
    0b0001111111000000,  // body
    0b0011111111100000,  // body + arms
    0b0001111111000000,  // body
    0b0001111111000000,  // body bottom
    0b0001100001100000,  // legs
    0b0011100011100000,  // feet
};

// Dragon: small dragon with wings and horns
static const uint16_t SPRITE_DRAGON[16] = {
    0b0010000001000000,  // horns
    0b0000111110000000,  // head top
    0b0001111111000000,  // head
    0b0001011011000000,  // eyes
    0b0001111111000000,  // face
    0b0000011100000000,  // snout
    0b0000001100000000,  // neck
    0b1100011110000000,  // wings + body
    0b0110111111000000,  // wings + body
    0b0011111111000000,  // body
    0b0001111111100000,  // body
    0b0001111111100010,  // body + tail
    0b0001111111000110,  // body + tail
    0b0000110011001000,  // legs + tail tip
    0b0000110011000000,  // legs
    0b0001110111000000,  // claws
};

static const uint16_t* PET_SPRITES[] = {
    SPRITE_DOG, SPRITE_CAT, SPRITE_ROBOT, SPRITE_DRAGON
};

// RGB565 color palettes per pet (body, dark detail, accent/eyes)
// Each pet gets 3 colors matching the frontend palettes
static const uint16_t PET_BODY_COLOR[] = {
    0xD5A9,  // Dog: warm golden (#D4A853)
    0x9CF5,  // Cat: silver grey (#9898A8)
    0x7CDB,  // Robot: steel blue (#7799BB)
    0x45E9,  // Dragon: green (#44BB55)
};
static const uint16_t PET_DARK_COLOR[] = {
    0x8B42,  // Dog: brown (#8B6914)
    0x6033,  // Cat: dark grey (#606068)
    0x3228,  // Robot: dark steel (#334455)
    0x2329,  // Dragon: dark green (#226633)
};
static const uint16_t PET_ACCENT_COLOR[] = {
    0x2104,  // Dog: near black eyes (#222222)
    0x4666,  // Cat: green eyes (#44CC44)
    0x477F,  // Robot: cyan glow (#44EEFF)
    0xFD44,  // Dragon: orange (#FFAA22)
};

// 2-bit sprite data for multi-color rendering
// Pack 2 bits per pixel into the existing 16-bit sprite rows:
//   bit 1 (from SPRITE_*) = pixel on/off
//   We use a separate "detail mask" to mark which pixels get dark/accent color
// For simplicity, encode detail rows per pet (rows where eyes/nose/paws are)

// Per-pet detail sprites: 0=body, 1=dark, 2=accent  (only for "on" pixels)
// Rows that are all-body can be 0x0000
static const uint16_t SPRITE_DOG_DARK[16] = {
    0,0,0,0,0,
    0b0000000100100000,  // row 5: nose
    0,0,0,0,0,0,0,0,
    0b0001100001100000,  // row 14: paw tips
    0b0001100001100000,  // row 15: paws
};
static const uint16_t SPRITE_DOG_ACCENT[16] = {
    0,0,0,
    0b0010011001110000,  // row 3: eyes
    0,0,0,0,0,0,0,0,0,0,0,0,
};

static const uint16_t SPRITE_CAT_DARK[16] = {
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0b0000110011000000,  // row 15: paws
};
static const uint16_t SPRITE_CAT_ACCENT[16] = {
    0,0,0,
    0b0010011001100000,  // row 3: eyes
    0,0,0,0,0,0,0,0,0,0,0,0,
};

static const uint16_t SPRITE_ROBOT_DARK[16] = {
    0,0,0,0,
    0b0011000000100000,  // row 4: screen bg
    0b0011000000100000,  // row 5: screen bg (eyes are accent)
    0b0011000000100000,  // row 6: screen bg
    0b0011000000100000,  // row 7: screen bg (mouth is accent)
    0,0,0,0,0,0,0,
    0b0011100011100000,  // row 15: feet
};
static const uint16_t SPRITE_ROBOT_ACCENT[16] = {
    0b0000001000000000,  // row 0: antenna tip
    0b0000001110000000,  // row 1: antenna
    0,0,0,
    0b0011010010100000,  // row 5: eyes
    0,
    0b0011011110100000,  // row 7: mouth
    0,0,0,
    0b0011111111100000,  // row 11: arms glow
    0,0,0,0,
};

static const uint16_t SPRITE_DRAGON_DARK[16] = {
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0b0001110111000000,  // row 15: claws
};
static const uint16_t SPRITE_DRAGON_ACCENT[16] = {
    0b0010000001000000,  // row 0: horns
    0,0,
    0b0001011011000000,  // row 3: eyes
    0,0,0,0,0,0,0,0,0,
    0b0000000000001000,  // row 13: tail tip
    0,0,
};

static const uint16_t* PET_DARK_MASKS[] = {
    SPRITE_DOG_DARK, SPRITE_CAT_DARK, SPRITE_ROBOT_DARK, SPRITE_DRAGON_DARK
};
static const uint16_t* PET_ACCENT_MASKS[] = {
    SPRITE_DOG_ACCENT, SPRITE_CAT_ACCENT, SPRITE_ROBOT_ACCENT, SPRITE_DRAGON_ACCENT
};

static void drawSprite2x(DisplayCanvas* d, int petIdx, int16_t x, int16_t y) {
    const uint16_t* sprite = PET_SPRITES[petIdx];
    const uint16_t* darkMask = PET_DARK_MASKS[petIdx];
    const uint16_t* accentMask = PET_ACCENT_MASKS[petIdx];
    uint16_t bodyClr = PET_BODY_COLOR[petIdx];
    uint16_t darkClr = PET_DARK_COLOR[petIdx];
    uint16_t accentClr = PET_ACCENT_COLOR[petIdx];

    for (int row = 0; row < 16; row++) {
        uint16_t bits = sprite[row];
        uint16_t dark = darkMask[row];
        uint16_t accent = accentMask[row];
        for (int col = 0; col < 16; col++) {
            uint16_t mask = 0x8000 >> col;
            if (bits & mask) {
                uint16_t clr = bodyClr;
                if (accent & mask) clr = accentClr;
                else if (dark & mask) clr = darkClr;
                d->fillRect(x + col * 2, y + row * 2, 2, 2, clr);
            }
        }
    }
}

// 3x-scaled sprite with rotation for fullscreen pet mode
static void drawSprite3x(DisplayCanvas* d, int petIdx, int16_t cx, int16_t cy, float rot, float scale) {
    const uint16_t* sprite = PET_SPRITES[petIdx];
    const uint16_t* darkMask = PET_DARK_MASKS[petIdx];
    const uint16_t* accentMask = PET_ACCENT_MASKS[petIdx];
    uint16_t bodyClr = PET_BODY_COLOR[petIdx];
    uint16_t darkClr = PET_DARK_COLOR[petIdx];
    uint16_t accentClr = PET_ACCENT_COLOR[petIdx];

    int pxSz = (int)(3 * scale);
    if (pxSz < 1) pxSz = 1;
    int halfW = 8 * pxSz;

    // Simple rotation: 0/90/180/270 snap for pixel art
    int rotSnap = ((int)(rot + 45) / 90) % 4;
    if (rotSnap < 0) rotSnap += 4;

    for (int row = 0; row < 16; row++) {
        uint16_t bits = sprite[row];
        uint16_t dark = darkMask[row];
        uint16_t accent = accentMask[row];
        for (int col = 0; col < 16; col++) {
            uint16_t mask = 0x8000 >> col;
            if (bits & mask) {
                uint16_t clr = bodyClr;
                if (accent & mask) clr = accentClr;
                else if (dark & mask) clr = darkClr;

                int px, py;
                switch (rotSnap) {
                    case 0: px = col; py = row; break;
                    case 1: px = 15 - row; py = col; break;
                    case 2: px = 15 - col; py = 15 - row; break;
                    case 3: px = row; py = 15 - col; break;
                    default: px = col; py = row; break;
                }
                d->fillRect(cx - halfW + px * pxSz, cy - halfW + py * pxSz, pxSz, pxSz, clr);
            }
        }
    }
}

// Animation state
static float petBounce = 0.0f;

static void drawPetOverlay(DisplayCanvas* d) {
    PetData& pet = HookbotServer::getPetData();

    // Bounce animation (gentle bob, livelier when happy)
    petBounce += 0.05f;
    float bounceAmt = sinf(petBounce) * (pet.happiness > 50 ? 2.0f : 0.8f);

    // Draw pet sprite at 2x scale (32x32 px) in bottom-right
    int petIdx = (int)pet.activePet;
    if (petIdx < 0 || petIdx >= 4) petIdx = 0;

    int16_t px = 84;
    int16_t py = 82 + (int16_t)bounceAmt;

    drawSprite2x(d, petIdx, px, py);

    // Colored stat bars above pet
    int16_t bx = 86;
    int16_t bw = 30;
    int16_t bh = 3;

    // Hunger bar (amber/orange)
    uint16_t hungerClr = 0xFCA0;  // RGB565 amber
    int16_t hy = py - 9;
    int16_t hFill = (int16_t)((float)pet.hunger / 100.0f * (bw - 2));
    d->drawRect(bx, hy, bw, bh, 0x4208);  // dark grey outline
    if (hFill > 0) d->fillRect(bx + 1, hy + 1, hFill, bh - 2, hungerClr);

    // Happiness bar (pink)
    uint16_t happyClr = 0xF8B2;  // RGB565 pink
    int16_t jy = hy + bh + 1;
    int16_t jFill = (int16_t)((float)pet.happiness / 100.0f * (bw - 2));
    d->drawRect(bx, jy, bw, bh, 0x4208);  // dark grey outline
    if (jFill > 0) d->fillRect(bx + 1, jy + 1, jFill, bh - 2, happyClr);
}

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

    // Top center: down arrow hint (pomodoro)
    if (pulse > 0.3f) {
        int16_t cx = 60;
        d->drawPixel(cx, 2, COLOR_WHITE);
        d->drawPixel(cx - 1, 1, COLOR_WHITE);
        d->drawPixel(cx + 1, 1, COLOR_WHITE);
        d->drawPixel(cx - 2, 0, COLOR_WHITE);
        d->drawPixel(cx + 2, 0, COLOR_WHITE);
    }
}

// ─── Public API ─────────────────────────────────────────────────

void draw() {
    DisplayCanvas* d = Display::getCanvas();

    // Pomodoro completion flash overlay
    if (pomoFlashActive) {
        uint32_t elapsed = millis() - pomoFlashStart;
        float phase = (float)elapsed / 250.0f; // flash frequency
        bool bright = ((int)phase % 2) == 0;

        // Fade out over time
        float fade = 1.0f - (float)elapsed / (float)POMO_FLASH_DURATION;
        if (fade < 0) fade = 0;

        if (bright && fade > 0.1f) {
            // Flash the border with session color
            PomodoroData& pomo = HookbotServer::getPomodoro();
            uint16_t clr = 0x631F;
            if (pomo.session == PomodoroSession::SHORT_BREAK) clr = 0x4726;
            if (pomo.session == PomodoroSession::LONG_BREAK) clr = 0x0EBE;
            // Draw flashing border
            for (int i = 0; i < 3; i++) {
                d->drawRect(i, i, 120 - i * 2, 120 - i * 2, clr);
            }
        }

        // "Done!" text
        d->setTextSize(2);
        d->setTextColor(COLOR_WHITE);
        d->setCursor(28, 52);
        d->print("Done!");
        d->setTextSize(1);
        d->setTextColor(0x8410);
        d->setCursor(30, 72);
        d->print("Tap to dismiss");
    }

    // Fullscreen pet mode takes over everything
    if (activePanel == Panel::PET_FULLSCREEN) {
        drawPetFullscreen(d);
        return;
    }

    drawPetOverlay(d);

    // Persistent pomodoro timer overlay (when running/paused and panel not open)
    {
        PomodoroData& pomo = HookbotServer::getPomodoro();
        if (pomo.status != PomodoroStatus::IDLE && activePanel != Panel::POMODORO) {
            uint16_t clr = 0x631F;
            if (pomo.session == PomodoroSession::SHORT_BREAK) clr = 0x4726;
            if (pomo.session == PomodoroSession::LONG_BREAK) clr = 0x0EBE;

            int mins = pomo.timeLeftSec / 60;
            int secs = pomo.timeLeftSec % 60;
            char buf[8];
            snprintf(buf, sizeof(buf), "%02d:%02d", mins, secs);

            // Background pill top-center
            d->fillRoundRect(35, 1, 50, 11, 4, 0x0000);
            d->drawRoundRect(35, 1, 50, 11, 4, clr);
            d->setTextSize(1);
            d->setTextColor(clr);
            d->setCursor(40, 3);
            d->print(buf);

            // Pause indicator
            if (pomo.status == PomodoroStatus::PAUSED) {
                d->fillRect(82, 3, 2, 7, 0xFD20);
                d->fillRect(85, 3, 2, 7, 0xFD20);
            }
        }
    }

    drawSwipeHints(d);

    if (activePanel == Panel::NONE && slideProgress <= 0.0f) return;

    switch (activePanel) {
        case Panel::SETTINGS:     drawSettingsPanel(d); break;
        case Panel::ACCESSORIES:  drawAccessoriesPanel(d); break;
        case Panel::STATE_SELECT: drawStatePanel(d); break;
        case Panel::POMODORO:     drawPomodoroPanel(d); break;
        default: break;
    }
}

bool isOverlayActive() {
    return activePanel != Panel::NONE || slideProgress > 0.0f;
}

} // namespace TouchUI

#endif // BOARD_ESP32_4848S040C
