#include "ble_prov.h"
#include "server.h"
#include "cloud_client.h"
#include <WiFi.h>
#include <Preferences.h>
#include <BLEDevice.h>
#include <BLEServer.h>
#include <BLEUtils.h>
#include <BLE2902.h>

// BLE UUIDs
#define SERVICE_UUID        "4fafc201-1fb5-459e-8fcc-c5c9c331914b"
#define WIFI_CONFIG_UUID    "beb5483e-36e1-4688-b7f5-ea07361b26a8"  // Write SSID\nPASS
#define STATUS_UUID         "beb5483e-36e1-4688-b7f5-ea07361b26a9"  // Read status
#define CLAIM_INFO_UUID     "beb5483e-36e1-4688-b7f5-ea07361b26aa"  // Read claim info (JSON)

namespace BleProv {

static BLEServer* pServer = nullptr;
static BLECharacteristic* pStatusChar = nullptr;
static BLECharacteristic* pClaimInfoChar = nullptr;
static bool bleActive = false;
static bool deviceConnected = false;
static bool wifiWasConnected = false;
static String pendingSsid;
static String pendingPass;
static bool hasPendingCreds = false;

// Whether BLE should stay active even with WiFi (for unclaimed cloud devices)
static bool needsBlePairing() {
    return CloudClient::isEnabled() && !CloudClient::isClaimed();
}

static void setStatus(const char* msg) {
    if (pStatusChar) {
        pStatusChar->setValue(msg);
        if (deviceConnected) {
            pStatusChar->notify();
        }
    }
    Serial.printf("[BLE] Status: %s\n", msg);
}

static void updateClaimInfo() {
    if (!pClaimInfoChar) return;

    // Build simple JSON with claim info
    String json = "{";
    json += "\"claim_code\":\"";
    json += CloudClient::getClaimCode();
    json += "\",\"claimed\":";
    json += CloudClient::isClaimed() ? "true" : "false";
    json += ",\"cloud\":";
    json += CloudClient::isEnabled() ? "true" : "false";
    json += ",\"wifi\":";
    json += (WiFi.status() == WL_CONNECTED) ? "true" : "false";

    // Include device name
    RuntimeConfig& config = HookbotServer::getConfig();
    json += ",\"name\":\"";
    json += config.hostname;
    json += "\"";

    json += "}";

    pClaimInfoChar->setValue(json.c_str());
    if (deviceConnected) {
        pClaimInfoChar->notify();
    }
}

// Save WiFi credentials to NVS
static bool saveWifiToNVS(const String& ssid, const String& pass) {
    Preferences prefs;
    prefs.begin("wifi", false);

    // Find empty slot
    for (int i = 0; i < 6; i++) {
        char keyS[8], keyP[8];
        snprintf(keyS, sizeof(keyS), "ssid%d", i);
        snprintf(keyP, sizeof(keyP), "pass%d", i);
        String existing = prefs.getString(keyS, "");
        if (existing.length() == 0 || existing == ssid) {
            prefs.putString(keyS, ssid.c_str());
            prefs.putString(keyP, pass.c_str());
            prefs.end();
            Serial.printf("[BLE] WiFi saved to NVS slot %d: %s\n", i, ssid.c_str());
            return true;
        }
    }
    prefs.end();
    return false;
}

class ServerCallbacks : public BLEServerCallbacks {
    void onConnect(BLEServer* s) override {
        deviceConnected = true;
        if (WiFi.status() == WL_CONNECTED) {
            setStatus("Connected. WiFi OK.");
        } else {
            setStatus("Connected. Send: SSID\\nPASSWORD");
        }
        updateClaimInfo();
        Serial.println("[BLE] Client connected");
    }
    void onDisconnect(BLEServer* s) override {
        deviceConnected = false;
        Serial.println("[BLE] Client disconnected");
        // Restart advertising
        if (bleActive) {
            s->getAdvertising()->start();
        }
    }
};

class WifiConfigCallback : public BLECharacteristicCallbacks {
    void onWrite(BLECharacteristic* pChar) override {
        String value = pChar->getValue().c_str();
        if (value.length() == 0) return;

        // Parse: "SSID\nPASSWORD" or "SSID\tPASSWORD"
        int sep = value.indexOf('\n');
        if (sep < 0) sep = value.indexOf('\t');
        if (sep < 0) {
            setStatus("Error: use SSID\\nPASSWORD");
            return;
        }

        pendingSsid = value.substring(0, sep);
        pendingPass = value.substring(sep + 1);
        pendingSsid.trim();
        pendingPass.trim();

        if (pendingSsid.length() == 0) {
            setStatus("Error: empty SSID");
            return;
        }

        Serial.printf("[BLE] Received WiFi: %s\n", pendingSsid.c_str());
        setStatus("Saving...");

        if (saveWifiToNVS(pendingSsid, pendingPass)) {
            hasPendingCreds = true;
            setStatus("Saved! Rebooting...");
        } else {
            setStatus("Error: NVS full");
        }
    }
};

static void startBLE() {
    if (bleActive) return;

    // Build name from MAC
    String name = "Hookbot-";
    String mac = WiFi.macAddress();
    name += mac.substring(mac.length() - 5);
    name.replace(":", "");

    BLEDevice::init(name.c_str());
    pServer = BLEDevice::createServer();
    pServer->setCallbacks(new ServerCallbacks());

    // Use max handles to fit all characteristics (default 15 may not be enough)
    BLEService* pService = pServer->createService(BLEUUID(SERVICE_UUID), 20);

    // WiFi config characteristic (write)
    BLECharacteristic* pWifiChar = pService->createCharacteristic(
        WIFI_CONFIG_UUID,
        BLECharacteristic::PROPERTY_WRITE
    );
    pWifiChar->setCallbacks(new WifiConfigCallback());

    // Status characteristic (read + notify)
    pStatusChar = pService->createCharacteristic(
        STATUS_UUID,
        BLECharacteristic::PROPERTY_READ | BLECharacteristic::PROPERTY_NOTIFY
    );
    pStatusChar->addDescriptor(new BLE2902());
    pStatusChar->setValue("Waiting for WiFi credentials");

    // Claim info characteristic (read + notify) — returns JSON with claim_code, claimed, etc.
    pClaimInfoChar = pService->createCharacteristic(
        CLAIM_INFO_UUID,
        BLECharacteristic::PROPERTY_READ | BLECharacteristic::PROPERTY_NOTIFY
    );
    pClaimInfoChar->addDescriptor(new BLE2902());
    pClaimInfoChar->setValue("{\"claim_code\":\"\",\"claimed\":false,\"cloud\":false}");

    pService->start();

    BLEAdvertising* pAdv = BLEDevice::getAdvertising();
    pAdv->addServiceUUID(SERVICE_UUID);
    pAdv->setScanResponse(true);
    pAdv->start();

    bleActive = true;
    Serial.printf("[BLE] Advertising as: %s\n", name.c_str());
}

static void stopBLE() {
    if (!bleActive) return;
    BLEDevice::getAdvertising()->stop();
    BLEDevice::deinit(false);
    bleActive = false;
    pStatusChar = nullptr;
    pClaimInfoChar = nullptr;
    Serial.println("[BLE] Stopped");
}

void init() {
    // Track current WiFi state so update() detects transitions correctly
    wifiWasConnected = HookbotServer::isConnected();

    // Start BLE if WiFi isn't connected (for provisioning)
    // OR if WiFi is connected but device still needs cloud pairing
    if (!wifiWasConnected || needsBlePairing()) {
        startBLE();
    }
}

void update() {
    bool wifiNow = HookbotServer::isConnected();

    if (wifiNow && !wifiWasConnected) {
        // WiFi just connected — only stop BLE if device doesn't need pairing
        if (!needsBlePairing()) {
            stopBLE();
        } else {
            // WiFi connected but unclaimed: update status and keep BLE for pairing
            setStatus("WiFi OK. Waiting for Bluetooth pairing...");
            updateClaimInfo();
        }
    }

    if (!wifiNow && wifiWasConnected) {
        // WiFi just disconnected -> start BLE
        startBLE();
    }

    // If cloud device just got claimed while BLE is active, stop BLE
    if (wifiNow && bleActive && !needsBlePairing()) {
        // Was kept alive for pairing, now claimed — shut down BLE
        setStatus("Claimed! Stopping BLE...");
        delay(500);
        stopBLE();
    }

    wifiWasConnected = wifiNow;

    // Handle pending credentials: reboot to apply
    if (hasPendingCreds) {
        hasPendingCreds = false;
        delay(1000);  // Let BLE notification send
        ESP.restart();
    }
}

bool isAdvertising() {
    return bleActive;
}

void refreshClaimInfo() {
    updateClaimInfo();
}

} // namespace BleProv

// C-linkage wrapper for avatar.cpp to call without including BLE headers
extern "C" bool _bleProv_isAdvertising() {
    return BleProv::isAdvertising();
}
