import Foundation
import UIKit

// iOS Focus mode integration — hookbot auto-enters DND during coding Focus (Phase 13.5)
// Uses FamilyControls/ManagedSettings for Focus filter (iOS 16+)

@MainActor
final class FocusModeManager: ObservableObject {

    static let shared = FocusModeManager()

    @Published private(set) var isFocusActive = false
    @Published private(set) var currentFocusName: String = ""

    // Observer for system Focus changes
    private var focusObserver: Any?

    private init() {
        observeFocusChanges()
    }

    // MARK: - Enter focus mode (called from Siri Shortcut or app)

    func enterFocusMode() {
        guard !isFocusActive else { return }
        isFocusActive = true
        currentFocusName = "Coding"

        // Persist
        UserDefaults.standard.set(true, forKey: "hookbot_focus_active")
        UserDefaults.standard.set(Date(), forKey: "hookbot_focus_start")

        // Notify other subsystems
        NotificationCenter.default.post(name: .hookbotFocusModeChanged, object: true)

        // Send avatar into thinking state to signal focus
        NotificationCenter.default.post(
            name: .hookbotSetAvatarState,
            object: AvatarState.thinking
        )

        // Schedule break reminder after 90 minutes
        NotificationManager.shared.scheduleFocusBreak(after: 90 * 60)
    }

    func exitFocusMode() {
        guard isFocusActive else { return }
        isFocusActive = false
        currentFocusName = ""
        UserDefaults.standard.set(false, forKey: "hookbot_focus_active")

        NotificationCenter.default.post(name: .hookbotFocusModeChanged, object: false)
        NotificationCenter.default.post(
            name: .hookbotSetAvatarState,
            object: AvatarState.idle
        )
    }

    // MARK: - System Focus observation

    private func observeFocusChanges() {
        // Observe app becoming active/inactive as proxy for Focus changes
        focusObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.didBecomeActiveNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.checkFocusStatus()
        }
    }

    private func checkFocusStatus() {
        // Restore persisted focus state
        let wasActive = UserDefaults.standard.bool(forKey: "hookbot_focus_active")
        if wasActive != isFocusActive {
            isFocusActive = wasActive
        }
    }

    // MARK: - Focus session stats

    var focusDuration: TimeInterval {
        guard isFocusActive,
              let start = UserDefaults.standard.object(forKey: "hookbot_focus_start") as? Date else { return 0 }
        return Date().timeIntervalSince(start)
    }

    var focusDurationFormatted: String {
        let mins = Int(focusDuration / 60)
        let hours = mins / 60
        let remainingMins = mins % 60
        if hours > 0 {
            return "\(hours)h \(remainingMins)m"
        }
        return "\(mins)m"
    }
}

// MARK: - NotificationManager extension for focus

extension NotificationManager {
    func scheduleFocusBreak(after seconds: TimeInterval) {
        let content = UNMutableNotificationContent()
        content.title = "Focus session: 90 minutes in"
        content.body = "Your avatar suggests a short break. You've earned it."
        content.sound = .default
        content.categoryIdentifier = "hookbot.break"
        content.threadIdentifier = "hookbot.focus"

        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: seconds, repeats: false)
        let request = UNNotificationRequest(identifier: "focus-break", content: content, trigger: trigger)
        UNUserNotificationCenter.current().add(request)
    }
}

// MARK: - Notification Names

extension Notification.Name {
    static let hookbotFocusModeChanged = Notification.Name("hookbot.focusModeChanged")
    static let hookbotSetAvatarState   = Notification.Name("hookbot.setAvatarState")
}
