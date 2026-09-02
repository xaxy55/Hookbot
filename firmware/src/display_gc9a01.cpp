#ifdef BOARD_XIAO_C6_GC9A01  // 240x240 GC9A01 round SPI panel, no touch

#define LGFX_USE_V1
#include "display.h"

// ─── LovyanGFX Configuration for XIAO ESP32-C6 + GC9A01 (1.28" 240x240) ─
// Wiring is declared in config.h (TFT_* defines). The panel is write-only
// 4-wire SPI on SPI2_HOST, the only general-purpose SPI host on the C6.

class LGFX : public lgfx::LGFX_Device {
public:
    lgfx::Bus_SPI      _bus_instance;
    lgfx::Panel_GC9A01 _panel_instance;
    lgfx::Light_PWM    _light_instance;

    LGFX(void) {
        // ── SPI bus ──
        {
            auto cfg = _bus_instance.config();
            cfg.spi_host   = SPI2_HOST;
            cfg.spi_mode   = 0;
            // 40MHz, not 80. TFT_SCK/TFT_MOSI are not the C6's IO_MUX pins for
            // SPI2, so the signals route through the GPIO matrix, which caps
            // reliable SPI at ~40MHz. At 80MHz the panel dropped the high byte
            // of each RGB565 pixel, so white (0xFFFF) arrived as 0x00FF — the
            // whole avatar rendered blue. The GC9A01 die is rated higher, but
            // the routing between the C6 and it is not.
            cfg.freq_write = 40000000;
            cfg.freq_read  = 16000000;
            cfg.spi_3wire  = true;   // no MISO line on these modules
            cfg.use_lock   = true;
            cfg.dma_channel = SPI_DMA_CH_AUTO;
            cfg.pin_sclk = TFT_SCK;
            cfg.pin_mosi = TFT_MOSI;
            cfg.pin_miso = -1;
            cfg.pin_dc   = TFT_DC;
            _bus_instance.config(cfg);
        }
        _panel_instance.setBus(&_bus_instance);

        // ── Panel ──
        {
            auto cfg = _panel_instance.config();
            cfg.pin_cs   = TFT_CS;
            cfg.pin_rst  = TFT_RST;
            cfg.pin_busy = -1;

            cfg.memory_width  = LCD_PHYS_WIDTH;
            cfg.memory_height = LCD_PHYS_HEIGHT;
            cfg.panel_width   = LCD_PHYS_WIDTH;
            cfg.panel_height  = LCD_PHYS_HEIGHT;
            cfg.offset_x = 0;
            cfg.offset_y = 0;
            cfg.offset_rotation = 0;

            cfg.dummy_read_pixel = 8;
            cfg.dummy_read_bits  = 1;
            cfg.readable   = false;  // MISO not wired
            cfg.invert     = true;   // GC9A01 ships with inverted colours
            cfg.rgb_order  = false;
            cfg.dlen_16bit = false;
            cfg.bus_shared = false;
            _panel_instance.config(cfg);
        }

        // ── Backlight (PWM on TFT_BL) ──
        {
            auto cfg = _light_instance.config();
            cfg.pin_bl = TFT_BL;
            cfg.invert = false;
            cfg.freq   = 44100;
            cfg.pwm_channel = 0;  // C6 LEDC exposes channels 0-5
            _light_instance.config(cfg);
        }
        _panel_instance.light(&_light_instance);

        setPanel(&_panel_instance);
    }
};

// ─── Display Namespace Implementation ────────────────────────────

namespace Display {

static LGFX* lcd = nullptr;
static lgfx::LGFX_Sprite* canvas = nullptr;

void init() {
    lcd = new LGFX();
    lcd->init();
    lcd->setRotation(0);
    lcd->setBrightness(255);  // Full brightness initially, server config applied later
    lcd->fillScreen(0);

    // Virtual canvas at 120x120, scaled 2x to fill 240x240
    canvas = new lgfx::LGFX_Sprite(lcd);
    canvas->setColorDepth(16);
    void* buf = canvas->createSprite(SCREEN_WIDTH, SCREEN_HEIGHT);
    canvas->fillSprite(0);

    Serial.printf("[Display] sprite buf=%p depth=%d heap=%u\n",
                  buf, (int)canvas->getColorDepth(), (unsigned)ESP.getFreeHeap());

#ifdef DISPLAY_COLOR_TEST
    // Build with -DDISPLAY_COLOR_TEST to prove the colour path end to end:
    // four labelled bars drawn straight to the panel, bypassing the sprite.
    // If these read R/G/B/W top to bottom, the panel, the SPI clock and the
    // channel order are all correct and any colour bug is in the drawing code.
    {
        const uint16_t bars[4]  = { 0xF800, 0x07E0, 0x001F, 0xFFFF };
        const char*    names[4] = { "RED", "GREEN", "BLUE", "WHITE" };
        const int      h        = LCD_PHYS_HEIGHT / 4;
        for (int i = 0; i < 4; i++) {
            lcd->fillRect(0, h * i, LCD_PHYS_WIDTH, h, bars[i]);
            lcd->setTextColor(0x0000);
            lcd->setTextSize(2);
            lcd->setCursor(70, h * i + h / 2 - 8);
            lcd->print(names[i]);
        }
        Serial.println("[Display] COLOUR TEST phase A: direct-to-panel R/G/B/W");
        delay(12000);

        // Phase B: the identical bars, but drawn into the sprite and pushed
        // through pushRotateZoom — the exact path the avatar uses. Direct
        // drawing and the sprite contents are both known good, so if these
        // bars differ from phase A, the push is where colour is being lost.
        canvas->fillSprite(0);
        const int sh = SCREEN_HEIGHT / 4;
        for (int i = 0; i < 4; i++) {
            canvas->fillRect(0, sh * i, SCREEN_WIDTH, sh, bars[i]);
            canvas->setTextColor(0x0000);
            canvas->setTextSize(1);
            canvas->setCursor(34, sh * i + sh / 2 - 4);
            canvas->print(names[i]);
        }
        canvas->pushRotateZoom(LCD_PHYS_WIDTH / 2, LCD_PHYS_HEIGHT / 2,
                               0, (float)LCD_SCALE, (float)LCD_SCALE);
        Serial.println("[Display] COLOUR TEST phase B: same bars via the sprite");
        delay(18000);
        lcd->fillScreen(0);
    }
#endif

    Serial.println("[Display] XIAO ESP32-C6 + GC9A01 240x240 round LCD initialized");
    Serial.printf("[Display] Virtual canvas: %dx%d (%dx scale, round safe radius %d)\n",
                  SCREEN_WIDTH, SCREEN_HEIGHT, LCD_SCALE, DISPLAY_SAFE_RADIUS);
}

void clear() {
    canvas->fillSprite(0);
}

void flush() {
    canvas->pushRotateZoom(LCD_PHYS_WIDTH / 2, LCD_PHYS_HEIGHT / 2,
                           0, (float)LCD_SCALE, (float)LCD_SCALE);
}

DisplayCanvas* getCanvas() {
    return canvas;
}

int16_t width()   { return SCREEN_WIDTH; }
int16_t height()  { return SCREEN_HEIGHT; }
int16_t centerX() { return SCREEN_WIDTH / 2; }
int16_t centerY() { return SCREEN_HEIGHT / 2; }

void setBrightness(uint8_t level) {
    if (lcd) {
        lcd->setBrightness(level);
    }
}

} // namespace Display

#endif // BOARD_XIAO_C6_GC9A01
