#include "ble_prov.h"
#include "server.h"
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

namespace BleProv {

static BLEServer* pServer = nullptr;
static BLECharacteristic* pStatusChar = nullptr;
static bool bleActive = false;
static bool deviceConnected = false;
static bool wifiWasConnected = false;
static String pendingSsid;
static String pendingPass;
static bool hasPendingCreds = false;

static void setStatus(const char* msg) {
    if (pStatusChar) {
        pStatusChar->setValue(msg);
        if (deviceConnected) {
            pStatusChar->notify();
        }
    }
    Serial.printf("[BLE] Status: %s\n", msg);
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
        setStatus("Connected. Send: SSID\\nPASSWORD");
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

    BLEService* pService = pServer->createService(SERVICE_UUID);

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
    Serial.println("[BLE] Stopped");
}

void init() {
    // Start BLE immediately if WiFi isn't connected
    if (!HookbotServer::isConnected()) {
        startBLE();
    }
}

void update() {
    bool wifiNow = HookbotServer::isConnected();

    // WiFi just connected -> stop BLE to save resources
    if (wifiNow && !wifiWasConnected) {
        stopBLE();
    }

    // WiFi just disconnected -> start BLE
    if (!wifiNow && wifiWasConnected) {
        startBLE();
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

} // namespace BleProv

// C-linkage wrapper for avatar.cpp to call without including BLE headers
extern "C" bool _bleProv_isAdvertising() {
    return BleProv::isAdvertising();
}
