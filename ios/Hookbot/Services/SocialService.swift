import Foundation
import Combine
import UIKit

// Social layer — buddy pairing, presence, emotes, gifting, raids (Phase 13.2 + 13.4)

// MARK: - Models

struct Friend: Codable, Identifiable {
    let id: String
    let username: String
    var avatarState: String
    var streak: Int
    var xp: Int
    var isOnline: Bool
    var lastActivity: String
    var pairingCode: String?
}

struct AvatarVisit: Codable, Identifiable {
    let id: String
    let fromFriendId: String
    let fromFriendName: String
    let message: String
    let gift: String?
    let timestamp: Date
    var isRead: Bool
}

struct CodingChallenge: Codable, Identifiable {
    let id: String
    let challengerId: String
    let challengerName: String
    let type: String   // "tests_written", "commits", "xp_earned"
    let metric: String
    let startDate: Date
    let endDate: Date
    var myScore: Int
    var theirScore: Int
    let isActive: Bool
}

enum AvatarEmote: String, Codable, CaseIterable {
    case wave       = "wave"
    case fireworks  = "fireworks"
    case skull      = "skull"
    case party      = "party"
    case hug        = "hug"
    case highFive   = "high_five"
    case applause   = "applause"

    var emoji: String {
        switch self {
        case .wave:      return "👋"
        case .fireworks: return "🎆"
        case .skull:     return "💀"
        case .party:     return "🎉"
        case .hug:       return "🤗"
        case .highFive:  return "🙏"
        case .applause:  return "👏"
        }
    }

    var displayName: String {
        switch self {
        case .wave:      return "Wave"
        case .fireworks: return "Fireworks"
        case .skull:     return "Skull"
        case .party:     return "Party"
        case .hug:       return "Hug"
        case .highFive:  return "High Five"
        case .applause:  return "Applause"
        }
    }
}

// MARK: - Pairing State

enum PairingState {
    case idle
    case generating(code: String, qrImage: UIImage?)
    case waiting
    case paired(friend: Friend)
    case failed(error: String)
}

// MARK: - Service

@MainActor
final class SocialService: ObservableObject {

    static let shared = SocialService()

    @Published var friends: [Friend] = []
    @Published var visitHistory: [AvatarVisit] = []
    @Published var pendingChallenges: [CodingChallenge] = []
    @Published var activeChallenges: [CodingChallenge] = []
    @Published var pairingState: PairingState = .idle
    @Published var unreadVisits = 0
    @Published var incomingRaid: (fromName: String, emote: AvatarEmote)?

    private weak var engine: AvatarEngine?
    private var serverURL: String = ""
    private var apiKey: String = ""
    private var deviceId: String = ""
    private var presencePollTimer: Timer?

    private init() {}

    func configure(engine: AvatarEngine) {
        self.engine = engine
        self.serverURL = engine.config.serverURL
        self.apiKey = engine.config.apiKey
        self.deviceId = engine.config.deviceId
    }

    func start() {
        loadCachedData()
        startPresencePolling()
    }

    func stop() {
        presencePollTimer?.invalidate()
        presencePollTimer = nil
    }

    // MARK: - Friend List & Presence

    func refreshFriends() {
        guard !serverURL.isEmpty else { return }
        let url = URL(string: "\(serverURL)/api/social/friends?device_id=\(deviceId)")!
        let req = authedRequest(url: url)
        URLSession.shared.dataTask(with: req) { [weak self] data, _, _ in
            guard let data,
                  let decoded = try? JSONDecoder().decode([Friend].self, from: data) else { return }
            DispatchQueue.main.async {
                self?.friends = decoded
                self?.cacheFriends(decoded)
            }
        }.resume()
    }

    private func startPresencePolling() {
        presencePollTimer?.invalidate()
        presencePollTimer = Timer.scheduledTimer(withTimeInterval: 30, repeats: true) { [weak self] _ in
            Task { await self?.refreshFriends() }
        }
    }

    // MARK: - Buddy Pairing (QR code or NFC)

