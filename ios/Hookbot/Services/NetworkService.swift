import Foundation
<<<<<<< Updated upstream

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
=======
import Network

// HTTP server that listens on port 8080 for state changes
// Same API as the ESP firmware: POST /state, GET /status, POST /config

final class NetworkService: ObservableObject {
    private var listener: NWListener?
    private weak var engine: AvatarEngine?
    private let port: UInt16 = 8080
    private var bonjourService: NetService?

    func startServer(engine: AvatarEngine) {
        self.engine = engine

        do {
            let params = NWParameters.tcp
            listener = try NWListener(using: params, on: NWEndpoint.Port(rawValue: port)!)
        } catch {
            print("[Network] Failed to create listener: \(error)")
            return
        }

        listener?.stateUpdateHandler = { state in
            switch state {
            case .ready:
                print("[Network] HTTP server listening on port \(self.port)")
            case .failed(let error):
                print("[Network] Listener failed: \(error)")
            default:
                break
            }
        }

        listener?.newConnectionHandler = { [weak self] connection in
            self?.handleConnection(connection)
        }

        listener?.start(queue: .global(qos: .userInitiated))
        print("[Network] Starting HTTP server on port \(port)")

        // Advertise via Bonjour/mDNS so the management server can discover us
        startBonjourAdvertising(name: engine.config.deviceName)

        // Register with management server if configured
        if !engine.config.serverURL.isEmpty {
            registerWithServer()
        }
    }

    func stop() {
        listener?.cancel()
        listener = nil
        bonjourService?.stop()
        bonjourService = nil
    }

    // MARK: - Bonjour/mDNS Advertising

    private func startBonjourAdvertising(name: String) {
        // Advertise as _http._tcp so the management server's mDNS scan finds us
        // Hostname must start with "hookbot" to match the server's filter
        let serviceName = name.hasPrefix("hookbot") ? name : "hookbot-ios"
        bonjourService = NetService(domain: "local.", type: "_http._tcp.", name: serviceName, port: Int32(port))
        bonjourService?.publish()
        print("[Network] Bonjour advertising: \(serviceName)._http._tcp.local. on port \(port)")
    }

    // MARK: - Connection Handling

