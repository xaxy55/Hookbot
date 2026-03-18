# Security Policy

## Supported Versions

Use this section to tell people about which versions of your project are
currently being supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| 5.1.x   | :white_check_mark: |
| 5.0.x   | :x:                |
| 4.0.x   | :white_check_mark: |
| < 4.0   | :x:                |

## Reporting a Vulnerability

Use this section to tell people how to report a vulnerability.

Tell them where to go, how often they can expect to get an update on a
reported vulnerability, what to expect if the vulnerability is accepted or
declined, etc.
# Security Policy

## Scope

This policy covers the Hookbot project, which consists of:

- **Firmware** — C++ / Arduino / PlatformIO running on ESP32
- - **Server** — Rust / Axum / SQLite backend (port 3000)
  - - **Web UI** — React 19 / TypeScript / Vite dashboard (port 5173)
    - - **iOS App** — Swift client for iPhone and Apple Watch
      - - **Hooks** — Node.js Claude Code integration scripts
       
        - ## Supported Versions
       
        - Hookbot is currently in active development. Security fixes are applied to the latest version on the `main` branch only.
       
        - | Component | Supported |
        - | --------- | --------- |
        - | Latest (`main`) | :white_check_mark: |
        - | Older commits | :x: |
       
        - ## Known Security Considerations
       
        - Hookbot is designed for **local network use only**. Please be aware of the following before deploying:
       
        - - The Rust server binds to all network interfaces (`0.0.0.0`) by default and uses permissive CORS. **Do not expose the server to the public internet** without adding authentication and restricting CORS origins.
          - - WiFi credentials are provisioned over BLE on first boot. Ensure you are in a trusted environment when setting up a new device.
            - - OTA firmware updates are pushed over WiFi from the web dashboard with no authentication by default. Restrict dashboard access on shared networks.
              - - The Claude Code hooks execute on your development machine and communicate with the local server over HTTP. Ensure the server is not reachable from untrusted hosts.
                - - No authentication is implemented on the REST API by default. Do not run Hookbot on a network with untrusted users.
                 
                  - ## Reporting a Vulnerability
                 
                  - If you discover a security vulnerability in Hookbot, please report it responsibly:
                 
                  - 1. **Do not open a public GitHub issue** for security vulnerabilities.
                    2. 2. Use GitHub's [private vulnerability reporting](https://github.com/xaxy55/Hookbot/security/advisories/new) to submit a report confidentially.
                       3. 3. Include as much detail as possible: affected component (firmware, server, web UI, iOS, hooks), steps to reproduce, potential impact, and any suggested fix.
                         
                          4. You can expect an acknowledgement within **72 hours** and a status update within **7 days**. If the vulnerability is accepted, a fix will be prioritised for the next release. If declined, you will receive a clear explanation.
                         
                          5. ## Out of Scope
                         
                          6. The following are considered out of scope for this project's security policy:
                         
                          7. - Issues requiring physical access to the ESP32 device
                             - - Vulnerabilities in third-party dependencies (please report those upstream)
                               - - Attacks that require the attacker to already have access to the local network with elevated privileges
