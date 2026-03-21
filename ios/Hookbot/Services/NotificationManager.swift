import UserNotifications
import UIKit

// Smart notification grouping and push delivery (Phase 13.1)
// Groups low-priority hookbot alerts; surfaces critical ones instantly.

final class NotificationManager: NSObject, UNUserNotificationCenterDelegate {

    static let shared = NotificationManager()

    // Notification categories / thread IDs
    private enum Category: String {
        case hookbotState   = "hookbot.state"
        case social         = "hookbot.social"
        case achievement    = "hookbot.achievement"
        case breakReminder  = "hookbot.break"
        case pomodoro       = "hookbot.pomodoro"
        case daily          = "hookbot.daily"
    }

    override private init() {
        super.init()
        UNUserNotificationCenter.current().delegate = self
    }

    // MARK: - Permission

    func requestPermission(completion: ((Bool) -> Void)? = nil) {
        UNUserNotificationCenter.current().requestAuthorization(
            options: [.alert, .sound, .badge, .criticalAlert]
        ) { granted, _ in
            DispatchQueue.main.async { completion?(granted) }
        }
    }

    func registerCategories() {
        let dismissAction = UNNotificationAction(
            identifier: "DISMISS",
            title: "Dismiss",
            options: []
        )
        let viewAction = UNNotificationAction(
            identifier: "VIEW",
            title: "View",
            options: [.foreground]
        )
        let snoozeAction = UNNotificationAction(
            identifier: "SNOOZE",
            title: "5 min",
            options: []
        )

        let stateCategory = UNNotificationCategory(
            identifier: Category.hookbotState.rawValue,
            actions: [viewAction, dismissAction],
            intentIdentifiers: [],
            options: [.customDismissAction]
        )
        let socialCategory = UNNotificationCategory(
            identifier: Category.social.rawValue,
            actions: [viewAction, dismissAction],
            intentIdentifiers: [],
            options: []
        )
        let breakCategory = UNNotificationCategory(
            identifier: Category.breakReminder.rawValue,
            actions: [snoozeAction, dismissAction],
            intentIdentifiers: [],
            options: []
        )
        let pomodoroCategory = UNNotificationCategory(
            identifier: Category.pomodoro.rawValue,
            actions: [viewAction, dismissAction],
            intentIdentifiers: [],
            options: []
        )

        UNUserNotificationCenter.current().setNotificationCategories([
            stateCategory, socialCategory, breakCategory, pomodoroCategory
        ])
    }

    // MARK: - Schedule helpers

    /// Send an avatar state change notification (batched for low-priority states).
    func notifyStateChange(_ state: AvatarState) {
        let isCritical = state == .error
        let title: String
        let body: String

        switch state {
        case .idle:      title = "Hookbot"; body = "Your bot is scheming..."
        case .thinking:  title = "Hookbot is plotting"; body = "Claude is busy thinking"
        case .waiting:   title = "Hookbot is waiting"; body = "Something needs your attention"
        case .success:   title = "Success!"; body = "Task completed"
        case .taskcheck: title = "Task checked off"; body = "Your bot approved"
        case .error:     title = "Build failed!"; body = "Your hookbot is furious"
        }

        schedule(
            id: "state-\(state.rawValue)",
            title: title,
            body: body,
            category: Category.hookbotState.rawValue,
            threadId: "hookbot.states",
            interruptionLevel: isCritical ? .critical : .passive,
            delay: isCritical ? 0 : 2 // batch low-priority with a small delay
        )
    }

    /// Friend avatar visit notification.
    func notifyFriendVisit(from friendName: String, message: String) {
        schedule(
            id: "visit-\(Date().timeIntervalSince1970)",
            title: "\(friendName)'s avatar is visiting!",
            body: message.isEmpty ? "Your buddy's avatar walked over to say hi" : message,
            category: Category.social.rawValue,
            threadId: "hookbot.social",
            interruptionLevel: .active
        )
    }

