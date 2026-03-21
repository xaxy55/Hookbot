#pragma once

#include "avatar.h"
#include <functional>

// Cloud client for hosted mode: device connects outbound to public server.
// Handles registration, heartbeat push, command polling, and command dispatch.

namespace CloudClient {
    /// Initialize cloud client with state change callback.
    void init(std::function<void(AvatarState)> onStateChange);

    /// Called from main loop. Handles heartbeat, command polling, registration.
    void update();

    /// Whether the device has a cloud server configured.
    bool isEnabled();

    /// Whether the device has been claimed by a user.
    bool isClaimed();

    /// Get the current claim code (empty if claimed).
    const char* getClaimCode();

    /// Get the device token (empty if not registered).
    const char* getDeviceToken();
}
