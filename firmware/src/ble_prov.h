#pragma once

#include <Arduino.h>

namespace BleProv {
    void init();
    void update();          // Call from loop - manages BLE start/stop based on WiFi
    bool isAdvertising();   // True when BLE is active and waiting for connection
    void refreshClaimInfo(); // Update the claim info characteristic (call after claim status changes)
}
