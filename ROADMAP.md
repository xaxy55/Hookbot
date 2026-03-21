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

## Phase 7: Social & Multiplayer

> Turn solo coding into a shared experience. Hookbots that interact across the internet.

- [x] **Buddy system** — Pair two hookbots over the internet: your bot mirrors your buddy's mood, see when they're coding
- [x] **Raid mode** — Send your avatar to "visit" a friend's hookbot OLED for 30 seconds with a custom message
- [x] **Shared streaks** — Collaborative streak challenges: both users must code daily to keep the streak alive
- [x] **Live coding indicator** — Hookbot glows when friends are actively coding, like a campfire you gather around
- [x] **Team dashboard** — Shared web view showing all team hookbots, who's in flow state, who needs help
- [x] **Hookbot reactions** — Send quick emoji reactions to someone's hookbot (fireworks for a deploy, skull for a crash)
- [x] **Global event wall** — Opt-in anonymous feed of hookbot milestones worldwide ("someone just hit level 50!")

## Phase 8: Desk Ecosystem & Smart Home

> The hookbot becomes the brain of your desk setup and connects to your smart home.

- [x] **Desk lighting sync** — Control Philips Hue / WLED strips: red for errors, green for passing tests, ambient for focus mode
- [x] **Spotify / Apple Music integration** — Auto-pause music on meetings, show now-playing on e-ink, focus playlists by state
- [x] **Standing desk integration** — Track sit/stand time, remind to switch, hookbot does a little celebration when you stand
- [x] **Stream Deck plugin** — Custom buttons for hookbot actions: change state, trigger animations, deploy OTA
- [x] **Home Assistant integration** — Expose hookbot as HA entity: use in automations, voice control via Alexa/Google
- [x] **Desk occupancy analytics** — Track desk usage patterns, suggest optimal break times, weekly desk health report
- [x] **Multi-monitor awareness** — Detect active monitor via USB, point servo-driven hookbot head toward the screen you're using

## Phase 9: Mini-Games & Easter Eggs

> Reward breaks, fight burnout, hide delightful surprises.

- [ ] **Tamagotchi mode** — Feed your hookbot by coding, it gets sad if ignored, evolves into different forms based on your coding style
- [ ] **OLED mini-games** — Snake, Pong, Tetris on the OLED during breaks, controlled via physical buttons or web UI
- [ ] **Boss battle** — Weekly "boss bug" challenge: fix a puzzle bug to earn bonus XP, hookbot cheers you on
- [ ] **Konami code** — Enter the classic code on physical buttons for a secret animation and hidden achievement
- [ ] **Avatar evolution** — Your avatar visually evolves every 10 levels: egg → blob → robot → mech → cosmic entity
- [ ] **Loot drops** — Random rare accessories and animations drop from normal coding activity
- [ ] **Seasonal events** — Halloween spooky mode, holiday themes, April Fools chaos mode with inverted controls
- [ ] **Typing speed mini-game** — Hookbot times your WPM during intense coding bursts, awards "speed demon" variants
- [ ] **Idle animations** — Hookbot does progressively weirder things the longer you're AFK (juggles, naps, builds a tiny house)

## Phase 10: Developer Analytics & Insights

> Deep productivity intelligence. Understand your coding patterns like never before.

- [ ] **Flow state detection** — ML model trained on your patterns: detect when you enter/exit flow, protect it aggressively
- [ ] **Code quality correlation** — Track bug rate vs. time of day, sleep, breaks: find your optimal coding conditions
- [ ] **Weekly AI digest** — Claude-generated weekly summary: what you shipped, patterns spotted, personalized tips
- [ ] **Burnout early warning** — Detect overwork patterns (late nights, no breaks, declining streak quality) and alert gently
- [ ] **Project time tracking** — Automatic per-project time tracking from hook data, exportable timesheets
- [ ] **Pair programming stats** — Detect pairing sessions, track effectiveness, suggest optimal pairing schedules
- [ ] **Retrospective generator** — Auto-generate sprint retro talking points from hookbot data

## Phase 11: Advanced Hardware Mods

> For the tinkerers who want to push the hardware to its limits.

- [ ] **Motorized base** — Hookbot can rotate to face you using a stepper motor and presence detection
- [ ] **Thermal printer** — Tiny receipt printer for daily standup notes, achievement certificates, or code snippets
- [ ] **LED matrix face** — 32x32 RGB LED matrix as an alternative face: pixel art expressions, scrolling text
- [ ] **Mechanical keyboard integration** — Custom key switch on hookbot: macro key for deploy, emergency stop, or celebration
- [ ] **Desk fan control** — PWM-controlled fan that spins up when your code is "hot" (lots of errors) and slows on clean builds
- [ ] **Modular snap-on accessories** — Magnetic mounts for swappable physical accessories: hats, arms, signs
- [ ] **Wireless charging pad** — Qi charging built into the base, also charges your phone while hookbot sits on it

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
- [x] Confetti animation on successful deploy
- [x] "Do Not Disturb" toggle (software toggle in web UI + firmware)
- [x] Git branch name on OLED when switching branches
- [ ] Compile progress bar on OLED during builds
- [x] Random motivational quotes on idle screen
- [x] Customizable startup boot animation
- [ ] Battery backup with low-power sleep mode
- [ ] QR code display for quick device pairing
- [ ] Desktop widget (macOS/Windows) showing hookbot status