    func generatePairingCode() {
        guard !serverURL.isEmpty else { return }
        pairingState = .waiting

        let url = URL(string: "\(serverURL)/api/social/pair/generate")!
        var req = authedRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try? JSONSerialization.data(withJSONObject: ["device_id": deviceId])

        URLSession.shared.dataTask(with: req) { [weak self] data, _, error in
            DispatchQueue.main.async {
                guard let self else { return }
                if let data,
                   let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let code = json["code"] as? String {
                    let qr = self.generateQRCode(from: code)
                    self.pairingState = .generating(code: code, qrImage: qr)
                    self.pollForPairingCompletion(code: code)
                } else {
                    self.pairingState = .failed(error: error?.localizedDescription ?? "Unknown error")
                }
            }
        }.resume()
    }

    func acceptPairingCode(_ code: String) {
        guard !serverURL.isEmpty else { return }
        let url = URL(string: "\(serverURL)/api/social/pair/accept")!
        var req = authedRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try? JSONSerialization.data(withJSONObject: [
            "device_id": deviceId,
            "code": code
        ])

        URLSession.shared.dataTask(with: req) { [weak self] data, _, _ in
            guard let data,
                  let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let friendData = json["friend"] as? [String: Any],
                  let friend = try? JSONDecoder().decode(Friend.self, from: JSONSerialization.data(withJSONObject: friendData)) else { return }
            DispatchQueue.main.async {
                self?.pairingState = .paired(friend: friend)
                self?.friends.append(friend)
                HapticManager.playEvent(.friendVisit)
            }
        }.resume()
    }

