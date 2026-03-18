import Foundation

// MARK: - Avatar State (matches ESP firmware exactly)

enum AvatarState: String, Codable, CaseIterable {
    case idle
    case thinking
    case waiting
    case success
    case taskcheck
    case error

    var displayName: String {
        switch self {
        case .idle:      return "Scheming"
        case .thinking:  return "Plotting"
        case .waiting:   return "Displeased"
        case .success:   return "Conquered"
        case .taskcheck: return "Approved"
        case .error:     return "DESTROY"
        }
    }
}

// MARK: - Avatar Parameters (matches firmware AvatarParams)

struct AvatarParams {
    var eyeX: Float = 0        // -1..1 horizontal gaze
    var eyeY: Float = 0        // -1..1 vertical gaze
    var eyeOpen: Float = 0.9   // 0=closed, 1.2=wide rage
    var mouthCurve: Float = 0  // -1=frown, 1=grin
    var mouthOpen: Float = 0   // 0=closed, 1=wide
    var bounce: Float = 0      // vertical offset
    var shake: Float = 0       // horizontal offset
    var browAngle: Float = 0   // -1=angry V, 1=raised
    var browY: Float = 0       // vertical brow offset
}

// MARK: - Tool Info

struct ToolInfo {
    var name: String = ""
    var detail: String = ""
}

// MARK: - Task Item

struct TaskItem: Identifiable {
    let id = UUID()
    var label: String
    var status: Int // 0=pending, 1=active, 2=done, 3=failed
}

// MARK: - Accessories Configuration

struct AccessoriesConfig: Codable {
    var topHat: Bool = true
    var cigar: Bool = false
    var glasses: Bool = false
    var monocle: Bool = false
    var bowtie: Bool = true
    var crown: Bool = false
    var horns: Bool = false
    var halo: Bool = false
}

// MARK: - Runtime Configuration

struct RuntimeConfig: Codable {
    var soundEnabled: Bool = true
    var soundVolume: Int = 50
    var accessories: AccessoriesConfig = AccessoriesConfig()
    var serverURL: String = ""
    var deviceName: String = "hookbot-ios"
}

// MARK: - LED Color (for glow effect)

struct LEDColor {
    var r: Float
    var g: Float
    var b: Float
    var a: Float = 1.0
}
