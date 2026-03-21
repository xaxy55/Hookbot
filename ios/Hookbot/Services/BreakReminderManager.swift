import Foundation
import UserNotifications

// Break reminders — avatar taps on screen when you code too long (Phase 13.6)
// Also handles the 20-20-20 eye strain rule and sleep guardian

// MARK: - Break Reminder Manager

@MainActor
final class BreakReminderManager: ObservableObject {

    static let shared = BreakReminderManager()

    @Published var shouldShowBreakPrompt = false
    @Published var shouldShowEyeStrainAlert = false
    @Published var shouldShowSleepWarning = false
    @Published var sessionMinutes: Int = 0

    // Settings
    var breakIntervalMinutes: Int {
        UserDefaults.standard.integer(forKey: "hookbot_break_interval").nonZero ?? 60
    }
    var eyeStrainIntervalMinutes: Int { 20 }
    var bedtime: DateComponents? {
        let hour = UserDefaults.standard.integer(forKey: "hookbot_bedtime_hour").nonZero ?? 23
        var dc = DateComponents()
        dc.hour = hour
        dc.minute = 0
        return dc
    }

    private var sessionTimer: Timer?
    private var eyeStrainTimer: Timer?
    private var sessionStartTime: Date?

    private init() {}

    // MARK: - Session control

    func startTracking() {
        sessionStartTime = Date()
        sessionMinutes = 0

        // Main break timer
        sessionTimer = Timer.scheduledTimer(withTimeInterval: 60, repeats: true) { [weak self] _ in
            Task { await self?.tick() }
        }

        // Eye strain 20-20-20 timer
        eyeStrainTimer = Timer.scheduledTimer(withTimeInterval: TimeInterval(eyeStrainIntervalMinutes * 60), repeats: true) { [weak self] _ in
            Task { await self?.triggerEyeStrainAlert() }
        }
    }

    func stopTracking() {
        sessionTimer?.invalidate()
        eyeStrainTimer?.invalidate()
        sessionTimer = nil
        eyeStrainTimer = nil
        sessionStartTime = nil
        sessionMinutes = 0
    }

    // MARK: - Tick (every minute)

    private func tick() {
        sessionMinutes += 1

        // Save daily minutes
        let daily = UserDefaults.standard.integer(forKey: "hookbot_daily_minutes") + 1
        UserDefaults.standard.set(daily, forKey: "hookbot_daily_minutes")

        // Check break threshold
        if sessionMinutes > 0 && sessionMinutes % breakIntervalMinutes == 0 {
            triggerBreakReminder()
        }

        // Check bedtime
        checkBedtime()
    }

    // MARK: - Break reminder

    private func triggerBreakReminder() {
        shouldShowBreakPrompt = true
        HapticManager.playEvent(.breakReminder)
        NotificationManager.shared.notifyBreakReminder()
    }

    func dismissBreakPrompt() {
        shouldShowBreakPrompt = false
    }

    func snoozeBreak(minutes: Int = 5) {
        shouldShowBreakPrompt = false
        DispatchQueue.main.asyncAfter(deadline: .now() + TimeInterval(minutes * 60)) { [weak self] in
            self?.triggerBreakReminder()
        }
    }

    // MARK: - Eye strain (20-20-20)

    private func triggerEyeStrainAlert() {
        shouldShowEyeStrainAlert = true
        HapticManager.playEvent(.eyeStrainAlert)
    }

    func dismissEyeStrainAlert() {
        shouldShowEyeStrainAlert = false
    }

    // MARK: - Sleep guardian

    private func checkBedtime() {
        guard let bedtime, let bedtimeHour = bedtime.hour else { return }
        let currentHour = Calendar.current.component(.hour, from: Date())
        let currentMinute = Calendar.current.component(.minute, from: Date())

        // If within 15 minutes of bedtime
        let minutesUntilBedtime = (bedtimeHour * 60) - (currentHour * 60 + currentMinute)
        if minutesUntilBedtime >= 0 && minutesUntilBedtime <= 15 {
            shouldShowSleepWarning = true
        } else if currentHour >= bedtimeHour {
            // Past bedtime — escalate
            shouldShowSleepWarning = true
        }
    }

    func dismissSleepWarning() {
        shouldShowSleepWarning = false
    }
}

// MARK: - Helpers

private extension Int {
    var nonZero: Int? { self == 0 ? nil : self }
}

