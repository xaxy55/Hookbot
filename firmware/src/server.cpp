#include "server.h"
#include "config.h"
#include "servo.h"
#include "sensors.h"
#include "sound.h"
#ifndef NO_LED
#include "led.h"
#endif
#ifndef NO_AUDIO
#include "audio.h"
#endif
#include "animation_player.h"
#include <WiFi.h>
#include <WiFiMulti.h>
#include <ESPmDNS.h>
#include <ESPAsyncWebServer.h>
#include <AsyncJson.h>
#include <ArduinoJson.h>
#include <Preferences.h>
#include <HTTPClient.h>
#include <HTTPUpdate.h>

namespace HookbotServer {

static AsyncWebServer server(80);
static WiFiMulti wifiMulti;
static std::function<void(AvatarState)> stateCallback;
static uint32_t connectStart = 0;
static ToolInfo currentTool = {"", ""};
static RuntimeConfig runtimeConfig;
static TaskList taskList = {};
static Preferences prefs;
static String pendingOtaUrl;
static NotificationData notifications[MAX_NOTIFICATIONS] = {};
static int notificationCount = 0;
static XpData xpData = {0, 0, 0, "Newbie"};
static ProjectInfo projectInfo = {"", 0};

static const char* stateToString(AvatarState s) {
    switch (s) {
        case AvatarState::IDLE:      return "idle";
        case AvatarState::THINKING:  return "thinking";
        case AvatarState::WAITING:   return "waiting";
        case AvatarState::SUCCESS:   return "success";
        case AvatarState::TASKCHECK: return "taskcheck";
        case AvatarState::ERROR:     return "error";
    }
    return "unknown";
}

static AvatarState stringToState(const String& s) {
    if (s == "idle")      return AvatarState::IDLE;
    if (s == "thinking")  return AvatarState::THINKING;
    if (s == "waiting")   return AvatarState::WAITING;
    if (s == "success")   return AvatarState::SUCCESS;
    if (s == "taskcheck") return AvatarState::TASKCHECK;
    if (s == "error")     return AvatarState::ERROR;
    return AvatarState::IDLE;
}

void loadConfigFromNVS() {
    prefs.begin("hookbot", true); // read-only
    runtimeConfig.ledBrightness = prefs.getInt("ledBright", 60);
    runtimeConfig.soundEnabled = prefs.getBool("soundOn", true);
    runtimeConfig.soundVolume = prefs.getInt("soundVol", 50);
    // Default hostname: hookbot-XXYY from last 2 bytes of MAC
    String defaultHostname = MDNS_HOSTNAME;
    {
        String mac = WiFi.macAddress(); // "AA:BB:CC:DD:EE:FF"
        String suffix = mac.substring(mac.length() - 5);
        suffix.replace(":", "");
        suffix.toLowerCase();
        defaultHostname += "-" + suffix;
    }
    String hostname = prefs.getString("hostname", "");
    if (hostname.length() == 0) hostname = defaultHostname;
    strncpy(runtimeConfig.hostname, hostname.c_str(), sizeof(runtimeConfig.hostname) - 1);
    String mgmt = prefs.getString("mgmtServer", DEFAULT_MGMT_SERVER);
    strncpy(runtimeConfig.mgmtServer, mgmt.c_str(), sizeof(runtimeConfig.mgmtServer) - 1);
    String apiKey = prefs.getString("apiKey", "");
    strncpy(runtimeConfig.apiKey, apiKey.c_str(), sizeof(runtimeConfig.apiKey) - 1);
    // Accessories (default: hat + cigar for the CEO)
    runtimeConfig.topHat = prefs.getBool("accHat", true);
    runtimeConfig.cigar = prefs.getBool("accCigar", true);
    runtimeConfig.glasses = prefs.getBool("accGlasses", false);
    runtimeConfig.monocle = prefs.getBool("accMonocle", false);
    runtimeConfig.bowtie = prefs.getBool("accBowtie", false);
    runtimeConfig.crown = prefs.getBool("accCrown", false);
    runtimeConfig.horns = prefs.getBool("accHorns", false);
    runtimeConfig.halo = prefs.getBool("accHalo", false);
    // Custom LED colors
    runtimeConfig.ledColorsCustom = prefs.getBool("ledCustom", false);
    if (runtimeConfig.ledColorsCustom) {
        prefs.getBytes("ledClrs", runtimeConfig.ledColors, sizeof(runtimeConfig.ledColors));
    }
    // Auto-brightness
    runtimeConfig.autoBrightness = prefs.getBool("autoBright", false);
    // Screensaver timeout (default 15 minutes, 0 = disabled)
    runtimeConfig.screensaverMins = prefs.getInt("ssTimeout", 15);
    prefs.end();
    Serial.printf("[Server] Config loaded: brightness=%d, sound=%s, vol=%d, host=%s\n",
        runtimeConfig.ledBrightness,
        runtimeConfig.soundEnabled ? "on" : "off",
        runtimeConfig.soundVolume,
        runtimeConfig.hostname);
}

void saveConfigToNVS() {
    prefs.begin("hookbot", false); // read-write
    prefs.putInt("ledBright", runtimeConfig.ledBrightness);
    prefs.putBool("soundOn", runtimeConfig.soundEnabled);
    prefs.putInt("soundVol", runtimeConfig.soundVolume);
    prefs.putString("hostname", runtimeConfig.hostname);
    prefs.putString("mgmtServer", runtimeConfig.mgmtServer);
    prefs.putString("apiKey", runtimeConfig.apiKey);
    prefs.putBool("accHat", runtimeConfig.topHat);
    prefs.putBool("accCigar", runtimeConfig.cigar);
    prefs.putBool("accGlasses", runtimeConfig.glasses);
    prefs.putBool("accMonocle", runtimeConfig.monocle);
    prefs.putBool("accBowtie", runtimeConfig.bowtie);
    prefs.putBool("accCrown", runtimeConfig.crown);
    prefs.putBool("accHorns", runtimeConfig.horns);
    prefs.putBool("accHalo", runtimeConfig.halo);
    // Custom LED colors
    prefs.putBool("ledCustom", runtimeConfig.ledColorsCustom);
    prefs.putBytes("ledClrs", runtimeConfig.ledColors, sizeof(runtimeConfig.ledColors));
    // Auto-brightness
    prefs.putBool("autoBright", runtimeConfig.autoBrightness);
    // Screensaver timeout
    prefs.putInt("ssTimeout", runtimeConfig.screensaverMins);
    prefs.end();
    Serial.println("[Server] Config saved to NVS");
}

static void registerWithServer() {
    if (strlen(runtimeConfig.mgmtServer) == 0) return;

    HTTPClient http;
    String url = String(runtimeConfig.mgmtServer) + "/api/devices";
    http.begin(url);
    http.addHeader("Content-Type", "application/json");
    if (strlen(runtimeConfig.apiKey) > 0) {
        http.addHeader("X-API-Key", runtimeConfig.apiKey);
    }

    JsonDocument doc;
    doc["name"] = runtimeConfig.hostname;
    doc["hostname"] = runtimeConfig.hostname;
    doc["ip_address"] = WiFi.localIP().toString();
    doc["purpose"] = "hookbot";

    String body;
    serializeJson(doc, body);

    int code = http.POST(body);
    if (code > 0) {
        Serial.printf("[Server] Registration response: %d\n", code);
    } else {
        Serial.printf("[Server] Registration failed: %s\n", http.errorToString(code).c_str());
    }
    http.end();
}

static const char CONTROL_PAGE[] PROGMEM = R"rawliteral(
<!DOCTYPE html>
<html>
<head>
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>CEO - Destroyer of Worlds</title>
<style>
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:system-ui,sans-serif;background:#0a0a0a;color:#eee;
     display:flex;flex-direction:column;align-items:center;padding:20px;min-height:100vh}