    private func handleConnection(_ connection: NWConnection) {
        connection.start(queue: .global(qos: .userInitiated))

        connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, _, _, error in
            guard let self, let data, error == nil else {
                connection.cancel()
                return
            }

            let request = String(data: data, encoding: .utf8) ?? ""
            let response = self.routeRequest(request)
            let httpResponse = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\nContent-Length: \(response.utf8.count)\r\n\r\n\(response)"

            connection.send(content: httpResponse.data(using: .utf8), completion: .contentProcessed { _ in
                connection.cancel()
            })
        }
    }

    private func routeRequest(_ raw: String) -> String {
        let lines = raw.split(separator: "\r\n", maxSplits: 1)
        guard let firstLine = lines.first else { return "{\"error\":\"bad request\"}" }

        let parts = firstLine.split(separator: " ")
        guard parts.count >= 2 else { return "{\"error\":\"bad request\"}" }

        let method = String(parts[0])
        let path = String(parts[1])

        // Extract JSON body (after empty line)
        var body: [String: Any] = [:]
        if let range = raw.range(of: "\r\n\r\n") {
            let jsonStr = String(raw[range.upperBound...])
            if let jsonData = jsonStr.data(using: .utf8),
               let json = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] {
                body = json
            }
        }

        switch (method, path) {
        case ("GET", "/status"):
            return handleStatus()
        case ("GET", "/info"):
            return handleInfo()
        case ("POST", "/state"):
            return handleState(body)
        case ("POST", "/config"):
            return handleConfig(body)
        case ("POST", "/tasks"):
            return handleTasks(body)
        case ("GET", "/"):
            return "{\"name\":\"Hookbot iOS\",\"version\":\"1.0.0\"}"
        default:
            // Handle CORS preflight
            if method == "OPTIONS" {
                return "{\"ok\":true}"
            }
            return "{\"error\":\"not found\"}"
        }
    }

    // MARK: - Route Handlers

    private func handleStatus() -> String {
        guard let engine else { return "{\"error\":\"not ready\"}" }
        let uptime = ProcessInfo.processInfo.systemUptime
        return """
        {"state":"\(engine.currentState.rawValue)","uptime":\(Int(uptime * 1000)),"ip":"\(Self.localIPAddress() ?? "unknown")","hostname":"\(engine.config.deviceName)","firmware_version":"ios-1.0.0","platform":"ios"}
        """
    }

    private func handleInfo() -> String {
        guard let engine else { return "{\"error\":\"not ready\"}" }
        return """
        {"hostname":"\(engine.config.deviceName)","firmware_version":"ios-1.0.0","ip":"\(Self.localIPAddress() ?? "unknown")","platform":"ios","capabilities":["display","sound","haptics"]}
        """
    }

    private func handleState(_ body: [String: Any]) -> String {
        guard let engine else { return "{\"error\":\"not ready\"}" }
        let stateStr = body["state"] as? String ?? "idle"
        let toolName = body["tool"] as? String ?? ""
        let toolDetail = body["detail"] as? String ?? ""

        if let state = AvatarState(rawValue: stateStr) {
            DispatchQueue.main.async {
                engine.setTool(name: toolName, detail: toolDetail)
                engine.setState(state)
            }
            return "{\"ok\":true,\"state\":\"\(stateStr)\",\"tool\":\"\(toolName)\"}"
        }
        return "{\"error\":\"invalid state\"}"
    }

    private func handleConfig(_ body: [String: Any]) -> String {
        guard let engine else { return "{\"error\":\"not ready\"}" }

        DispatchQueue.main.async {
            if let soundEnabled = body["sound_enabled"] as? Bool {
                engine.config.soundEnabled = soundEnabled
            }
            if let preset = body["avatar_preset"] as? [String: Any],
               let acc = preset["accessories"] as? [String: Any] {
                if let v = acc["topHat"] as? Bool  { engine.config.accessories.topHat = v }
                if let v = acc["cigar"] as? Bool    { engine.config.accessories.cigar = v }
                if let v = acc["glasses"] as? Bool  { engine.config.accessories.glasses = v }
                if let v = acc["monocle"] as? Bool  { engine.config.accessories.monocle = v }
                if let v = acc["bowtie"] as? Bool   { engine.config.accessories.bowtie = v }
                if let v = acc["crown"] as? Bool    { engine.config.accessories.crown = v }
                if let v = acc["horns"] as? Bool    { engine.config.accessories.horns = v }
                if let v = acc["halo"] as? Bool     { engine.config.accessories.halo = v }
            }
            // Persist
            if let data = try? JSONEncoder().encode(engine.config) {
                UserDefaults.standard.set(data, forKey: "hookbot_config")
            }
        }

        return "{\"ok\":true}"
    }

    private func handleTasks(_ body: [String: Any]) -> String {
        guard let engine else { return "{\"error\":\"not ready\"}" }

        DispatchQueue.main.async {
            var newTasks: [TaskItem] = []
            if let items = body["items"] as? [[String: Any]] {
                for item in items {
>>>>>>> Stashed changes
                    let label = item["label"] as? String ?? ""
                    let status = item["status"] as? Int ?? 0
                    newTasks.append(TaskItem(label: label, status: status))
                }
<<<<<<< Updated upstream
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
=======
            }
            engine.tasks = newTasks
            engine.activeTaskIndex = body["active"] as? Int ?? 0
        }

        return "{\"ok\":true}"
    }

    // MARK: - Management Server Registration

    private func registerWithServer() {
        guard let engine, !engine.config.serverURL.isEmpty else { return }

        let url = URL(string: "\(engine.config.serverURL)/api/devices")!
        var request = URLRequest(url: url)
>>>>>>> Stashed changes
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.timeoutInterval = 5

        let payload: [String: Any] = [
            "name": engine.config.deviceName,
            "hostname": engine.config.deviceName,
<<<<<<< Updated upstream
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
=======
            "ip_address": Self.localIPAddress() ?? "unknown",
            "purpose": "hookbot-ios"
        ]
        request.httpBody = try? JSONSerialization.data(withJSONObject: payload)

        URLSession.shared.dataTask(with: request) { _, response, error in
            if let error {
                print("[Network] Registration failed: \(error)")
            } else if let http = response as? HTTPURLResponse {
                print("[Network] Registered with server: \(http.statusCode)")
            }
        }.resume()
    }

    // MARK: - IP Address
>>>>>>> Stashed changes

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
