# API Reference

Base URL: `http://hookbot.local:3000` (or your server address)

## Hook Events

### POST /api/hook

Receives events from Claude Code via `hookbot-hook.js`.

```json
{
  "event": "PostToolUse",
  "tool_name": "Edit",
  "tool_output": "File updated successfully",
  "project": "/home/user/my-project",
  "device_id": "optional-device-uuid",
  "tasks": [
    { "label": "Fix login bug", "status": 2 },
    { "label": "Add tests", "status": 1 }
  ],
  "active_task": 1
}
```

**Events:** `PreToolUse`, `PostToolUse`, `UserPromptSubmit`, `TaskCompleted`, `Stop`

**Task status:** `0` pending, `1` active, `2` done, `3` failed

---

## Devices

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/devices` | List all devices |
| POST | `/api/devices` | Register a device |
| GET | `/api/devices/{id}` | Get device details |
| PUT | `/api/devices/{id}` | Update device |
| DELETE | `/api/devices/{id}` | Remove device |
| POST | `/api/devices/{id}/state` | Set avatar state |
| POST | `/api/devices/{id}/tasks` | Push task list |
| GET | `/api/devices/{id}/status` | Proxy to device status |
| GET | `/api/devices/{id}/history` | Status log (last 100) |
| GET | `/api/devices/{id}/config` | Get device config |
| PUT | `/api/devices/{id}/config` | Update config |
| POST | `/api/devices/{id}/config/push` | Push config to device |
| GET | `/api/devices/{id}/servos` | Get servo config |
| POST | `/api/devices/{id}/servos` | Control servos |
| POST | `/api/devices/{id}/servos/config` | Configure servo pins |

### Set State

```bash
curl -X POST http://hookbot.local:3000/api/devices/{id}/state \
  -H 'Content-Type: application/json' \
  -d '{"state": "thinking"}'
```

States: `idle`, `thinking`, `success`, `error`, `waiting`, `taskcheck`

---

## Gamification

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/gamification/stats` | XP, level, title, achievement count |
| GET | `/api/gamification/activity` | Recent tool usage feed |
| GET | `/api/gamification/analytics` | Charts data (tools/day, hourly, distribution) |
| GET | `/api/gamification/achievements` | All 17 badges with earned status |
| GET | `/api/gamification/leaderboard` | Device rankings by XP |
| GET | `/api/gamification/streaks` | Coding streak data |

### Stats Response

```json
{
  "device_id": "abc-123",
  "total_xp": 4250,
  "level": 8,
  "title": "Developer",
  "xp_to_next": 550,
  "xp_for_next_level": 4500,
  "achievements_earned": 7,
  "achievements_total": 17
}
```

---

## Store

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/store` | List items (27 total) with prices |
| POST | `/api/store/buy` | Purchase item with XP |
| GET | `/api/store/owned` | Owned items for device |

### Buy Item

```bash
curl -X POST http://hookbot.local:3000/api/store/buy \
  -H 'Content-Type: application/json' \
  -d '{"device_id": "abc-123", "item_id": "top_hat"}'
```

---

## Firmware & OTA

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/firmware` | Upload firmware binary |
| GET | `/api/firmware` | List uploaded firmwares |
| GET | `/api/firmware/{id}/binary` | Download firmware binary |
| POST | `/api/firmware/build` | Trigger PlatformIO build |
| POST | `/api/ota/deploy` | Deploy firmware OTA |
| GET | `/api/ota/jobs` | List OTA jobs |

---

## Notifications

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/devices/{id}/notifications` | Push notification to device |
| POST | `/api/notifications/webhook` | Webhook (Slack/Teams format) |

---

## Discovery & Diagnostics

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/discovery` | mDNS scan for hookbot devices |
| GET | `/api/diagnostics` | Server + device health checks |
| GET | `/api/health` | Server uptime |

---

## Settings

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/settings` | Get server settings |
| PUT | `/api/settings` | Update settings |
| GET | `/api/logs/stats` | Log statistics |
| DELETE | `/api/logs/prune` | Prune old logs |

---

## Device HTTP API (ESP32)

These endpoints run directly on the ESP32 firmware (default port 80):

| Method | Path | Description |
|--------|------|-------------|
| GET | `/status` | Device state, uptime, heap, firmware version |
| POST | `/state` | Set avatar state |
| POST | `/tasks` | Update task list on OLED |
| POST | `/xp` | Update XP bar on OLED |
| POST | `/notifications` | Show notification badge |
| GET | `/config` | Get device config (brightness, sound, etc.) |
| POST | `/config` | Update device config |
| POST | `/ota` | Receive firmware binary for OTA update |
| GET | `/servos` | Get servo positions |
| POST | `/servos` | Set servo angles |
