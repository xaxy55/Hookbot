#pragma once

#include <Arduino.h>

// QR code generator and display for OLED/LCD.
// Used to display claim codes as scannable QR codes.
namespace QRDisplay {
    /// Generate and draw a QR code centered on the display.
    /// text: the string to encode (e.g. "hookbot://claim/AB3X9K")
    /// Returns true if drawn successfully.
    bool draw(const char* text);
}