    private func pollForPairingCompletion(code: String) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in
            guard let self, case .generating = self.pairingState else { return }
            let url = URL(string: "\(self.serverURL)/api/social/pair/status?code=\(code)")!
            URLSession.shared.dataTask(with: self.authedRequest(url: url)) { data, _, _ in
                guard let data,
                      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                    Task { @MainActor in
                        self.pollForPairingCompletion(code: code)
                    }
                    return
                }
                DispatchQueue.main.async {
                    if let friendData = json["friend"] as? [String: Any],
                       let friend = try? JSONDecoder().decode(
                           Friend.self,
                           from: JSONSerialization.data(withJSONObject: friendData)
                       ) {
                        self.pairingState = .paired(friend: friend)
                        self.friends.append(friend)
                        HapticManager.playEvent(.friendVisit)
                        NotificationManager.shared.notifyFriendVisit(from: friend.username, message: "Pairing complete!")
                    } else if let status = json["status"] as? String, status == "pending" {
                        self.pollForPairingCompletion(code: code)
                    }
                }
            }.resume()
        }
    }

    // MARK: - Avatar Walk-Over (visit)

    func sendAvatarVisit(to friendId: String, message: String, gift: String? = nil) {
        guard !serverURL.isEmpty else { return }
        let url = URL(string: "\(serverURL)/api/social/visit")!
        var req = authedRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        var body: [String: Any] = [
            "from_device_id": deviceId,
            "to_friend_id": friendId,
            "message": message
        ]
        if let gift { body["gift"] = gift }
        req.httpBody = try? JSONSerialization.data(withJSONObject: body)
        URLSession.shared.dataTask(with: req) { _, _, _ in }.resume()
    }

    func fetchVisitHistory() {
        guard !serverURL.isEmpty else { return }
        let url = URL(string: "\(serverURL)/api/social/visits?device_id=\(deviceId)")!
        URLSession.shared.dataTask(with: authedRequest(url: url)) { [weak self] data, _, _ in
            guard let data,
                  let decoder = Optional(JSONDecoder()),
                  let _ = {
                    decoder.dateDecodingStrategy = .iso8601
                    return true
                  }(),
                  let visits = try? decoder.decode([AvatarVisit].self, from: data) else { return }
            DispatchQueue.main.async {
                self?.visitHistory = visits
                self?.unreadVisits = visits.filter { !$0.isRead }.count
            }
        }.resume()
    }

    func markVisitRead(_ visitId: String) {
        visitHistory = visitHistory.map {
            var v = $0
            if v.id == visitId { v.isRead = true }
            return v
        }
        unreadVisits = visitHistory.filter { !$0.isRead }.count
    }

    // MARK: - Emotes

    func sendEmote(_ emote: AvatarEmote, to friendId: String?) async -> Bool {
        guard !serverURL.isEmpty else { return false }
        let targetId = friendId ?? friends.first?.id ?? ""
        guard !targetId.isEmpty else { return false }

        let url = URL(string: "\(serverURL)/api/social/emote")!
        var req = authedRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try? JSONSerialization.data(withJSONObject: [
            "from_device_id": deviceId,
            "to_friend_id": targetId,
            "emote": emote.rawValue
        ])

        do {
            let (_, response) = try await URLSession.shared.data(for: req)
            return (response as? HTTPURLResponse)?.statusCode == 200
        } catch {
            return false
        }
    }

    // MARK: - Avatar Raid

    func sendRaid(to friendId: String) {
        guard !serverURL.isEmpty else { return }
        let url = URL(string: "\(serverURL)/api/social/raid")!
        var req = authedRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try? JSONSerialization.data(withJSONObject: [
            "from_device_id": deviceId,
            "to_friend_id": friendId
        ])
        URLSession.shared.dataTask(with: req) { _, _, _ in }.resume()
        NotificationManager.shared.notifyAvatarRaid(from: "You")
    }

    func handleIncomingRaid(from friendName: String, emote: AvatarEmote) {
        incomingRaid = (fromName: friendName, emote: emote)
        HapticManager.playEvent(.avatarRaid)
        DispatchQueue.main.asyncAfter(deadline: .now() + 5) { [weak self] in
            self?.incomingRaid = nil
        }
    }

    // MARK: - Coding Challenges

    func sendChallenge(to friendId: String, type: String) {
        guard !serverURL.isEmpty else { return }
        let url = URL(string: "\(serverURL)/api/social/challenges")!
        var req = authedRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = try? JSONSerialization.data(withJSONObject: [
            "challenger_device_id": deviceId,
            "challenged_friend_id": friendId,
            "type": type
        ])
        URLSession.shared.dataTask(with: req) { [weak self] _, _, _ in
            Task { @MainActor in
                self?.refreshChallenges()
            }
        }.resume()
    }

    func refreshChallenges() {
        guard !serverURL.isEmpty else { return }
        let url = URL(string: "\(serverURL)/api/social/challenges?device_id=\(deviceId)")!
        URLSession.shared.dataTask(with: authedRequest(url: url)) { [weak self] data, _, _ in
            guard let data else { return }
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            guard let challenges = try? decoder.decode([CodingChallenge].self, from: data) else { return }
            DispatchQueue.main.async {
                self?.pendingChallenges = challenges.filter { !$0.isActive }
                self?.activeChallenges = challenges.filter { $0.isActive }
            }
        }.resume()
    }

    // MARK: - QR Code Generation

    private func generateQRCode(from string: String) -> UIImage? {
        guard let filter = CIFilter(name: "CIQRCodeGenerator") else { return nil }
        filter.setValue(Data(string.utf8), forKey: "inputMessage")
        filter.setValue("H", forKey: "inputCorrectionLevel")
        guard let ciImage = filter.outputImage else { return nil }
        let transform = CGAffineTransform(scaleX: 10, y: 10)
        let scaled = ciImage.transformed(by: transform)
        let context = CIContext()
        guard let cgImage = context.createCGImage(scaled, from: scaled.extent) else { return nil }
        return UIImage(cgImage: cgImage)
    }

    // MARK: - Cache

    private func cacheFriends(_ friends: [Friend]) {
        if let data = try? JSONEncoder().encode(friends) {
            UserDefaults.standard.set(data, forKey: "hookbot_friends_cache")
        }
    }

    private func loadCachedData() {
        if let data = UserDefaults.standard.data(forKey: "hookbot_friends_cache"),
           let cached = try? JSONDecoder().decode([Friend].self, from: data) {
            friends = cached
        }
    }

    // MARK: - Auth

    private func authedRequest(url: URL) -> URLRequest {
        var req = URLRequest(url: url)
        req.timeoutInterval = 10
        if !apiKey.isEmpty {
            req.setValue(apiKey, forHTTPHeaderField: "X-API-Key")
        }
        return req
    }
}
