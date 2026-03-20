# Hookbot Roadmap

## What's Built (v0.2.0)

- ESP32 firmware: animated OLED avatar (6 states, 8 accessories, face params), LED, buzzer, servo support, task list overlay, notification badges, WiFi/mDNS, NVS config, HTTP OTA
- Rust management server: 25+ API routes, SQLite, device poller, mDNS discovery, OTA deploy, diagnostics
- React frontend: 11 pages (dashboard, device detail with 5 tabs, avatar editor, animation editor, OTA, discovery, integrations, settings, logs, diagnostics)
- Hook system: Claude Code integration with server/direct routing modes
- Docker Compose deployment

---

## Phase 1: Polish & Stability

> Finish what's started, fix rough edges, make it reliable for daily use.

- [x] **Servo control UI** — Add a "Servos" tab to Device Detail with angle sliders, per-state map editor, and pin configuration form
- [x] **LED color editor** — Per-state RGB color picker in the personality tab, push to device
- [x] **Custom animation playback on device** — Firmware endpoint to receive and play keyframe animations from the animation editor
- [x] **Notification persistence** — Store notification sources/webhook configs in SQLite, not just in-memory
- [x] **Auto-discovery on startup** — Server scans mDNS on boot and registers new devices automatically
- [x] **Connection resilience** — Exponential backoff on poller failures, WiFi reconnect improvements on ESP32
- [x] **Status log pruning** — Configurable retention period, auto-cleanup old entries
- [x] **Error toasts in frontend** — Show API errors as dismissable notifications instead of inline text

## Phase 2: Gamification & Analytics

> Make it fun and insightful. Track usage, reward productivity.

- [x] **XP & Leveling system** — Earn XP per tool use, completed task, successful build. Level up the hookbot (unlocks accessories, animations, titles)
- [x] **Usage tracking database** — New tables: `tool_uses` (tool_name, timestamp, duration_ms, project), `sessions` (start, end, tools_used, states), `achievements`
- [x] **Activity feed** — Real-time log of hook events with tool icons and timestamps
- [x] **Analytics dashboard** — Charts for: tools used per day, active coding hours, state distribution pie chart, session length trends, heap/uptime over time
- [x] **Streak tracker** — Daily coding streak counter, displayed on OLED and dashboard
- [x] **Achievement badges** — "First OTA", "100 tool calls", "24h uptime", "Night owl" (coding past midnight), "Speed demon" (10 builds in an hour)
- [x] **Leaderboard** — If multiple users/devices, compare XP and streaks
- [x] **OLED level display** — Show current level and XP bar on idle screen

## Phase 3: Sensors & Automation (IFTTT)

> React to the physical world. User-configurable triggers and actions.

- [x] **Sensor framework** — GPIO input abstraction: digital (button, PIR motion), analog (light, temp, potentiometer), I2C (BME280, etc.)
- [x] **Sensor config UI** — Frontend page to add/configure sensors: pin, type, label, polling interval, threshold
- [x] **Trigger engine** — IF-THIS-THEN-THAT rules stored in SQLite:
  - Triggers: sensor threshold, time of day, device state change, webhook received, button press
  - Actions: change avatar state, play animation, move servo, send notification, call webhook, play sound
- [x] **Rule editor UI** — Visual drag-and-drop or form-based rule builder with live preview
- [x] **Sensor data logging** — Store readings in time-series table, display graphs on frontend
- [x] **Physical button support** — Configurable GPIO button: short press = cycle state, long press = custom action
- [x] **Presence detection** — PIR or ultrasonic sensor: sleep avatar when away, wake on return
- [x] **Ambient light** — Auto-adjust LED brightness based on room light level

## Phase 3.5: Community Plugin Store & Asset Sharing

> Open marketplace for community-created plugins, shared avatars, animations, and screensavers.

