import Foundation
import WatchKit

/// Manages haptic feedback on Apple Watch, with state-specific vibration patterns
/// that mirror the physical ESP32 buzzer behavior
final class HapticManager {

    static let shared = HapticManager()

    private var isPlaying = false

    // MARK: - State-Based Haptics

    /// Play a haptic pattern matching the avatar state transition
    func playStateHaptic(_ state: AvatarState) {
        switch state {
        case .idle:
            // Gentle tap — back to calm
            play(.click)

        case .thinking:
            // Quick double-tap — "working on it"
            playSequence([.click, .click], intervals: [0.12])

        case .waiting:
            // Rising urgency tap
            play(.directionUp)

        case .success:
            // Triumphant celebration — notification + success
            playSequence([.notification, .success], intervals: [0.3])

        case .taskcheck:
            // Crisp confirmation snap
            play(.success)

        case .error:
            // Aggressive triple buzz — SOMETHING IS WRONG
            playSequence([.failure, .retry, .failure], intervals: [0.15, 0.15])
        }
    }

    /// Escalation haptics for the waiting state (CEO getting impatient)
    /// Intensity increases with wait duration
    func playWaitingEscalation(secondsWaiting: Float) {
        if secondsWaiting < 6.0 {
            // Mild impatience — single tap
            play(.directionDown)
        } else if secondsWaiting < 10.0 {
            // Getting angry — heavier tap
            play(.retry)
        } else {
            // FULL TANTRUM — heavy buzz
            playSequence([.failure, .start, .failure], intervals: [0.1, 0.1])
        }
    }

    /// Notification haptic — new achievement, level up, etc.
    func playNotification() {
        play(.notification)
    }

    /// Level up celebration — extended pattern
    func playLevelUp() {
        playSequence(
            [.success, .notification, .success, .notification],
            intervals: [0.2, 0.3, 0.2]
        )
    }

    /// Achievement unlocked
    func playAchievement() {
        playSequence([.notification, .success], intervals: [0.25])
    }

    /// XP gained — subtle tick
    func playXPGain() {
        play(.click)
    }

    // MARK: - Playback Engine

    private func play(_ type: WKHapticType) {
        WKInterfaceDevice.current().play(type)
    }

    private func playSequence(_ types: [WKHapticType], intervals: [TimeInterval]) {
        guard !isPlaying else { return }
        isPlaying = true

        for (index, type) in types.enumerated() {
            let delay = intervals.prefix(index).reduce(0, +)
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
                WKInterfaceDevice.current().play(type)
                if index == types.count - 1 {
                    self?.isPlaying = false
                }
            }
        }
    }
}
