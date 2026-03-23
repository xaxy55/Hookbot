import Foundation

// Polls the management server for state changes — no open ports required.
// Same approach as the Watch: poll /api/devices/{id}/status every 3 seconds.

final class NetworkService: ObservableObject {
    @Published var isConnected = false

    private weak var engine: AvatarEngine?
    private var pollTimer: Timer?
    private var lastKnownState: AvatarState = .idle
    private var lastKnownTool: String = ""
    private var statsPollCount = 0
    private let session: URLSession

    init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 10
        config.waitsForConnectivity = false
        self.session = URLSession(configuration: config)
    }

    // MARK: - Start / Stop

    func start(engine: AvatarEngine) {
        self.engine = engine
        self.lastKnownState = engine.currentState

        // Register with server on first launch, then start polling
        if !engine.config.serverURL.isEmpty {
            registerWithServer()
        }

        startPolling()

        print("[Network] Polling mode — no open ports")
    }

    func stop() {
        pollTimer?.invalidate()
        pollTimer = nil
    }

    func startPolling() {
        pollTimer?.invalidate()
        guard let engine, !engine.config.serverURL.isEmpty, !engine.config.deviceId.isEmpty else {
            return
        }

        // Poll every 3 seconds for state changes
        pollTimer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: true) { [weak self] _ in
            self?.pollState()
        }
        // Initial fetch + heartbeat
        pollState()
        sendHeartbeat()

        print("[Network] Polling \(engine.config.serverURL) for device \(engine.config.deviceId)")
    }

    // MARK: - Polling

    private func pollState() {
        guard let engine,
              !engine.config.serverURL.isEmpty,
              !engine.config.deviceId.isEmpty else { return }

        let urlStr = "\(engine.config.serverURL)/api/devices/\(engine.config.deviceId)/status"
        guard let url = URL(string: urlStr) else { return }

        session.dataTask(with: authedRequest(url: url)) { [weak self] data, response, _ in
            DispatchQueue.main.async {
                guard let self else { return }

                if let http = response as? HTTPURLResponse, http.statusCode == 200, let data {
                    self.isConnected = true
                    self.parseStateResponse(data)
                } else {
                    self.isConnected = false
                }

                // Fetch tasks every 10th poll (~30s), heartbeat every 5th (~15s)
                self.statsPollCount += 1
                if self.statsPollCount % 5 == 0 {
                    self.sendHeartbeat()
                }
                if self.statsPollCount % 10 == 0 {
                    self.pollTasks()
                }
            }
        }.resume()
    }

    private func parseStateResponse(_ data: Data) {
        guard let engine,
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }

        // State is in latest_status.state or top-level
        let statusDict: [String: Any]?
        if let latestStatus = json["latest_status"] as? [String: Any] {
            statusDict = latestStatus
        } else {
            statusDict = json
        }

        guard let statusDict else { return }

        let stateStr = statusDict["state"] as? String
        let toolName = statusDict["tool"] as? String ?? ""
        let toolDetail = statusDict["detail"] as? String ?? ""

        guard let stateStr, let state = AvatarState(rawValue: stateStr) else { return }

        // Only update on change
        if state != lastKnownState || toolName != lastKnownTool {
            lastKnownState = state
            lastKnownTool = toolName

            engine.setTool(name: toolName, detail: toolDetail)
            engine.setState(state)

            // Forward to Watch
            WatchBridge.shared.sendState(state, tool: toolName, detail: toolDetail)
        }
    }

    private func pollTasks() {
        guard let engine,
              !engine.config.serverURL.isEmpty,
              !engine.config.deviceId.isEmpty else { return }

        let urlStr = "\(engine.config.serverURL)/api/devices/\(engine.config.deviceId)/status"
        guard let url = URL(string: urlStr) else { return }

        // Tasks come from the same status endpoint
        session.dataTask(with: authedRequest(url: url)) { [weak self] data, _, _ in
            guard let data,
                  let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let tasks = json["tasks"] as? [[String: Any]] else { return }

            DispatchQueue.main.async {
                guard let self, let engine = self.engine else { return }
                var newTasks: [TaskItem] = []
                for item in tasks {
                    let label = item["label"] as? String ?? ""
                    let status = item["status"] as? Int ?? 0
                    newTasks.append(TaskItem(label: label, status: status))
                }
                engine.tasks = newTasks
                engine.activeTaskIndex = json["active_task"] as? Int ?? 0
            }
        }.resume()
    }

    // MARK: - Heartbeat

    /// Send heartbeat to server so it knows this device is online (cloud mode)
    private func sendHeartbeat() {
        guard let engine,
              !engine.config.serverURL.isEmpty,
              !engine.config.deviceId.isEmpty else { return }

        guard let url = URL(string: "\(engine.config.serverURL)/api/devices/\(engine.config.deviceId)/state") else { return }
        var request = authedRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        let payload: [String: Any] = [
            "state": lastKnownState.rawValue,
            "ip": Self.localIPAddress() ?? "unknown",
            "connection_mode": "cloud"
        ]
        request.httpBody = try? JSONSerialization.data(withJSONObject: payload)

        session.dataTask(with: request) { _, _, _ in }.resume()
    }

    // MARK: - Helpers

    /// Extract an ID from a JSON value that may be a String or a number.
    private func stringID(from value: Any?) -> String? {
        if let s = value as? String, !s.isEmpty { return s }
        if let n = value as? NSNumber { return n.stringValue }
        return nil
    }

    // MARK: - Auth Helper

    private func authedRequest(url: URL) -> URLRequest {
        var request = URLRequest(url: url)
        if let engine, !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }
        return request
    }

    // MARK: - Device Discovery & Registration

    func registerWithServer() {
        guard let engine, !engine.config.serverURL.isEmpty else { return }

        // If we already have a device ID, just start polling
        if !engine.config.deviceId.isEmpty {
            startPolling()
            return
        }

        // First, try to fetch existing devices for this user
        fetchUserDevices { [weak self] in
            guard let self, let engine = self.engine else { return }
            if !engine.config.deviceId.isEmpty {
                // Found an existing device, start polling
                self.startPolling()
            } else {
                // No existing devices — register the iPhone as a new device
                self.registerSelfAsDevice()
            }
        }
    }

    /// Fetch the user's existing devices and auto-select the first one
    private func fetchUserDevices(completion: @escaping () -> Void) {
        guard let engine, !engine.config.serverURL.isEmpty else {
            completion()
            return
        }

        guard let url = URL(string: "\(engine.config.serverURL)/api/devices") else {
            completion()
            return
        }

        session.dataTask(with: authedRequest(url: url)) { [weak self] data, response, _ in
            DispatchQueue.main.async {
                guard let self, let engine = self.engine else {
                    completion()
                    return
                }

                guard let data,
                      let http = response as? HTTPURLResponse,
                      http.statusCode == 200 else {
                    print("[Network] Failed to fetch devices, will register self")
                    completion()
                    return
                }

                // Try parsing as array of devices
                if let devices = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]],
                   let firstDevice = devices.first,
                   let id = self.stringID(from: firstDevice["id"]) {
                    engine.config.deviceId = id
                    if let name = firstDevice["name"] as? String {
                        print("[Network] Auto-selected device: \(name) (\(id))")
                    }
                    if let encoded = try? JSONEncoder().encode(engine.config) {
                        UserDefaults.standard.set(encoded, forKey: "hookbot_config")
                    }
                }
                // Also try parsing as { devices: [...] }
                else if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                        let devices = json["devices"] as? [[String: Any]],
                        let firstDevice = devices.first,
                        let id = self.stringID(from: firstDevice["id"]) {
                    engine.config.deviceId = id
                    if let name = firstDevice["name"] as? String {
                        print("[Network] Auto-selected device: \(name) (\(id))")
                    }
                    if let encoded = try? JSONEncoder().encode(engine.config) {
                        UserDefaults.standard.set(encoded, forKey: "hookbot_config")
                    }
                } else {
                    print("[Network] No existing devices found")
                }

                completion()
            }
        }.resume()
    }

    /// Register the iPhone itself as a new device on the server
    private func registerSelfAsDevice() {
        guard let engine, !engine.config.serverURL.isEmpty else { return }

        let url = URL(string: "\(engine.config.serverURL)/api/devices")!
        var request = authedRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.timeoutInterval = 5

        let payload: [String: Any] = [
            "name": engine.config.deviceName,
            "hostname": engine.config.deviceName,
            "ip_address": "cloud",
            "purpose": "hookbot-\(Self.platformID)",
            "connection_mode": "cloud"
        ]
        request.httpBody = try? JSONSerialization.data(withJSONObject: payload)

        session.dataTask(with: request) { [weak self] data, response, error in
            if let error {
                print("[Network] Registration failed: \(error)")
                return
            }
            if let http = response as? HTTPURLResponse {
                print("[Network] Registered with server: \(http.statusCode)")
            }
            if let data,
               let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let id = self?.stringID(from: json["id"]),
               let engine = self?.engine {
                DispatchQueue.main.async {
                    if engine.config.deviceId.isEmpty {
                        engine.config.deviceId = id
                        if let encoded = try? JSONEncoder().encode(engine.config) {
                            UserDefaults.standard.set(encoded, forKey: "hookbot_config")
                        }
                        print("[Network] Saved device ID: \(id)")
                        self?.startPolling()
                    }
                }
            }
        }.resume()
    }

    // MARK: - Send State (for manual state changes from iPhone UI)

    func sendState(_ state: AvatarState) {
        guard let engine,
              !engine.config.serverURL.isEmpty,
              !engine.config.deviceId.isEmpty else { return }

        guard let url = URL(string: "\(engine.config.serverURL)/api/devices/\(engine.config.deviceId)/state") else { return }
        var request = authedRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try? JSONSerialization.data(withJSONObject: ["state": state.rawValue])

        session.dataTask(with: request) { _, _, _ in }.resume()
    }

    // MARK: - Platform

    static var platformID: String {
        #if targetEnvironment(macCatalyst)
        return "macos"
        #else
        return "ios"
        #endif
    }

    // MARK: - IP Address (kept for settings display)

    static func localIPAddress() -> String? {
        var address: String?
        var ifaddr: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifaddr) == 0, let firstAddr = ifaddr else { return nil }
        defer { freeifaddrs(ifaddr) }

        for ptr in sequence(first: firstAddr, next: { $0.pointee.ifa_next }) {
            let interface = ptr.pointee
            let addrFamily = interface.ifa_addr.pointee.sa_family
            if addrFamily == UInt8(AF_INET) {
                let name = String(cString: interface.ifa_name)
                if name == "en0" || name == "en1" {
                    var hostname = [CChar](repeating: 0, count: Int(NI_MAXHOST))
                    getnameinfo(interface.ifa_addr, socklen_t(interface.ifa_addr.pointee.sa_len),
                               &hostname, socklen_t(hostname.count), nil, 0, NI_NUMERICHOST)
                    address = String(cString: hostname)
                }
            }
        }
        return address
    }
}
