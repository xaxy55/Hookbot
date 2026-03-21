import Foundation
import Combine
import Security
import WidgetKit

/// Independent network service for watchOS — polls the management server directly
/// so the watch works even without the iPhone nearby
final class WatchNetworkService: ObservableObject {

    @Published var stats: GamificationStats?
    @Published var recentActivity: [ActivityEntry] = []
    @Published var isConnected = false
    @Published var serverURL: String = "" {
        didSet {
            UserDefaults.standard.set(serverURL, forKey: "hookbot_watch_server")
            if !serverURL.isEmpty { startPolling() }
        }
    }
    @Published var deviceId: String = "" {
        didSet {
            UserDefaults.standard.set(deviceId, forKey: "hookbot_watch_device")
        }
    }
    @Published var apiKey: String = "" {
        didSet {
            Self.saveToKeychain(key: "hookbot_watch_api_key", value: apiKey)
        }
    }

    // MARK: - Keychain helpers

    private static let keychainService = "com.hookbot.watch"

    private static func saveToKeychain(key: String, value: String) {
        let data = Data(value.utf8)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: key,
        ]
        SecItemDelete(query as CFDictionary)
        guard !value.isEmpty else { return }
        var add = query
        add[kSecValueData as String] = data
        add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
        SecItemAdd(add as CFDictionary, nil)
    }

    private static func loadFromKeychain(key: String) -> String {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data else { return "" }
        return String(data: data, encoding: .utf8) ?? ""
    }

    private var pollTimer: Timer?
    private var lastKnownState: AvatarState = .idle
    weak var engine: AvatarEngine?
    private let session: URLSession

    init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 10
        config.waitsForConnectivity = false
        self.session = URLSession(configuration: config)

        self.serverURL = UserDefaults.standard.string(forKey: "hookbot_watch_server") ?? ""
        self.deviceId = UserDefaults.standard.string(forKey: "hookbot_watch_device") ?? ""
        self.apiKey = Self.loadFromKeychain(key: "hookbot_watch_api_key")
    }

    private func authedRequest(url: URL) -> URLRequest {
        var request = URLRequest(url: url)
        if !apiKey.isEmpty {
            request.setValue(apiKey, forHTTPHeaderField: "X-API-Key")
        }
        return request
    }

    // MARK: - Polling

    func startPolling() {
        pollTimer?.invalidate()
        guard !serverURL.isEmpty else { return }

        // Poll every 3 seconds for state, every 30 seconds for stats
        pollTimer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: true) { [weak self] _ in
            self?.pollState()
        }
        // Initial fetch
        pollState()
        fetchStats()
        fetchActivity()
    }

    func stopPolling() {
        pollTimer?.invalidate()
        pollTimer = nil
    }

    private var statsPollCount = 0

    private func pollState() {
        guard !serverURL.isEmpty, !deviceId.isEmpty else { return }

        guard let url = URL(string: "\(serverURL)/api/devices/\(deviceId)/status") else { return }
        session.dataTask(with: authedRequest(url: url)) { [weak self] data, response, _ in
            DispatchQueue.main.async {
                guard let self else { return }

                if let http = response as? HTTPURLResponse, http.statusCode == 200, let data {
                    self.isConnected = true
                    self.parseStateResponse(data)
                } else {
                    self.isConnected = false
                }

                // Fetch stats every 10th poll (~30s)
                self.statsPollCount += 1
                if self.statsPollCount % 10 == 0 {
                    self.fetchStats()
                    self.fetchActivity()
                }
            }
        }.resume()
    }

    private func parseStateResponse(_ data: Data) {
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }

        // Check for state in the latest_status field or top-level
        let stateStr: String?
        if let latestStatus = json["latest_status"] as? [String: Any] {
            stateStr = latestStatus["state"] as? String
        } else {
            stateStr = json["state"] as? String
        }

        guard let stateStr, let state = AvatarState(rawValue: stateStr) else { return }

        if state != lastKnownState {
            lastKnownState = state
            engine?.setState(state)
            HapticManager.shared.playStateHaptic(state)

            // Persist for complications
            UserDefaults.standard.set(stateStr, forKey: "hookbot_last_state")
            WidgetCenter.shared.reloadTimelines(ofKind: "HookbotComplication")
        }
    }

    // MARK: - Stats

    func fetchStats() {
        guard !serverURL.isEmpty else { return }

        let deviceQuery = deviceId.isEmpty ? "" : "?device_id=\(deviceId)"
        guard let url = URL(string: "\(serverURL)/api/gamification/stats\(deviceQuery)") else { return }

        session.dataTask(with: authedRequest(url: url)) { [weak self] data, response, _ in
            guard let data,
                  let http = response as? HTTPURLResponse, http.statusCode == 200 else { return }

            if let decoded = try? JSONDecoder().decode(GamificationStats.self, from: data) {
                let oldLevel = self?.stats?.level ?? 0
                DispatchQueue.main.async {
                    self?.stats = decoded
                    if oldLevel > 0 && decoded.level > oldLevel {
                        HapticManager.shared.playLevelUp()
                    }
                    // Persist for complications
                    UserDefaults.standard.set(decoded.level, forKey: "hookbot_level")
                    UserDefaults.standard.set(decoded.total_xp, forKey: "hookbot_xp")
                    UserDefaults.standard.set(decoded.current_streak, forKey: "hookbot_streak")
                }
            }
        }.resume()
    }

    func fetchActivity() {
        guard !serverURL.isEmpty else { return }

        let deviceQuery = deviceId.isEmpty ? "?limit=10" : "?device_id=\(deviceId)&limit=10"
        guard let url = URL(string: "\(serverURL)/api/gamification/activity\(deviceQuery)") else { return }

        session.dataTask(with: authedRequest(url: url)) { [weak self] data, response, _ in
            guard let data,
                  let http = response as? HTTPURLResponse, http.statusCode == 200 else { return }

            if let decoded = try? JSONDecoder().decode([ActivityEntry].self, from: data) {
                DispatchQueue.main.async {
                    self?.recentActivity = decoded
                }
            }
        }.resume()
    }

    // MARK: - Send State (from watch buttons)

    func sendState(_ state: AvatarState) {
        guard !serverURL.isEmpty, !deviceId.isEmpty else { return }

        guard let url = URL(string: "\(serverURL)/api/devices/\(deviceId)/state") else { return }
        var request = authedRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try? JSONSerialization.data(withJSONObject: ["state": state.rawValue])

        session.dataTask(with: request) { _, _, _ in }.resume()
    }
}

// MARK: - API Models for watchOS

struct GamificationStats: Codable {
    let total_xp: Int
    let level: Int
    let xp_for_current_level: Int
    let xp_for_next_level: Int
    let total_tool_uses: Int
    let current_streak: Int
    let longest_streak: Int
    let achievements_earned: Int
    let title: String
}

struct ActivityEntry: Codable, Identifiable {
    let id: Int
    let tool_name: String
    let event: String
    let xp_earned: Int
    let created_at: String
    let device_id: String?
}
