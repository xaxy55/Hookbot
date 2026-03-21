#include "boot_anim.h"
#include "display.h"
#include "config.h"

namespace BootAnim {

// ─── Classic: CEO boot sequence ─────────────────────────────────
static void playClassic() {
    DisplayCanvas* canvas = Display::getCanvas();
    int16_t w = Display::width();
    int16_t h = Display::height();

    const char* title = "HOOKBOT";
    int titleLen = strlen(title);

    // Phase 1: Type out "HOOKBOT" letter by letter
    for (int i = 1; i <= titleLen; i++) {
        Display::clear();
        canvas->setTextSize(2);
        canvas->setTextColor(COLOR_WHITE);
        // Center the text horizontally (each char ~12px wide at size 2)
        int16_t tx = (w - titleLen * 12) / 2;
        int16_t ty = h / 2 - 16;
        canvas->setCursor(tx, ty);
        for (int c = 0; c < i; c++) {
            canvas->print(title[c]);
        }
        Display::flush();
        delay(120);
    }

    delay(300);

    // Phase 2: "CEO MODE ACTIVATED" appears below
    const char* subtitle = "CEO MODE";
    const char* subtitle2 = "ACTIVATED";
    for (int step = 0; step < 2; step++) {
        Display::clear();
        canvas->setTextSize(2);
        canvas->setTextColor(COLOR_WHITE);
        int16_t tx = (w - titleLen * 12) / 2;
        int16_t ty = h / 2 - 16;
        canvas->setCursor(tx, ty);
        canvas->print(title);

        canvas->setTextSize(1);
        if (step >= 0) {
            int16_t sx = (w - strlen(subtitle) * 6) / 2;
            canvas->setCursor(sx, h / 2 + 4);
            canvas->print(subtitle);
        }
        if (step >= 1) {
            int16_t sx2 = (w - strlen(subtitle2) * 6) / 2;
            canvas->setCursor(sx2, h / 2 + 14);
            canvas->print(subtitle2);
        }
        Display::flush();
        delay(400);
    }

    delay(200);

    // Phase 3: Brief flash
    canvas->fillRect(0, 0, w, h, COLOR_WHITE);
    Display::flush();
    delay(80);
    Display::clear();
    Display::flush();
    delay(150);
}

// ─── Matrix: falling characters ─────────────────────────────────
static void playMatrix() {
    DisplayCanvas* canvas = Display::getCanvas();
    int16_t w = Display::width();
    int16_t h = Display::height();

    const int cols = w / 6;  // character width at size 1
    const int rows = h / 8;  // character height at size 1

    // Track the head position of each column's falling stream
    int headRow[22];  // max cols
    int colCount = min(cols, 22);
    for (int c = 0; c < colCount; c++) {
        headRow[c] = -(random(0, rows));  // stagger starts
    }

    uint32_t start = millis();
    while (millis() - start < 2000) {
        Display::clear();
        canvas->setTextSize(1);
        canvas->setTextColor(COLOR_WHITE);

        for (int c = 0; c < colCount; c++) {
            // Draw a trail of characters behind the head
            int trail = 4 + random(0, 3);
            for (int t = 0; t < trail; t++) {
                int row = headRow[c] - t;
                if (row >= 0 && row < rows) {
                    char ch = (char)(33 + random(0, 94));  // printable ASCII
                    canvas->setCursor(c * 6, row * 8);
                    canvas->print(ch);
                }
            }
            headRow[c]++;
            if (headRow[c] - 6 > rows) {
                headRow[c] = -(random(0, rows / 2));
            }
        }
        Display::flush();
        delay(60);
    }

    Display::clear();
    Display::flush();
}

// ─── Glitch: CRT power-on effect ───────────────────────────────
static void playGlitch() {
    DisplayCanvas* canvas = Display::getCanvas();
    int16_t w = Display::width();
    int16_t h = Display::height();

    uint32_t start = millis();
    int phase = 0;

    while (millis() - start < 1500) {
        uint32_t elapsed = millis() - start;
        Display::clear();

        if (elapsed < 300) {
            // Phase: horizontal scan line expanding from center
            int lineH = (elapsed * h) / 600;  // grows to half height
            int y0 = h / 2 - lineH / 2;
            canvas->fillRect(0, y0, w, max(1, lineH), COLOR_WHITE);
        } else if (elapsed < 800) {
            // Phase: random static bursts
            int density = 80 + random(0, 120);
            for (int i = 0; i < density; i++) {
                int px = random(0, w);
                int py = random(0, h);
                canvas->drawPixel(px, py, COLOR_WHITE);
            }
            // Occasional horizontal glitch bars
            if (random(0, 3) == 0) {
                int barY = random(0, h);
                int barH = 1 + random(0, 4);
                canvas->fillRect(0, barY, w, barH, COLOR_WHITE);
            }
        } else {
            // Phase: settling - show garbled text fragments, flickering
            bool show = (random(0, 4) > 0);  // 75% chance to show
            if (show) {
                canvas->setTextSize(2);
                canvas->setTextColor(COLOR_WHITE);
                int16_t tx = (w - 7 * 12) / 2 + random(-3, 4);
                int16_t ty = h / 2 - 8 + random(-2, 3);
                canvas->setCursor(tx, ty);
                // Garble some characters
                const char* text = "HOOKBOT";
                for (int i = 0; i < 7; i++) {
                    if (random(0, 5) == 0 && elapsed < 1300) {
                        canvas->print((char)(33 + random(0, 94)));
                    } else {
                        canvas->print(text[i]);
                    }
                }
            }
        }

        Display::flush();
        delay(40);
    }

    // Final clean frame
    Display::clear();
    Display::flush();
    delay(100);
}

void play(int type) {
    switch (type) {
        case 0:  // none
            break;
        case 1:  // classic
            playClassic();
            break;
        case 2:  // matrix
            playMatrix();
            break;
        case 3:  // glitch
            playGlitch();
            break;
        default:
            playClassic();
            break;
    }
}

} // namespace BootAnim
