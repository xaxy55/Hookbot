#ifdef BOARD_ESP32_4848S040C

#define LGFX_USE_V1
#include "display.h"
#include <lgfx/v1/platforms/esp32s3/Panel_RGB.hpp>
#include <lgfx/v1/platforms/esp32s3/Bus_RGB.hpp>

// ─── LovyanGFX Configuration for ESP32-4848S040C (4.0" 480x480) ─
// Pin mapping from: https://homeding.github.io/boards/esp32s3/panel-4848S040.htm

class LGFX : public lgfx::LGFX_Device {
public:
    lgfx::Bus_RGB      _bus_instance;
    lgfx::Panel_ST7701_guition_esp32_4848S040 _panel_instance;
    lgfx::Light_PWM    _light_instance;
    lgfx::Touch_GT911  _touch_instance;

    LGFX(void) {
        // ── Panel ──
        {
            auto cfg = _panel_instance.config();
            cfg.memory_width  = 480;
            cfg.memory_height = 480;
            cfg.panel_width   = 480;
            cfg.panel_height  = 480;
            cfg.offset_x = 0;
            cfg.offset_y = 0;
            _panel_instance.config(cfg);
        }

        // ── SPI init pins for ST7701S (9-bit software SPI) ──
        {
            auto cfg = _panel_instance.config_detail();
            cfg.use_psram = 1;
            cfg.pin_cs   = 39;
            cfg.pin_sclk = 48;
            cfg.pin_mosi = 47;
            _panel_instance.config_detail(cfg);
        }

        // ── RGB Bus ──
        {
            auto cfg = _bus_instance.config();
            cfg.panel = &_panel_instance;

            cfg.pin_d0  = GPIO_NUM_4;   // B0
            cfg.pin_d1  = GPIO_NUM_5;   // B1
            cfg.pin_d2  = GPIO_NUM_6;   // B2
            cfg.pin_d3  = GPIO_NUM_7;   // B3
            cfg.pin_d4  = GPIO_NUM_15;  // B4
            cfg.pin_d5  = GPIO_NUM_8;   // G0
            cfg.pin_d6  = GPIO_NUM_20;  // G1
            cfg.pin_d7  = GPIO_NUM_3;   // G2
            cfg.pin_d8  = GPIO_NUM_46;  // G3
            cfg.pin_d9  = GPIO_NUM_9;   // G4
            cfg.pin_d10 = GPIO_NUM_10;  // G5
            cfg.pin_d11 = GPIO_NUM_11;  // R0
            cfg.pin_d12 = GPIO_NUM_12;  // R1
            cfg.pin_d13 = GPIO_NUM_13;  // R2
            cfg.pin_d14 = GPIO_NUM_14;  // R3
            cfg.pin_d15 = GPIO_NUM_0;   // R4

            cfg.pin_henable = GPIO_NUM_18;  // DE
            cfg.pin_vsync   = GPIO_NUM_17;
            cfg.pin_hsync   = GPIO_NUM_16;
            cfg.pin_pclk    = GPIO_NUM_21;

            cfg.freq_write = 14000000;

            cfg.hsync_polarity    = 1;
            cfg.hsync_front_porch = 10;
            cfg.hsync_pulse_width = 8;
            cfg.hsync_back_porch  = 50;
            cfg.vsync_polarity    = 1;
            cfg.vsync_front_porch = 10;
            cfg.vsync_pulse_width = 8;
            cfg.vsync_back_porch  = 20;
            cfg.pclk_idle_high    = 1;
            cfg.de_idle_high      = 1;

            _bus_instance.config(cfg);
        }
        _panel_instance.setBus(&_bus_instance);

        // ── Backlight (GPIO 38) ──
        {
            auto cfg = _light_instance.config();
            cfg.pin_bl = GPIO_NUM_38;
            cfg.invert = false;
            cfg.freq   = 44100;
            cfg.pwm_channel = 7;
            _light_instance.config(cfg);
        }
        _panel_instance.light(&_light_instance);

        // ── Touch (GT911) ──
        {
            auto cfg = _touch_instance.config();
            cfg.x_min = 0;
            cfg.x_max = 479;
            cfg.y_min = 0;
            cfg.y_max = 479;
            cfg.pin_int  = -1;
            cfg.pin_rst  = -1;
            cfg.bus_shared = true;  // GPIO 20 shared between touch SCL and RGB G1
            cfg.offset_rotation = 0;
            cfg.i2c_port = 1;
            cfg.i2c_addr = 0x5D;
            cfg.pin_sda  = GPIO_NUM_19;
            cfg.pin_scl  = GPIO_NUM_20;
            cfg.freq     = 400000;
            _touch_instance.config(cfg);
            _panel_instance.setTouch(&_touch_instance);
        }

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
    lcd->setBrightness(255);
    lcd->fillScreen(0);

    // Virtual canvas at 120x120, scaled 4x to fill 480x480
    canvas = new lgfx::LGFX_Sprite(lcd);
    canvas->setColorDepth(16);
    canvas->createSprite(SCREEN_WIDTH, SCREEN_HEIGHT);
    canvas->fillSprite(0);

    Serial.println("[Display] ESP32-4848S040C 480x480 LCD initialized");
    Serial.printf("[Display] Virtual canvas: %dx%d (4x scale)\n", SCREEN_WIDTH, SCREEN_HEIGHT);
}

void clear() {
    canvas->fillSprite(0);
}

void flush() {
    canvas->pushRotateZoom(240, 240, 0, 4.0f, 4.0f);
}

DisplayCanvas* getCanvas() {
    return canvas;
}

int16_t width()   { return SCREEN_WIDTH; }
int16_t height()  { return SCREEN_HEIGHT; }
int16_t centerX() { return SCREEN_WIDTH / 2; }
int16_t centerY() { return SCREEN_HEIGHT / 2; }

bool getTouchPoint(int16_t& x, int16_t& y) {
    lgfx::touch_point_t tp;
    if (lcd->getTouch(&tp)) {
        x = tp.x / 4;
        y = tp.y / 4;
        return true;
    }
    return false;
}

} // namespace Display

#endif // BOARD_ESP32_4848S040C
