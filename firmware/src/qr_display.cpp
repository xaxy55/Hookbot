#include "qr_display.h"
#include "display.h"
#include "config.h"
#include <qrcode.h>

namespace QRDisplay {

bool draw(const char* text) {
    if (!text || strlen(text) == 0) return false;

    // QR code version 3 = 29x29 modules, can hold ~35 alphanumeric chars
    // Version 2 = 25x25, can hold ~20 alphanumeric chars
    // "hookbot://claim/XXXXXX" = 22 chars → version 3 fits
    const uint8_t qrVersion = 3;
    const uint8_t qrSize = 4 * qrVersion + 17;  // 29 for v3

    QRCode qrcode;
    uint8_t qrcodeData[qrcode_getBufferSize(qrVersion)];

    if (qrcode_initText(&qrcode, qrcodeData, qrVersion, ECC_LOW, text) != 0) {
        Serial.printf("[QR] Failed to generate QR code for: %s\n", text);
        return false;
    }

    DisplayCanvas* d = Display::getCanvas();

    // Calculate pixel size to fit the screen with some margin
    int16_t screenMin = min(SCREEN_WIDTH, SCREEN_HEIGHT);
    int16_t margin = 4;
    int16_t available = screenMin - margin * 2;
    int16_t pixelSize = available / qrSize;
    if (pixelSize < 1) pixelSize = 1;

    // Center the QR code
    int16_t totalSize = qrSize * pixelSize;
    int16_t offsetX = (SCREEN_WIDTH - totalSize) / 2;
    int16_t offsetY = (SCREEN_HEIGHT - totalSize) / 2;

    // Draw white background (quiet zone)
    d->fillRect(offsetX - 2, offsetY - 2, totalSize + 4, totalSize + 4, COLOR_WHITE);

    // Draw QR modules
    for (uint8_t y = 0; y < qrSize; y++) {
        for (uint8_t x = 0; x < qrSize; x++) {
            if (qrcode_getModule(&qrcode, x, y)) {
                d->fillRect(
                    offsetX + x * pixelSize,
                    offsetY + y * pixelSize,
                    pixelSize, pixelSize,
                    COLOR_BLACK
                );
            }
        }
    }

    return true;
}

} // namespace QRDisplay
