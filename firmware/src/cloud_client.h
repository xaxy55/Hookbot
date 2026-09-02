#pragma once

#include "avatar.h"
#include <functional>

// Cloud client for hosted mode: device connects outbound to public server.
// Handles registration, heartbeat push, command polling, and command dispatch.

namespace CloudClient {
    /// Initialize cloud client with state change callback.
    void init(std::function<void(AvatarState)> onStateChange);

    /// Runs on its own FreeRTOS task, started by init(). Not called from the
    /// main loop: registration, heartbeat and command polling are synchronous
    /// HTTPS calls, and a TLS handshake on this chip costs seconds. Driving
    /// them from the render loop froze the display for ~5s at a time.

    /// Whether the device has a cloud server configured.
    bool isEnabled();

    /// Whether the device has been claimed by a user.
    bool isClaimed();

    /// Get the current claim code (empty if claimed).
    const char* getClaimCode();

    /// Get the device token (empty if not registered).
    const char* getDeviceToken();

    /// Reset cloud state: clears device token, claim code, claimed status.
    /// Device will re-register on next heartbeat cycle.
    void resetCloud();
}