    /// Avatar raid — expands into a full-screen animation when tapped.
    func notifyAvatarRaid(from friendName: String) {
        schedule(
            id: "raid-\(Date().timeIntervalSince1970)",
            title: "\(friendName) sent a RAID!",
            body: "Tap to see the chaos unfold",
            category: Category.social.rawValue,
            threadId: "hookbot.social",
            interruptionLevel: .timeSensitive,
            userInfo: ["action": "raid", "from": friendName]
        )
    }

    /// Daily reward reminder.
    func scheduleDailyReward(at hour: Int = 9, minute: Int = 0) {
        var components = DateComponents()
        components.hour = hour
        components.minute = minute

        let trigger = UNCalendarNotificationTrigger(dateMatching: components, repeats: true)

        let content = UNMutableNotificationContent()
        content.title = "Daily reward waiting!"
        content.body = "Your hookbot has a gift for you"
        content.sound = .default
        content.categoryIdentifier = Category.daily.rawValue
        content.threadIdentifier = "hookbot.daily"
        if #available(iOS 15.0, *) {
            content.interruptionLevel = .passive
        }

        let request = UNNotificationRequest(
            identifier: "daily-reward",
            content: content,
            trigger: trigger
        )
        UNUserNotificationCenter.current().add(request)
    }

    /// Break reminder notification.
    func notifyBreakReminder() {
        schedule(
            id: "break-\(Date().timeIntervalSince1970)",
            title: "Time for a break",
            body: "You've been coding for a while. Your avatar is stretching",
            category: Category.breakReminder.rawValue,
            threadId: "hookbot.break",
            interruptionLevel: .passive
        )
    }

    /// Pomodoro timer done.
    func notifyPomodoroComplete(isWork: Bool) {
        schedule(
            id: "pomodoro-\(Date().timeIntervalSince1970)",
            title: isWork ? "Pomodoro done!" : "Break's over!",
            body: isWork ? "Great sprint! Time for a break" : "Back to it — your avatar is ready",
            category: Category.pomodoro.rawValue,
            threadId: "hookbot.pomodoro",
            interruptionLevel: .timeSensitive
        )
    }

    // MARK: - UNUserNotificationCenterDelegate

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        // Show banner + play sound even when app is in foreground for critical alerts
        let category = notification.request.content.categoryIdentifier
        if category == Category.hookbotState.rawValue || category == Category.pomodoro.rawValue {
            completionHandler([.banner, .sound])
        } else {
            completionHandler([.list])
        }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        let userInfo = response.notification.request.content.userInfo
        if let action = userInfo["action"] as? String, action == "raid" {
            NotificationCenter.default.post(name: .hookbotAvatarRaidReceived, object: userInfo["from"])
        }
        if response.actionIdentifier == "SNOOZE" {
            // Re-schedule break reminder in 5 minutes
            DispatchQueue.main.asyncAfter(deadline: .now() + 300) { [weak self] in
                self?.notifyBreakReminder()
            }
        }
        completionHandler()
    }

    // MARK: - Scheduling

    func schedule(
        id: String,
        title: String,
        body: String,
        category: String,
        threadId: String,
        interruptionLevel: UNNotificationInterruptionLevel = .active,
        delay: TimeInterval = 0,
        userInfo: [AnyHashable: Any] = [:]
    ) {
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        content.categoryIdentifier = category
        content.threadIdentifier = threadId
        content.userInfo = userInfo
        if #available(iOS 15.0, *) {
            content.interruptionLevel = interruptionLevel
        }

        let trigger: UNNotificationTrigger? = delay > 0
            ? UNTimeIntervalNotificationTrigger(timeInterval: delay, repeats: false)
            : nil

        let request = UNNotificationRequest(identifier: id, content: content, trigger: trigger)
        UNUserNotificationCenter.current().add(request)
    }
}

// MARK: - Notification Names

extension Notification.Name {
    static let hookbotAvatarRaidReceived = Notification.Name("hookbot.avatarRaidReceived")
}