- [x] **Community plugin registry** — SQLite-backed catalog for user-submitted plugins with name, description, author, version, category, and download count
- [x] **Plugin publishing** — Upload new plugins with metadata, auto-assigned unique IDs
- [x] **Plugin installation tracking** — Per-device install/uninstall with version pinning
- [x] **Plugin ratings & reviews** — Star ratings (1-5) per device, average score displayed
- [x] **Shared asset library** — Community-uploaded avatars, animations, and screensavers with JSON payloads
- [x] **Asset publishing** — Upload custom avatar presets, keyframe animations, screensaver configs for others to use
- [x] **Asset installation** — Browse, preview metadata, and install shared assets to your device
- [x] **Asset ratings** — Rate and sort shared assets by popularity
- [x] **Community Store page** — Frontend browse/search/install UI for community plugins
- [x] **Asset Sharing page** — Frontend browse/upload/install UI for shared avatars & animations
- [x] **Plugin sandboxing** — Run community plugins in isolated contexts with permission scoping
- [x] **Verified publishers** — Badge system for trusted community plugin authors

## Phase 4: Multi-Device & Collaboration

> Scale from one hookbot to a fleet. Different bots for different jobs.

- [x] **Per-project device routing** — `.hookbot` config maps workspaces to specific devices (already partially built)
- [x] **Device groups** — Tag devices, send commands to groups ("all calendar bots")
- [x] **Device-to-device communication** — One bot's state change triggers another (e.g. "error on build bot" -> "alert on notification bot")
- [x] **Shared animation library** — Upload/download community animations, sync across devices
- [x] **Multi-user support** — User accounts, per-user device assignments, auth tokens
- [x] **Remote access** — Tunnel/proxy for controlling hookbots outside LAN (Cloudflare Tunnel or similar)
- [x] **Mobile app** — Native iOS companion (iPhone + Apple Watch) for quick state changes and notifications on the go

## Phase 5: AI & Intelligence

> Make the hookbot smarter. Context-aware reactions, natural language control.

- [x] **Claude API integration** — Hookbot can ask Claude for status summaries, generate witty status messages
- [x] **Context-aware reactions** — Analyze tool patterns: lots of grep = "searching", many edits = "refactoring", test failures = escalating frustration
- [x] **Voice control** — I2S microphone + wake word detection, send commands to Claude
- [x] **Text-to-speech** — I2S DAC speaker, Claude speaks responses through the hookbot
- [x] **Smart notifications** — AI decides priority: suppress low-importance Teams messages during deep work, escalate urgent ones
- [x] **Mood learning** — Track which states/animations the user responds to, adapt personality over time
- [x] **Meeting awareness** — Calendar integration: go quiet during meetings, show countdown to next meeting

## Phase 6: Hardware V2

> Physical upgrades for a more expressive, interactive desk companion.

- [ ] **Larger display** — 1.3" or 2.4" OLED/LCD for richer avatar graphics and more info
- [ ] **E-ink sidebar** — Small e-ink display for persistent info (today's stats, next meeting, streak count)
- [ ] **RGB LED matrix** — Replace single LED with 8x8 or ring for ambient patterns
- [ ] **Custom PCB** — Integrated board with ESP32-S3, display connector, servo headers, sensor ports, USB-C
- [ ] **3D printed enclosure** — Desk-friendly housing with articulated head/arms, cable management
- [ ] **Haptic feedback** — Vibration motor for subtle alerts (desk vibration on error)
- [ ] **NFC tag** — Tap phone to pair, change profiles, or trigger actions

---

## Quick Wins (anytime)

- [x] Dark/light theme actually works (CSS variables for light mode)
- [x] Firmware version shown on OLED idle screen
- [x] Device uptime display in human-readable format on OLED
- [x] Keyboard shortcut to trigger states from web UI
- [x] Export/import device config as JSON backup
- [x] Bulk OTA deploy with progress bar
- [x] Webhook templates for common integrations (Slack, Discord, GitHub Actions)
- [x] Sound pack system — upload custom melodies per state
- [x] OLED screen saver — random animations after extended idle
