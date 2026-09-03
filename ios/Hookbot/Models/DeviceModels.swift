import Foundation

struct DeviceWithStatus: Codable, Identifiable {
    let id: String
    let name: String
    let hostname: String?
    let ipAddress: String?
    let purpose: String?
    let connectionMode: String?
    let firmwareVersion: String?
    let latestStatus: DeviceStatus?

    enum CodingKeys: String, CodingKey {
        case id, name, hostname, purpose
        case ipAddress = "ip_address"
        case connectionMode = "connection_mode"
        case firmwareVersion = "firmware_version"
        case latestStatus = "latest_status"
    }
}

struct DeviceStatus: Codable {
    let state: String?
    let tool: String?
    let detail: String?
    let uptime: Int?
    let freeHeap: Int?

    enum CodingKeys: String, CodingKey {
        case state, tool, detail, uptime
        case freeHeap = "free_heap"
    }
}

struct DeviceConfig: Codable {
    var name: String?
    var screensaverMins: Int?
    var soundEnabled: Bool?
    var doNotDisturb: Bool?
    var ledColor: String?
    var bootAnim: Int?

    enum CodingKeys: String, CodingKey {
        case name
        case screensaverMins = "screensaver_mins"
        case soundEnabled = "sound_enabled"
        case doNotDisturb = "do_not_disturb"
        case ledColor = "led_color"
        case bootAnim = "boot_anim"
    }
}

struct StatusHistoryEntry: Codable, Identifiable {
    let id: Int?
    let state: String
    let tool: String?
    let detail: String?
    let createdAt: String?

    enum CodingKeys: String, CodingKey {
        case id, state, tool, detail
        case createdAt = "created_at"
    }
}

/// One servo channel as the device reports it on GET /servos.
struct ServoChannel: Codable {
    let pin: Int
    let min: Int
    let max: Int
    let rest: Int
    var current: Int
    let label: String
    let enabled: Bool
    /// Whether the firmware's PWM attach actually succeeded. An enabled
    /// channel that failed to attach will never move, however healthy the rest
    /// of the record looks. Optional: older firmware does not report it.
    let attached: Bool?
}

struct ServoResponse: Codable {
    let channels: [ServoChannel]
}
