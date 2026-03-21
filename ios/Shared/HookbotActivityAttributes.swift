#if canImport(ActivityKit)
import ActivityKit
import Foundation

/// Shared Live Activity attributes — used by both the main app and the iOS widget extension.
struct HookbotActivityAttributes: ActivityAttributes {
    struct ContentState: Codable, Hashable {
        var state: String        // matches AvatarState.rawValue
        var displayName: String
        var xp: Int
        var streak: Int
        var toolName: String
    }

    var deviceName: String
}
#endif
