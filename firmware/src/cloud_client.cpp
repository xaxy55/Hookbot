#include "cloud_client.h"
#include "config.h"
#include "server.h"

#include <Arduino.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include <ArduinoJson.h>
#include <Preferences.h>

namespace CloudClient {

// --- State ---
static std::function<void(AvatarState)> stateCallback;
static Preferences prefs;

static char deviceToken[128] = "";
static char claimCode[8] = "";
static char deviceId[64] = "";
static bool claimed = false;
static bool registered = false;
static bool enabled = false;

// --- Timing ---
static uint32_t lastHeartbeat = 0;
static uint32_t lastCommandPoll = 0;
static uint32_t lastRegisterAttempt = 0;

static const uint32_t HEARTBEAT_INTERVAL_MS    = 5000;   // 5 seconds
static const uint32_t COMMAND_POLL_INTERVAL_MS  = 2000;   // 2 seconds (short poll)
static const uint32_t REGISTER_RETRY_MS         = 15000;  // 15 seconds between retries

// --- Forward declarations ---
static void loadCloudConfig();
static void saveCloudConfig();
static void attemptRegistration();
static void sendHeartbeat();
static void pollCommands();
static void executeCommand(const char* type, JsonObject payload, const char* cmdId);
static AvatarState stringToState(const char* str);

// --- Public API ---

void init(std::function<void(AvatarState)> onStateChange) {
    stateCallback = onStateChange;
    loadCloudConfig();

    // Check if cloud mode is enabled (mgmtServer is set)
    RuntimeConfig& config = HookbotServer::getConfig();
    enabled = (strlen(config.mgmtServer) > 0);

    if (enabled) {
        Serial.printf("[Cloud] Enabled, server: %s\n", config.mgmtServer);
        if (strlen(deviceToken) > 0) {
            registered = true;
            Serial.printf("[Cloud] Registered as %s (claimed: %s)\n",
                deviceId, claimed ? "yes" : "no");
        } else {
            Serial.println("[Cloud] Not yet registered, will attempt on next update");
        }
    }
}

void update() {
    if (!enabled || WiFi.status() != WL_CONNECTED) return;

    uint32_t now = millis();

    // Step 1: Register if not yet registered
    if (!registered) {
        if (now - lastRegisterAttempt >= REGISTER_RETRY_MS || lastRegisterAttempt == 0) {
            lastRegisterAttempt = now;
            attemptRegistration();
        }
        return; // Don't heartbeat or poll until registered
    }

    // Step 2: Send heartbeat
    if (now - lastHeartbeat >= HEARTBEAT_INTERVAL_MS) {
        lastHeartbeat = now;
        sendHeartbeat();
    }

    // Step 3: Poll for commands
    if (now - lastCommandPoll >= COMMAND_POLL_INTERVAL_MS) {
        lastCommandPoll = now;
        pollCommands();
    }
}

bool isEnabled() { return enabled; }
bool isClaimed() { return claimed; }
const char* getClaimCode() { return claimCode; }
const char* getDeviceToken() { return deviceToken; }

// --- Private Implementation ---

static void loadCloudConfig() {
    prefs.begin("cloud", true); // read-only
    String token = prefs.getString("devToken", "");
    strncpy(deviceToken, token.c_str(), sizeof(deviceToken) - 1);
    String code = prefs.getString("claimCode", "");
    strncpy(claimCode, code.c_str(), sizeof(claimCode) - 1);
    String id = prefs.getString("deviceId", "");
    strncpy(deviceId, id.c_str(), sizeof(deviceId) - 1);
    claimed = prefs.getBool("claimed", false);
    prefs.end();
}

static void saveCloudConfig() {
    prefs.begin("cloud", false); // read-write
    prefs.putString("devToken", deviceToken);
    prefs.putString("claimCode", claimCode);
    prefs.putString("deviceId", deviceId);
    prefs.putBool("claimed", claimed);
    prefs.end();
}

static void attemptRegistration() {
    RuntimeConfig& config = HookbotServer::getConfig();

    HTTPClient http;
    String url = String(config.mgmtServer) + "/api/device/register";
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    http.setTimeout(10000);

    // Build registration payload
    JsonDocument doc;
    doc["hostname"] = config.hostname;

    // Get MAC address
    uint8_t mac[6];
    WiFi.macAddress(mac);
    char macStr[18];
    snprintf(macStr, sizeof(macStr), "%02X:%02X:%02X:%02X:%02X:%02X",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
    doc["mac_address"] = macStr;
    doc["firmware_version"] = FIRMWARE_VERSION;

    String body;
    serializeJson(doc, body);

    Serial.printf("[Cloud] Registering with %s\n", url.c_str());
    int code = http.POST(body);

    if (code == 200) {
        String response = http.getString();
        JsonDocument resp;
        if (deserializeJson(resp, response) == DeserializationError::Ok) {
            const char* id = resp["device_id"] | "";
            const char* token = resp["device_token"] | "";
            const char* cc = resp["claim_code"] | "";
            bool isClaimed = resp["claimed"] | false;

            if (strlen(id) > 0 && strlen(token) > 0) {
                strncpy(deviceId, id, sizeof(deviceId) - 1);
                strncpy(deviceToken, token, sizeof(deviceToken) - 1);
                strncpy(claimCode, cc, sizeof(claimCode) - 1);
                claimed = isClaimed;
                registered = true;
                saveCloudConfig();

                Serial.printf("[Cloud] Registered! ID: %s, Claim: %s, Claimed: %s\n",
                    deviceId, claimCode, claimed ? "yes" : "no");
            }
        }
    } else {
        Serial.printf("[Cloud] Registration failed: %d\n", code);
    }
    http.end();
}

static void sendHeartbeat() {
    RuntimeConfig& config = HookbotServer::getConfig();

    HTTPClient http;
    String url = String(config.mgmtServer) + "/api/device/heartbeat";
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    http.addHeader("X-Device-Token", deviceToken);
    http.setTimeout(5000);

    // Build status payload (same as /status endpoint response)
    JsonDocument doc;
    // Get current avatar state name from the namespace
    // We'll use a simple approach: store last known state
    doc["state"] = "idle"; // Will be updated when state changes
    doc["uptime"] = millis();
    doc["freeHeap"] = (int)ESP.getFreeHeap();
    doc["ip"] = WiFi.localIP().toString();
    doc["firmware_version"] = FIRMWARE_VERSION;

    String body;
    serializeJson(doc, body);

    int code = http.POST(body);
    if (code == 200) {
        String response = http.getString();
        JsonDocument resp;
        if (deserializeJson(resp, response) == DeserializationError::Ok) {
            bool wasClaimed = claimed;
            claimed = resp["claimed"] | claimed;

            // If just claimed, update NVS and clear claim code
            if (claimed && !wasClaimed) {
                claimCode[0] = '\0';
                saveCloudConfig();
                Serial.println("[Cloud] Device was claimed by a user!");
            }
        }
    } else if (code == 401) {
        // Token invalid — re-register
        Serial.println("[Cloud] Token expired, re-registering...");
        registered = false;
        deviceToken[0] = '\0';
        saveCloudConfig();
    }
    http.end();
}

static void pollCommands() {
    RuntimeConfig& config = HookbotServer::getConfig();

    HTTPClient http;
    String url = String(config.mgmtServer) + "/api/device/commands?wait=0";
    http.begin(url);
    http.addHeader("X-Device-Token", deviceToken);
    http.setTimeout(5000);

    int code = http.GET();
    if (code == 200) {
        String response = http.getString();
        JsonDocument doc;
        if (deserializeJson(doc, response) == DeserializationError::Ok) {
            JsonArray commands = doc["commands"].as<JsonArray>();
            for (JsonObject cmd : commands) {
                const char* type = cmd["type"] | "";
                const char* cmdId = cmd["id"] | "";
                JsonObject payload = cmd["payload"].as<JsonObject>();

                Serial.printf("[Cloud] Command: %s (id: %s)\n", type, cmdId);
                executeCommand(type, payload, cmdId);

                // Acknowledge command
                if (strlen(cmdId) > 0) {
                    HTTPClient ackHttp;
                    String ackUrl = String(config.mgmtServer) +
                        "/api/device/commands/" + String(cmdId) + "/ack";
                    ackHttp.begin(ackUrl);
                    ackHttp.addHeader("X-Device-Token", deviceToken);
                    ackHttp.addHeader("Content-Type", "application/json");
                    ackHttp.POST("{\"success\":true}");
                    ackHttp.end();
                }
            }
        }
    }
    http.end();
}

static AvatarState stringToState(const char* str) {
    if (strcmp(str, "idle") == 0) return AvatarState::IDLE;
    if (strcmp(str, "thinking") == 0) return AvatarState::THINKING;
    if (strcmp(str, "waiting") == 0) return AvatarState::WAITING;
    if (strcmp(str, "success") == 0) return AvatarState::SUCCESS;
    if (strcmp(str, "taskcheck") == 0) return AvatarState::TASKCHECK;
    if (strcmp(str, "error") == 0) return AvatarState::ERROR;
    return AvatarState::IDLE;
}

static void executeCommand(const char* type, JsonObject payload, const char* cmdId) {
    if (strcmp(type, "state_change") == 0) {
        // Change avatar state
        const char* state = payload["state"] | "idle";
        AvatarState newState = stringToState(state);
        if (stateCallback) {
            stateCallback(newState);
        }
        Serial.printf("[Cloud] State changed to: %s\n", state);
    }
    else if (strcmp(type, "config_update") == 0) {
        // Update runtime config — same as POST /config handler
        JsonObject cfg = payload["config"].as<JsonObject>();
        RuntimeConfig& config = HookbotServer::getConfig();

        if (!cfg["led_brightness"].isNull()) {
            config.ledBrightness = cfg["led_brightness"].as<int>();
        }
        if (!cfg["sound_enabled"].isNull()) {
            config.soundEnabled = cfg["sound_enabled"].as<bool>();
        }
        if (!cfg["sound_volume"].isNull()) {
            config.soundVolume = cfg["sound_volume"].as<int>();
        }
        HookbotServer::saveConfigToNVS();
        Serial.println("[Cloud] Config updated from cloud command");
    }
    else if (strcmp(type, "ota") == 0) {
        // OTA firmware update — queue URL for main loop
        const char* url = payload["url"] | "";
        if (strlen(url) > 0) {
            Serial.printf("[Cloud] OTA update queued: %s\n", url);
            // Queue OTA for main loop processing (same as HTTP /ota endpoint)
            HookbotServer::queueOtaUrl(url);
        }
    }
    else if (strcmp(type, "tasks") == 0) {
        // Update task list
        TaskList& tasks = HookbotServer::getTasks();
        tasks.count = 0;

        JsonArray items = payload["items"].as<JsonArray>();
        for (JsonObject item : items) {
            if (tasks.count >= MAX_TASKS) break;
            const char* label = item["label"] | "";
            int status = item["status"] | 0;
            strncpy(tasks.items[tasks.count].label, label, MAX_TASK_LEN - 1);
            tasks.items[tasks.count].label[MAX_TASK_LEN - 1] = '\0';
            tasks.items[tasks.count].status = (uint8_t)status;
            tasks.count++;
        }

        if (!payload["active"].isNull()) {
            tasks.activeIndex = payload["active"].as<int>();
        }
        Serial.printf("[Cloud] Tasks updated: %d items\n", tasks.count);
    }
    else if (strcmp(type, "servo") == 0) {
        // Servo control — forward to servo subsystem
        // This would need servo.h integration
        Serial.printf("[Cloud] Servo command (not yet dispatched)\n");
    }
    else if (strcmp(type, "animation") == 0) {
        // Play animation — would need AnimPlayer integration
        Serial.printf("[Cloud] Animation command (not yet dispatched)\n");
    }
    else if (strcmp(type, "animation_stop") == 0) {
        Serial.printf("[Cloud] Animation stop (not yet dispatched)\n");
    }
    else {
        Serial.printf("[Cloud] Unknown command type: %s\n", type);
    }
}

} // namespace CloudClient
