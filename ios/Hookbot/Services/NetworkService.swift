import Foundation
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
                    let label = item["label"] as? String ?? ""
                    let status = item["status"] as? Int ?? 0
                    newTasks.append(TaskItem(label: label, status: status))
                }
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
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.timeoutInterval = 5

        let payload: [String: Any] = [
            "name": engine.config.deviceName,
            "hostname": engine.config.deviceName,
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
