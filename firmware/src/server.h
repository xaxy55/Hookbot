#pragma once

#include "avatar.h"
#include <functional>

// Tool info received from Claude Code hooks
struct ToolInfo {
    char name[24];    // e.g. "Read", "Bash", "Edit"
    char detail[24];  // e.g. filename or pattern
};

// Task/checklist item for OLED display
#define MAX_TASKS 8
#define MAX_TASK_LEN 16

struct TaskItem {
    char label[MAX_TASK_LEN];
    uint8_t status; // 0=pending, 1=active, 2=done, 3=failed
};

struct TaskList {
    TaskItem items[MAX_TASKS];
    uint8_t count;
    uint8_t activeIndex;
};

// Notification data (e.g. Teams unread messages)
#define MAX_NOTIF_SOURCE 16
struct NotificationData {
    char source[MAX_NOTIF_SOURCE];  // e.g. "teams", "slack", "email"
    int unread;
    bool active;  // whether to show on display
};

#define MAX_NOTIFICATIONS 4

// Active project info
#define MAX_PROJECT_LEN 24
struct ProjectInfo {
    char name[MAX_PROJECT_LEN];
    uint32_t lastUpdatedAt;  // millis() when last set
};

// XP / Level data from management server
struct XpData {
    int level;
    int xp;
    int progress;  // 0-100 percent toward next level
    char title[24];
};

// LED color override per avatar state
struct LedColorRGB { uint8_t r, g, b; };

// Runtime configuration stored in NVS
struct RuntimeConfig {
    int ledBrightness;
    bool soundEnabled;
    int soundVolume;
    char hostname[32];
    char mgmtServer[128];
    char apiKey[64];
    // Accessories
    bool topHat;
    bool cigar;
    bool glasses;
    bool monocle;
    bool bowtie;
    bool crown;
    bool horns;
    bool halo;
    // Custom LED colors (one per AvatarState: IDLE=0 through ERROR=5)
    LedColorRGB ledColors[6];
    bool ledColorsCustom;
    // Auto-brightness from ambient light sensor
    bool autoBrightness;
    // Screensaver timeout in minutes (0 = disabled)
    int screensaverMins;
};

// WiFi + HTTP API + mDNS subsystem
namespace HookbotServer {
    void init(std::function<void(AvatarState)> onStateChange);
    void update();
    bool isConnected();
    const ToolInfo& getCurrentTool();
    RuntimeConfig& getConfig();
    TaskList& getTasks();
    void loadConfigFromNVS();
    void saveConfigToNVS();
    NotificationData* getNotifications();
    int getNotificationCount();
    XpData& getXpData();
    ProjectInfo& getProject();
    void sendVoiceToServer(const uint8_t* data, size_t size);
}