h1{font-size:1.5rem;margin-bottom:4px;color:#ff4444;text-transform:uppercase;letter-spacing:2px}
h2{font-size:0.85rem;margin-bottom:20px;color:#666;font-weight:400;letter-spacing:4px}
.status{background:#1a0a0a;padding:12px 24px;border-radius:12px;margin-bottom:24px;
        font-size:0.9rem;color:#94a3b8;border:1px solid #331111}
.grid{display:grid;grid-template-columns:1fr 1fr;gap:12px;max-width:340px;width:100%}
button{padding:16px;border:none;border-radius:12px;font-size:1rem;font-weight:600;
       cursor:pointer;transition:transform 0.1s,opacity 0.1s;color:#fff}
button:active{transform:scale(0.95);opacity:0.8}
.idle{background:#8b0000}.thinking{background:#4a0080}
.waiting{background:#b8860b}.success{background:#006400}
.taskcheck{background:#005555}.error{background:#ff0000}
#resp{margin-top:16px;font-size:0.8rem;color:#64748b;min-height:1.5em}
</style>
</head>
<body>
<h1>Destroyer of Worlds</h1>
<h2>CEO COMMAND CENTER</h2>
<div class="status" id="status">Loading...</div>
<div class="grid">
<button class="idle" onclick="send('idle')">Scheming</button>
<button class="thinking" onclick="send('thinking')">Plotting</button>
<button class="waiting" onclick="send('waiting')">Displeased</button>
<button class="success" onclick="send('success')">Conquered</button>
<button class="taskcheck" onclick="send('taskcheck')">Approved</button>
<button class="error" onclick="send('error')">DESTROY</button>
</div>
<div id="resp"></div>
<script>
async function send(state){
  document.getElementById('resp').textContent='Sending...';
  try{
    const r=await fetch('/state',{method:'POST',
      headers:{'Content-Type':'application/json'},
      body:JSON.stringify({state})});
    const j=await r.json();
    document.getElementById('resp').textContent=JSON.stringify(j);
    refresh();
  }catch(e){document.getElementById('resp').textContent='Error: '+e.message}
}
async function refresh(){
  try{
    const r=await fetch('/status');
    const j=await r.json();
    const up=Math.floor(j.uptime/1000);
    document.getElementById('status').textContent=
      'Mood: '+j.state.toUpperCase()+' | Reign: '+up+'s | Power: '+j.freeHeap+'B | FW: '+j.firmware_version;
  }catch(e){}
}
refresh();setInterval(refresh,3000);
</script>
</body>
</html>
)rawliteral";

void init(std::function<void(AvatarState)> onStateChange) {
    stateCallback = onStateChange;

    // Load config from NVS
    loadConfigFromNVS();

    // Connect WiFi (multi-network support)
    WiFi.mode(WIFI_STA);

    // Add compile-time networks (if secrets.h was provided)
#ifdef WIFI_SSID
    wifiMulti.addAP(WIFI_SSID, WIFI_PASS);
    Serial.printf("[Server] WiFi network added: %s\n", WIFI_SSID);
#ifdef WIFI_SSID2
    wifiMulti.addAP(WIFI_SSID2, WIFI_PASS2);
    Serial.printf("[Server] WiFi network added: %s\n", WIFI_SSID2);
#endif
#ifdef WIFI_SSID3
    wifiMulti.addAP(WIFI_SSID3, WIFI_PASS3);
    Serial.printf("[Server] WiFi network added: %s\n", WIFI_SSID3);
#endif
#endif

    // Add NVS-stored networks
    {
        Preferences wifiPrefs;
        wifiPrefs.begin("wifi", true);
        for (int i = 0; i < MAX_WIFI_NETWORKS; i++) {
            char keyS[8], keyP[8];
            snprintf(keyS, sizeof(keyS), "ssid%d", i);
            snprintf(keyP, sizeof(keyP), "pass%d", i);
            String ssid = wifiPrefs.getString(keyS, "");
            String pass = wifiPrefs.getString(keyP, "");
            if (ssid.length() > 0) {
                wifiMulti.addAP(ssid.c_str(), pass.c_str());
                Serial.printf("[Server] WiFi network added from NVS: %s\n", ssid.c_str());
            }
        }
        wifiPrefs.end();
    }

    connectStart = millis();
    Serial.print("[Server] Connecting to WiFi");

    while (wifiMulti.run() != WL_CONNECTED && millis() - connectStart < 15000) {
        delay(500);
        Serial.print(".");
    }
    Serial.println();

    if (WiFi.status() == WL_CONNECTED) {
        Serial.printf("[Server] Connected to %s | IP: %s\n", WiFi.SSID().c_str(), WiFi.localIP().toString().c_str());
    } else {
        Serial.println("[Server] WiFi connection failed - continuing offline");
    }

    // mDNS - use configurable hostname
    if (MDNS.begin(runtimeConfig.hostname)) {
        MDNS.addService("http", "tcp", 80);
        Serial.printf("[Server] mDNS: %s.local\n", runtimeConfig.hostname);
    }

    // Routes
    server.on("/", HTTP_GET, [](AsyncWebServerRequest* req) {
        req->send_P(200, "text/html", CONTROL_PAGE);
    });

    server.on("/status", HTTP_GET, [](AsyncWebServerRequest* req) {
        JsonDocument doc;
        doc["state"] = stateToString(Avatar::getState());
        doc["uptime"] = millis();
        doc["freeHeap"] = ESP.getFreeHeap();
        doc["ip"] = WiFi.localIP().toString();
        doc["hostname"] = runtimeConfig.hostname;
        doc["firmware_version"] = FIRMWARE_VERSION;
#ifdef BOARD_ESP32_4848S040C
        doc["device_type"] = "esp32_4848s040c_lcd";
#else
        doc["device_type"] = "esp32_oled";
#endif

        // Sensor readings
        SensorChannel* sCh = Sensors::getChannels();
        JsonArray sensors = doc["sensors"].to<JsonArray>();
        for (int i = 0; i < Sensors::getChannelCount(); i++) {
            if (sCh[i].type != SensorType::None) {
                JsonObject s = sensors.add<JsonObject>();
                s["ch"] = i;
                s["label"] = sCh[i].label;
                s["value"] = sCh[i].lastValue;
                s["triggered"] = sCh[i].triggered;
            }
        }
        doc["presence_away"] = Sensors::isPresenceAway();

        // Active project
        if (strlen(projectInfo.name) > 0) {
            doc["project"] = projectInfo.name;
        }

        String json;
        serializeJson(doc, json);
        req->send(200, "application/json", json);
    });

    // GET /info - device info for management server
    server.on("/info", HTTP_GET, [](AsyncWebServerRequest* req) {
        JsonDocument doc;
        doc["hostname"] = runtimeConfig.hostname;
        doc["mac"] = WiFi.macAddress();
        doc["firmware_version"] = FIRMWARE_VERSION;
        doc["ip"] = WiFi.localIP().toString();
        doc["freeHeap"] = ESP.getFreeHeap();
        doc["uptime"] = millis();
        doc["chip_model"] = ESP.getChipModel();
#ifdef BOARD_ESP32_4848S040C
        doc["device_type"] = "esp32_4848s040c_lcd";
#else
        doc["device_type"] = "esp32_oled";
#endif

        JsonArray caps = doc["capabilities"].to<JsonArray>();
        caps.add("display");
#ifndef NO_LED
        caps.add("led");
#endif
#ifndef NO_SOUND
        caps.add("buzzer");
#endif
        caps.add("ota");
#ifndef NO_AUDIO
        caps.add("microphone");
        caps.add("speaker");
#endif
#ifdef BOARD_ESP32_4848S040C
        caps.add("touch");
#endif

        String json;
        serializeJson(doc, json);
        req->send(200, "application/json", json);
    });

    // POST /state with body handler
    AsyncCallbackJsonWebHandler* stateHandler = new AsyncCallbackJsonWebHandler(
        "/state",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();
            String stateStr = body["state"] | "idle";
            AvatarState newState = stringToState(stateStr);

            // Parse tool info if present
            const char* toolName = body["tool"] | "";
            const char* toolDetail = body["detail"] | "";
            strncpy(currentTool.name, toolName, sizeof(currentTool.name) - 1);
            currentTool.name[sizeof(currentTool.name) - 1] = '\0';
            strncpy(currentTool.detail, toolDetail, sizeof(currentTool.detail) - 1);
            currentTool.detail[sizeof(currentTool.detail) - 1] = '\0';

            if (strlen(currentTool.name) > 0) {
                Serial.printf("[Server] Tool: %s (%s)\n", currentTool.name, currentTool.detail);
                Servos::onToolChange(currentTool.name);
            }

            if (stateCallback) {
                stateCallback(newState);
            }

            JsonDocument resp;
            resp["ok"] = true;
            resp["state"] = stateToString(newState);
            resp["tool"] = currentTool.name;

            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(stateHandler);

    // POST /ota - receive OTA URL, defer download to loop()
    AsyncCallbackJsonWebHandler* otaHandler = new AsyncCallbackJsonWebHandler(
        "/ota",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();
            const char* url = body["url"] | "";
            if (strlen(url) == 0) {
                req->send(400, "application/json", "{\"error\":\"url required\"}");
                return;
            }
            pendingOtaUrl = String(url);
            Serial.printf("[Server] OTA queued from: %s\n", url);
            req->send(200, "application/json", "{\"ok\":true,\"msg\":\"OTA queued\"}");
        }
    );
    server.addHandler(otaHandler);

    // POST /config - update runtime config from management server
    AsyncCallbackJsonWebHandler* configHandler = new AsyncCallbackJsonWebHandler(
        "/config",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            if (!body["led_brightness"].isNull()) {
                runtimeConfig.ledBrightness = body["led_brightness"];
            }
            if (!body["sound_enabled"].isNull()) {
                runtimeConfig.soundEnabled = body["sound_enabled"];
            }
            if (!body["sound_volume"].isNull()) {
                runtimeConfig.soundVolume = body["sound_volume"];
            }
            if (!body["hostname"].isNull()) {
                const char* h = body["hostname"];
                strncpy(runtimeConfig.hostname, h, sizeof(runtimeConfig.hostname) - 1);
            }
            if (!body["mgmt_server"].isNull()) {
                const char* m = body["mgmt_server"];
                strncpy(runtimeConfig.mgmtServer, m, sizeof(runtimeConfig.mgmtServer) - 1);
            }
            // Accessories from avatar_preset
            if (!body["avatar_preset"].isNull()) {
                JsonObject preset = body["avatar_preset"];
                if (!preset["accessories"].isNull()) {
                    JsonObject acc = preset["accessories"];
                    if (!acc["topHat"].isNull())  runtimeConfig.topHat  = acc["topHat"];
                    if (!acc["cigar"].isNull())   runtimeConfig.cigar   = acc["cigar"];
                    if (!acc["glasses"].isNull()) runtimeConfig.glasses = acc["glasses"];
                    if (!acc["monocle"].isNull()) runtimeConfig.monocle = acc["monocle"];
                    if (!acc["bowtie"].isNull())  runtimeConfig.bowtie  = acc["bowtie"];
                    if (!acc["crown"].isNull())   runtimeConfig.crown   = acc["crown"];
                    if (!acc["horns"].isNull())   runtimeConfig.horns   = acc["horns"];
                    if (!acc["halo"].isNull())    runtimeConfig.halo    = acc["halo"];
                    Serial.printf("[Server] Accessories updated: hat=%d cigar=%d glasses=%d\n",
                        runtimeConfig.topHat, runtimeConfig.cigar, runtimeConfig.glasses);
                }
            }

            // Custom LED colors per state
            if (!body["led_colors"].isNull() && body["led_colors"].is<JsonObject>()) {
                JsonObject colors = body["led_colors"];
                const char* stateNames[] = {"idle", "thinking", "waiting", "success", "taskcheck", "error"};
                for (int i = 0; i < 6; i++) {
                    if (!colors[stateNames[i]].isNull()) {
                        const char* hex = colors[stateNames[i]];
                        if (hex && hex[0] == '#' && strlen(hex) == 7) {
                            unsigned long val = strtoul(hex + 1, NULL, 16);
                            runtimeConfig.ledColors[i].r = (val >> 16) & 0xFF;
                            runtimeConfig.ledColors[i].g = (val >> 8) & 0xFF;
                            runtimeConfig.ledColors[i].b = val & 0xFF;
                        }
                    }
                }
                runtimeConfig.ledColorsCustom = true;
                Serial.println("[Server] Custom LED colors updated");
            }

            // Screensaver timeout
            if (!body["screensaver_mins"].isNull()) {
                runtimeConfig.screensaverMins = body["screensaver_mins"];
                Serial.printf("[Server] Screensaver timeout: %d min\n", runtimeConfig.screensaverMins);
            }

            // Auto-brightness from ambient light sensor
            if (!body["auto_brightness"].isNull()) {
                runtimeConfig.autoBrightness = body["auto_brightness"];
#ifndef NO_LED
                Led::setAutoBrightness(runtimeConfig.autoBrightness);
#endif
                Serial.printf("[Server] Auto-brightness: %s\n",
                    runtimeConfig.autoBrightness ? "on" : "off");
            }

            saveConfigToNVS();

            JsonDocument resp;
            resp["ok"] = true;
            resp["led_brightness"] = runtimeConfig.ledBrightness;
            resp["sound_enabled"] = runtimeConfig.soundEnabled;
            resp["sound_volume"] = runtimeConfig.soundVolume;
            resp["hostname"] = runtimeConfig.hostname;
            resp["auto_brightness"] = runtimeConfig.autoBrightness;
            resp["screensaver_mins"] = runtimeConfig.screensaverMins;

            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(configHandler);

    // POST /tasks - receive checklist from management server
    AsyncCallbackJsonWebHandler* tasksHandler = new AsyncCallbackJsonWebHandler(
        "/tasks",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            // Clear existing
            taskList.count = 0;
            taskList.activeIndex = 0;

            if (!body["items"].isNull()) {
                JsonArray items = body["items"];
                for (size_t i = 0; i < items.size() && i < MAX_TASKS; i++) {
                    JsonObject item = items[i];
                    const char* label = item["label"] | "";
                    strncpy(taskList.items[i].label, label, MAX_TASK_LEN - 1);
                    taskList.items[i].label[MAX_TASK_LEN - 1] = '\0';
                    taskList.items[i].status = item["status"] | 0;
                    taskList.count++;
                }
            }
            if (!body["active"].isNull()) {
                taskList.activeIndex = body["active"] | 0;
            }

            Serial.printf("[Server] Tasks updated: %d items, active=%d\n", taskList.count, taskList.activeIndex);

            JsonDocument resp;
            resp["ok"] = true;
            resp["count"] = taskList.count;

            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(tasksHandler);

    // POST /notifications - receive notification data (Teams unread, etc.)
    AsyncCallbackJsonWebHandler* notifHandler = new AsyncCallbackJsonWebHandler(
        "/notifications",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            const char* source = body["source"] | "teams";
            int unread = body["unread"] | 0;
            bool active = body["active"].isNull() ? (unread > 0) : (bool)body["active"];

            // Find or create slot for this source
            int slot = -1;
            for (int i = 0; i < notificationCount; i++) {
                if (strcmp(notifications[i].source, source) == 0) {
                    slot = i;
                    break;
                }
            }
            if (slot < 0 && notificationCount < MAX_NOTIFICATIONS) {
                slot = notificationCount++;
            }
            if (slot >= 0) {
                strncpy(notifications[slot].source, source, MAX_NOTIF_SOURCE - 1);
                notifications[slot].source[MAX_NOTIF_SOURCE - 1] = '\0';
                notifications[slot].unread = unread;
                notifications[slot].active = active;
            }

            Serial.printf("[Server] Notification: %s unread=%d active=%d\n", source, unread, active);

            JsonDocument resp;
            resp["ok"] = true;
            resp["source"] = source;
            resp["unread"] = unread;

            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(notifHandler);

    // POST /xp - receive XP/level data from management server
    AsyncCallbackJsonWebHandler* xpHandler = new AsyncCallbackJsonWebHandler(
        "/xp",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            xpData.level = body["level"] | 0;
            xpData.xp = body["xp"] | 0;
            xpData.progress = body["progress"] | 0;
            const char* title = body["title"] | "Newbie";
            strncpy(xpData.title, title, sizeof(xpData.title) - 1);
            xpData.title[sizeof(xpData.title) - 1] = '\0';

            Serial.printf("[Server] XP update: level=%d xp=%d progress=%d%% title=%s\n",
                xpData.level, xpData.xp, xpData.progress, xpData.title);

            JsonDocument resp;
            resp["ok"] = true;
            resp["level"] = xpData.level;

            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(xpHandler);

    // POST /project - receive active project name from management server
    AsyncCallbackJsonWebHandler* projectHandler = new AsyncCallbackJsonWebHandler(
        "/project",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            const char* name = body["name"] | "";
            strncpy(projectInfo.name, name, MAX_PROJECT_LEN - 1);
            projectInfo.name[MAX_PROJECT_LEN - 1] = '\0';
            projectInfo.lastUpdatedAt = millis();

            Serial.printf("[Server] Project: %s\n", projectInfo.name);

            JsonDocument resp;
            resp["ok"] = true;
            resp["project"] = projectInfo.name;

            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(projectHandler);

#ifndef NO_SOUND
    // POST /sounds - set custom sound pack melodies
    AsyncCallbackJsonWebHandler* soundsHandler = new AsyncCallbackJsonWebHandler(
        "/sounds",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            const char* pack = body["pack"] | "default";
            Serial.printf("[Server] Sound pack: %s\n", pack);

            if (String(pack) == "default") {
                // Disable custom melodies, revert to hardcoded
                Sound::setCustomMelodies(false);
            } else {
                // Enable custom melodies and parse them
                Sound::setCustomMelodies(true);

                if (!body["melodies"].isNull() && body["melodies"].is<JsonObject>()) {
                    JsonObject melodies = body["melodies"];
                    const char* stateNames[] = {"0", "1", "2", "3", "4", "5"};
                    for (int i = 0; i < 6; i++) {
                        if (!melodies[stateNames[i]].isNull()) {
                            JsonArray notes = melodies[stateNames[i]];
                            Melody m = {};
                            for (size_t n = 0; n < notes.size() && n < MAX_MELODY_NOTES; n++) {
                                JsonObject note = notes[n];
                                m.freqs[n] = note["freq"] | 0;
                                m.durations[n] = note["dur"] | 0;
                                m.count++;
                            }
                            Sound::setMelody(i, m);
                        }
                    }
                }
            }

            // Save pack name to NVS
            {
                Preferences packPrefs;
                packPrefs.begin("sndpack", false);
                packPrefs.putString("name", pack);
                packPrefs.end();
            }
            Sound::saveMelodiesToNVS();

            JsonDocument resp;
            resp["ok"] = true;
            resp["pack"] = pack;

            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(soundsHandler);
#endif // NO_SOUND

    // GET /notifications - read current notification state
    server.on("/notifications", HTTP_GET, [](AsyncWebServerRequest* req) {
        JsonDocument doc;
        JsonArray arr = doc["notifications"].to<JsonArray>();
        for (int i = 0; i < notificationCount; i++) {
            JsonObject n = arr.add<JsonObject>();
            n["source"] = notifications[i].source;
            n["unread"] = notifications[i].unread;
            n["active"] = notifications[i].active;
        }

        String json;
        serializeJson(doc, json);
        req->send(200, "application/json", json);
    });

    // GET /wifi - list configured networks + current connection
    server.on("/wifi", HTTP_GET, [](AsyncWebServerRequest* req) {
        JsonDocument doc;
        doc["connected"] = WiFi.status() == WL_CONNECTED;
        doc["ssid"] = WiFi.SSID();
        doc["rssi"] = WiFi.RSSI();
        doc["ip"] = WiFi.localIP().toString();

        JsonArray nets = doc["networks"].to<JsonArray>();
        // Compile-time networks (names only, not passwords)
#ifdef WIFI_SSID
        JsonObject n0 = nets.add<JsonObject>();
        n0["ssid"] = WIFI_SSID;
        n0["source"] = "compile";
#ifdef WIFI_SSID2
        JsonObject n1 = nets.add<JsonObject>();
        n1["ssid"] = WIFI_SSID2;
        n1["source"] = "compile";
#endif
#ifdef WIFI_SSID3
        JsonObject n2 = nets.add<JsonObject>();
        n2["ssid"] = WIFI_SSID3;
        n2["source"] = "compile";
#endif
#endif
        // NVS networks
        Preferences wifiPrefs;
        wifiPrefs.begin("wifi", true);
        for (int i = 0; i < MAX_WIFI_NETWORKS; i++) {
            char keyS[8];
            snprintf(keyS, sizeof(keyS), "ssid%d", i);
            String ssid = wifiPrefs.getString(keyS, "");
            if (ssid.length() > 0) {
                JsonObject n = nets.add<JsonObject>();
                n["ssid"] = ssid;
                n["source"] = "nvs";
                n["index"] = i;
            }
        }
        wifiPrefs.end();

        String json;
        serializeJson(doc, json);
        req->send(200, "application/json", json);
    });

    // POST /wifi - add or remove WiFi networks in NVS
    AsyncCallbackJsonWebHandler* wifiHandler = new AsyncCallbackJsonWebHandler(
        "/wifi",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            Preferences wifiPrefs;
            wifiPrefs.begin("wifi", false);

            // Add a network: {"action": "add", "ssid": "...", "password": "..."}
            if (String((const char*)(body["action"] | "")) == "add") {
                const char* ssid = body["ssid"] | "";
                const char* pass = body["password"] | "";
                if (strlen(ssid) == 0) {
                    req->send(400, "application/json", "{\"error\":\"ssid required\"}");
                    wifiPrefs.end();
                    return;
                }
                // Find empty slot
                int slot = -1;
                for (int i = 0; i < MAX_WIFI_NETWORKS; i++) {
                    char keyS[8];
                    snprintf(keyS, sizeof(keyS), "ssid%d", i);
                    String existing = wifiPrefs.getString(keyS, "");
                    if (existing.length() == 0) { slot = i; break; }
                    if (existing == ssid) { slot = i; break; } // overwrite same SSID
                }
                if (slot < 0) {
                    req->send(400, "application/json", "{\"error\":\"max networks reached\"}");
                    wifiPrefs.end();
                    return;
                }
                char keyS[8], keyP[8];
                snprintf(keyS, sizeof(keyS), "ssid%d", slot);
                snprintf(keyP, sizeof(keyP), "pass%d", slot);
                wifiPrefs.putString(keyS, ssid);
                wifiPrefs.putString(keyP, pass);
                wifiMulti.addAP(ssid, pass);
                Serial.printf("[Server] WiFi network added: %s (slot %d)\n", ssid, slot);

                JsonDocument resp;
                resp["ok"] = true;
                resp["slot"] = slot;
                resp["ssid"] = ssid;
                String json;
                serializeJson(resp, json);
                req->send(200, "application/json", json);
            }
            // Remove a network: {"action": "remove", "index": 0}
            else if (String((const char*)(body["action"] | "")) == "remove") {
                int idx = body["index"] | -1;
                if (idx < 0 || idx >= MAX_WIFI_NETWORKS) {
                    req->send(400, "application/json", "{\"error\":\"invalid index\"}");
                    wifiPrefs.end();
                    return;
                }
                char keyS[8], keyP[8];
                snprintf(keyS, sizeof(keyS), "ssid%d", idx);
                snprintf(keyP, sizeof(keyP), "pass%d", idx);
                wifiPrefs.remove(keyS);
                wifiPrefs.remove(keyP);
                Serial.printf("[Server] WiFi network removed: slot %d\n", idx);

                // Note: WiFiMulti doesn't support removing. Reboot needed for full effect.
                JsonDocument resp;
                resp["ok"] = true;
                resp["msg"] = "Removed. Reboot device to apply.";
                String json;
                serializeJson(resp, json);
                req->send(200, "application/json", json);
            }
            else {
                req->send(400, "application/json", "{\"error\":\"action must be add or remove\"}");
            }

            wifiPrefs.end();
        }
    );
    server.addHandler(wifiHandler);

    // GET /servos - read current servo state
    server.on("/servos", HTTP_GET, [](AsyncWebServerRequest* req) {
        ServoChannel* ch = Servos::getChannels();
        ServoStateMap* maps = Servos::getStateMaps();
        JsonDocument doc;
        JsonArray arr = doc["channels"].to<JsonArray>();
        for (int i = 0; i < MAX_SERVOS; i++) {
            JsonObject o = arr.add<JsonObject>();
            o["pin"] = ch[i].pin;
            o["min"] = ch[i].minAngle;
            o["max"] = ch[i].maxAngle;
            o["rest"] = ch[i].restAngle;
            o["current"] = ch[i].currentAngle;
            o["label"] = ch[i].label;
            o["enabled"] = ch[i].enabled;
        }
        // Include state maps
        JsonObject sm = doc["state_maps"].to<JsonObject>();
        const char* stateNames[] = {"idle","thinking","waiting","success","taskcheck","error"};
        for (int s = 0; s < 6; s++) {
            JsonArray a = sm[stateNames[s]].to<JsonArray>();
            for (int i = 0; i < MAX_SERVOS; i++) {
                a.add(maps[s].angles[i]);
            }
        }
        String json;
        serializeJson(doc, json);
        req->send(200, "application/json", json);
    });

    // POST /servos - set servo positions
    AsyncCallbackJsonWebHandler* servoHandler = new AsyncCallbackJsonWebHandler(
        "/servos",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            // Set individual angles: {"angles": [90, 90, 45, 135]}
            if (!body["angles"].isNull()) {
                JsonArray angles = body["angles"];
                for (size_t i = 0; i < angles.size() && i < MAX_SERVOS; i++) {
                    Servos::setAngle(i, angles[i]);
                }
            }

            // Set single channel: {"channel": 0, "angle": 45}
            if (!body["channel"].isNull()) {
                uint8_t ch = body["channel"];
                uint8_t angle = body["angle"] | 90;
                Servos::setAngle(ch, angle);
            }

            // Rest all: {"rest": true}
            if (body["rest"] == true) {
                Servos::setAllToRest();
            }

            JsonDocument resp;
            resp["ok"] = true;
            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(servoHandler);

    // POST /servos/config - configure servo channels
    AsyncCallbackJsonWebHandler* servoConfigHandler = new AsyncCallbackJsonWebHandler(
        "/servos/config",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            if (!body["channels"].isNull()) {
                JsonArray chArr = body["channels"];
                for (size_t i = 0; i < chArr.size() && i < MAX_SERVOS; i++) {
                    JsonObject ch = chArr[i];
                    int8_t pin = ch["pin"] | -1;
                    uint8_t minA = ch["min"] | 0;
                    uint8_t maxA = ch["max"] | 180;
                    uint8_t rest = ch["rest"] | 90;
                    const char* label = ch["label"] | "servo";
                    Servos::configureChannel(i, pin, minA, maxA, rest, label);
                }
            }

            // Update state maps if provided
            if (!body["state_maps"].isNull()) {
                JsonObject sm = body["state_maps"];
                ServoStateMap* maps = Servos::getStateMaps();
                const char* stateNames[] = {"idle","thinking","waiting","success","taskcheck","error"};
                for (int s = 0; s < 6; s++) {
                    if (!sm[stateNames[s]].isNull()) {
                        JsonArray a = sm[stateNames[s]];
                        for (size_t i = 0; i < a.size() && i < MAX_SERVOS; i++) {
                            maps[s].angles[i] = a[i];
                        }
                    }
                }
                Servos::saveToNVS();
            }

            JsonDocument resp;
            resp["ok"] = true;
            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(servoConfigHandler);

    // GET /sensors - read current sensor channels and values
    server.on("/sensors", HTTP_GET, [](AsyncWebServerRequest* req) {
        JsonDocument doc;
        SensorChannel* sCh = Sensors::getChannels();
        JsonArray arr = doc["channels"].to<JsonArray>();
        for (int i = 0; i < Sensors::getChannelCount(); i++) {
            JsonObject o = arr.add<JsonObject>();
            o["pin"] = sCh[i].pin;
            o["type"] = (int)sCh[i].type;
            o["label"] = sCh[i].label;
            o["pollIntervalMs"] = sCh[i].pollIntervalMs;
            o["threshold"] = sCh[i].threshold;
            o["lastValue"] = sCh[i].lastValue;
            o["triggered"] = sCh[i].triggered;
        }
        doc["presence_away"] = Sensors::isPresenceAway();
        doc["presence_timeout_ms"] = Sensors::getPresenceTimeoutMs();

        String json;
        serializeJson(doc, json);
        req->send(200, "application/json", json);
    });

    // POST /sensors/config - configure sensor channels
    AsyncCallbackJsonWebHandler* sensorConfigHandler = new AsyncCallbackJsonWebHandler(
        "/sensors/config",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();

            if (!body["channels"].isNull()) {
                JsonArray chArr = body["channels"];
                for (size_t i = 0; i < chArr.size() && i < MAX_SENSOR_CHANNELS; i++) {
                    JsonObject ch = chArr[i];
                    int8_t pin = ch["pin"] | (int8_t)-1;
                    SensorType type = (SensorType)(ch["type"] | 0);
                    const char* label = ch["label"] | "sensor";
                    uint16_t pollMs = ch["pollIntervalMs"] | 1000;
                    int16_t threshold = ch["threshold"] | 0;
                    Sensors::configureChannel(i, pin, type, label, pollMs, threshold);
                }
            }

            if (!body["presence_timeout_ms"].isNull()) {
                Sensors::setPresenceTimeoutMs(body["presence_timeout_ms"]);
            }

            Sensors::saveToNVS();

            JsonDocument resp;
            resp["ok"] = true;
            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(sensorConfigHandler);

    // POST /animation - receive animation JSON and play it
    AsyncCallbackJsonWebHandler* animHandler = new AsyncCallbackJsonWebHandler(
        "/animation",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            String body;
            serializeJson(jsonBody, body);

            if (!AnimPlayer::loadFromJson(body.c_str())) {
                req->send(400, "application/json", "{\"error\":\"invalid animation JSON\"}");
                return;
            }

            AnimPlayer::play();

            JsonDocument resp;
            resp["ok"] = true;
            resp["msg"] = "animation started";

            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(animHandler);

    // POST /animation/stop - stop current animation
    server.on("/animation/stop", HTTP_POST, [](AsyncWebServerRequest* req) {
        AnimPlayer::stop();

        JsonDocument resp;
        resp["ok"] = true;
        resp["msg"] = "animation stopped";

        String json;
        serializeJson(resp, json);
        req->send(200, "application/json", json);
    });

#ifndef NO_AUDIO
    // GET /audio/status - check audio subsystem state
    server.on("/audio/status", HTTP_GET, [](AsyncWebServerRequest* req) {
        JsonDocument doc;
        doc["recording"] = Audio::isRecording();
        doc["playing"] = Audio::isPlaying();
        doc["has_audio"] = Audio::hasRecordedAudio();
        doc["recorded_bytes"] = Audio::getRecordedSize();
        doc["volume"] = Audio::getVolume();
        doc["wake_enabled"] = true;  // always report capability

        String json;
        serializeJson(doc, json);
        req->send(200, "application/json", json);
    });

    // POST /audio/record - start recording from I2S mic
    server.on("/audio/record", HTTP_POST, [](AsyncWebServerRequest* req) {
        if (Audio::isPlaying()) {
            req->send(409, "application/json", "{\"error\":\"playback in progress\"}");
            return;
        }
        bool ok = Audio::startRecording();
        if (stateCallback) stateCallback(AvatarState::THINKING);
        JsonDocument resp;
        resp["ok"] = ok;
        resp["msg"] = ok ? "recording started" : "failed to start recording";
        String json;
        serializeJson(resp, json);
        req->send(ok ? 200 : 500, "application/json", json);
    });

    // POST /audio/stop - stop current recording
    server.on("/audio/stop", HTTP_POST, [](AsyncWebServerRequest* req) {
        Audio::stopRecording();
        JsonDocument resp;
        resp["ok"] = true;
        resp["has_audio"] = Audio::hasRecordedAudio();
        resp["recorded_bytes"] = Audio::getRecordedSize();
        String json;
        serializeJson(resp, json);
        req->send(200, "application/json", json);
    });

    // GET /audio/data - download recorded audio as raw PCM (16-bit, 16kHz, mono)
    server.on("/audio/data", HTTP_GET, [](AsyncWebServerRequest* req) {
        if (!Audio::hasRecordedAudio()) {
            req->send(404, "application/json", "{\"error\":\"no recorded audio\"}");
            return;
        }
        const uint8_t* data = Audio::getRecordedData();
        size_t size = Audio::getRecordedSize();
        AsyncWebServerResponse* response = req->beginResponse_P(
            200, "application/octet-stream", data, size);
        response->addHeader("X-Audio-Rate", String(AUDIO_SAMPLE_RATE));
        response->addHeader("X-Audio-Bits", String(AUDIO_BITS_PER_SAMPLE));
        response->addHeader("X-Audio-Channels", String(AUDIO_CHANNELS));
        req->send(response);
    });

    // POST /audio/play - receive raw PCM audio and play through I2S speaker
    // Body: raw PCM data (16-bit, 16kHz, mono)
    server.on("/audio/play", HTTP_POST,
        [](AsyncWebServerRequest* req) {
            // Response sent after body is received
        },
        NULL,
        [](AsyncWebServerRequest* req, uint8_t* data, size_t len, size_t index, size_t total) {
            // Accumulate body data into a static buffer
            static uint8_t* ttsBuffer = nullptr;
            static size_t ttsSize = 0;

            if (index == 0) {
                // First chunk: allocate buffer
#if BOARD_HAS_PSRAM
                ttsBuffer = (uint8_t*)ps_malloc(total);
#else
                ttsBuffer = (uint8_t*)malloc(total);
#endif
                ttsSize = 0;
                if (!ttsBuffer) {
                    req->send(500, "application/json", "{\"error\":\"out of memory\"}");
                    return;
                }
            }

            if (ttsBuffer && index + len <= total) {
                memcpy(ttsBuffer + index, data, len);
                ttsSize = index + len;
            }

            if (ttsSize == total && ttsBuffer) {
                // All data received, start playback
                bool ok = Audio::startPlayback(ttsBuffer, ttsSize);
                JsonDocument resp;
                resp["ok"] = ok;
                resp["bytes"] = ttsSize;
                String json;
                serializeJson(resp, json);
                req->send(ok ? 200 : 500, "application/json", json);
                // Note: buffer must persist until playback completes.
                // Audio module references it directly.
            }
        }
    );

    // POST /audio/volume - set speaker volume
    AsyncCallbackJsonWebHandler* volHandler = new AsyncCallbackJsonWebHandler(
        "/audio/volume",
        [](AsyncWebServerRequest* req, JsonVariant& jsonBody) {
            JsonObject body = jsonBody.as<JsonObject>();
            uint8_t vol = body["volume"] | 80;
            Audio::setVolume(vol);

            JsonDocument resp;
            resp["ok"] = true;
            resp["volume"] = Audio::getVolume();
            String json;
            serializeJson(resp, json);
            req->send(200, "application/json", json);
        }
    );
    server.addHandler(volHandler);
#endif // NO_AUDIO

    server.begin();
    Serial.println("[Server] HTTP server started on port 80");

    // Register with management server on boot
    if (WiFi.status() == WL_CONNECTED) {
        registerWithServer();
    }
}

void update() {
    // WiFi reconnection (tries all configured networks)
    static uint32_t lastReconnect = 0;
    if (WiFi.status() != WL_CONNECTED && millis() - lastReconnect > 10000) {
        lastReconnect = millis();
        wifiMulti.run();
    }

    // Handle pending OTA (must run from loop, not from async handler)
    if (pendingOtaUrl.length() > 0) {
        String url = pendingOtaUrl;
        pendingOtaUrl = "";
        Serial.printf("[OTA] Starting HTTP update from: %s\n", url.c_str());

        WiFiClient client;
        httpUpdate.rebootOnUpdate(true);
        t_httpUpdate_return ret = httpUpdate.update(client, url);

        switch (ret) {
            case HTTP_UPDATE_FAILED:
                Serial.printf("[OTA] Failed: %s\n", httpUpdate.getLastErrorString().c_str());
                break;
            case HTTP_UPDATE_NO_UPDATES:
                Serial.println("[OTA] No update available");
                break;
            case HTTP_UPDATE_OK:
                Serial.println("[OTA] Success - rebooting");
                break;
        }
    }
}

bool isConnected() {
    return WiFi.status() == WL_CONNECTED;
}

const ToolInfo& getCurrentTool() {
    return currentTool;
}

RuntimeConfig& getConfig() {
    return runtimeConfig;
}

TaskList& getTasks() {
    return taskList;
}

NotificationData* getNotifications() {
    return notifications;
}

int getNotificationCount() {
    return notificationCount;
}

XpData& getXpData() {
    return xpData;
}

ProjectInfo& getProject() {
    return projectInfo;
}

void sendVoiceToServer(const uint8_t* data, size_t size) {
#ifndef NO_AUDIO
    if (strlen(runtimeConfig.mgmtServer) == 0 || !data || size == 0) return;

    HTTPClient http;
    String url = String(runtimeConfig.mgmtServer) + "/api/voice/transcribe";
    http.begin(url);
    http.addHeader("Content-Type", "application/octet-stream");
    http.addHeader("X-Audio-Rate", String(AUDIO_SAMPLE_RATE));
    http.addHeader("X-Audio-Bits", String(AUDIO_BITS_PER_SAMPLE));
    http.addHeader("X-Device-Id", runtimeConfig.hostname);
    if (strlen(runtimeConfig.apiKey) > 0) {
        http.addHeader("X-API-Key", runtimeConfig.apiKey);
    }

    Serial.printf("[Server] Sending %d bytes of audio to %s\n", size, url.c_str());
    int code = http.POST((uint8_t*)data, size);

    if (code == 200) {
        String response = http.getString();
        Serial.printf("[Server] Voice response: %s\n", response.c_str());

        JsonDocument doc;
        if (deserializeJson(doc, response) == DeserializationError::Ok) {
            if (!doc["state"].isNull()) {
                const char* st = doc["state"];
                AvatarState newState = stringToState(String(st));
                if (stateCallback) stateCallback(newState);
            }
        }
    } else {
        Serial.printf("[Server] Voice upload failed: %d\n", code);
        if (stateCallback) stateCallback(AvatarState::ERROR);
    }
    http.end();
#else
    (void)data; (void)size;
#endif
}

} // namespace HookbotServer

// Global helper for avatar IP display
String _hookbot_get_ip() {
    return WiFi.localIP().toString();
}
