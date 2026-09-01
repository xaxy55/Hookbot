# Contributing to Hookbot

Thanks for your interest in contributing! This project welcomes contributions of all kinds — bug fixes, new features, docs, hardware designs, and ideas.

## Getting Started

1. Fork and clone the repo
2. Check the [README](README.md) for setup instructions
3. Look at [ROADMAP.md](ROADMAP.md) for planned features and areas where help is needed

## Development Setup

**Server (Rust):**
```bash
cd server && cargo run
```

**Frontend (React):**
```bash
cd web && npm install && npm run dev
```

**Firmware (ESP32):**
```bash
# Build and flash with PlatformIO
# On first boot, provision WiFi via BLE (device advertises as DeskBot-XXYY)
```

**Everything at once:**
```bash
make up  # Docker Compose
```

## Submitting Changes

1. Create a branch from `main`
2. Make your changes
3. Test locally (see `make test` for Playwright tests)
4. Open a pull request with a clear description of what and why

## Code Style

- **Rust:** `cargo fmt` and `cargo clippy`
- **TypeScript/React:** Follow existing patterns in `web/src/`
- **C++ (firmware):** Follow existing conventions in `firmware/src/`

## Continuous Integration

GitHub Actions are scoped by path so a change only triggers what it affects:

| Workflow | Runs on |
|----------|---------|
| Deploy Web to Cloudflare Pages | pushes to `main` touching `web/**` |
| Infrastructure | pushes/PRs touching `infra/*.tf` |
| Build and Push Docker Images | `v*` tags, or manually via `workflow_dispatch` |
| Deploy Server to GCE | manual only (GCE billing is disabled) |
| App Store Screenshots | manual only |

**The iOS app is built by Xcode Cloud, not by GitHub Actions.** The repo only
holds its hook scripts (`ios/ci_scripts/`); the triggers live in App Store
Connect, so no path filter here can gate them. To stop the app rebuilding on
commits that touch no Apple files, set it in **App Store Connect → Xcode Cloud →
(workflow) → Edit → Start Conditions → Branch Changes → Files and Folders** and
restrict it to `ios/`. Xcode Cloud also honours `[ci skip]` in a commit message,
which is a per-commit escape hatch rather than a fix.

## Reporting Issues

Open a GitHub issue with:
- What you expected vs what happened
- Steps to reproduce
- Hardware details (if firmware-related)

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
