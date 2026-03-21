# Hookbot Desktop Widget

A lightweight system tray widget for macOS and Windows that displays real-time Hookbot device status.

Built with [Tauri v2](https://v2.tauri.app/) — native performance with a tiny footprint.

## Features

- **System tray icon** — click to toggle the status popup
- **Live status polling** — updates every 5 seconds (configurable)
- **Device overview** — online/offline status, current state, IP, uptime
- **XP & gamification** — level, XP bar, streak counter
- **State indicators** — color-coded states (idle, thinking, waiting, success, error)
- **Always-on-top popup** — auto-hides when you click away
- **Settings panel** — configure server URL and API key from the widget

## Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)
  ```bash
  cargo install tauri-cli --version "^2"
  ```
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools, WebView2
  - **Linux**: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`

## Development

```bash
cd desktop

# Run in dev mode (hot reload)
cargo tauri dev

# Build release
cargo tauri build
```

## Configuration

Set environment variables before launching:

| Variable | Default | Description |
|----------|---------|-------------|
| `HOOKBOT_URL` | `http://localhost:3001` | Hookbot server URL |
| `HOOKBOT_API_KEY` | _(none)_ | API key for authenticated servers |
| `HOOKBOT_POLL_INTERVAL` | `5` | Status poll interval in seconds |

Or configure from the settings panel (gear icon) in the widget itself.

## Architecture

```
desktop/
├── src/                    # Frontend (vanilla HTML/CSS/JS)
│   └── index.html          # Widget UI
├── src-tauri/
│   ├── Cargo.toml          # Rust dependencies
│   ├── tauri.conf.json     # Tauri configuration
│   └── src/
│       ├── main.rs         # App setup, tray, polling, IPC commands
│       └── status.rs       # HTTP client for Hookbot API
└── package.json
```

The widget communicates with the Hookbot server via the same REST API used by the web dashboard. The Rust backend polls `/api/devices` and `/api/gamification/stats`, then emits events to the frontend via Tauri's event system.
