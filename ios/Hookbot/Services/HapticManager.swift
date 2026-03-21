#if canImport(UIKit)
import UIKit
#endif

// Haptic feedback — state haptics + rich custom event patterns (Phase 13.1)

// MARK: - Haptic Events

enum HapticEvent {
    case taskComplete
    case buildFailure
    case flowState        // heartbeat pulse during long coding sessions
    case friendVisit      // soft welcome when buddy avatar arrives
    case levelUp          // ascending celebration burst
    case dailyReward      // warm reward tap
    case breakReminder    // gentle repeated nudge
    case avatarRaid       // rapid celebratory taps from incoming raid
    case pomodoroStart
    case pomodoroEnd
    case eyeStrainAlert   // soft reminder to look away
}

// MARK: - Manager

enum HapticManager {

    // MARK: State haptics (per avatar state change)

    static func playStateHaptic(_ state: AvatarState) {
        #if targetEnvironment(macCatalyst)
        // No haptic engine on Mac
        #else
        switch state {
        case .idle:
            let generator = UIImpactFeedbackGenerator(style: .light)
            generator.impactOccurred(intensity: 0.4)

        case .thinking:
            let generator = UIImpactFeedbackGenerator(style: .medium)
            generator.impactOccurred()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                generator.impactOccurred(intensity: 0.6)
            }

        case .waiting:
            let generator = UIImpactFeedbackGenerator(style: .heavy)
            generator.impactOccurred()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) {
                generator.impactOccurred(intensity: 0.9)
            }

        case .success:
            playEvent(.taskComplete)

        case .taskcheck:
            let generator = UIImpactFeedbackGenerator(style: .medium)
            generator.impactOccurred(intensity: 0.7)

        case .error:
            playEvent(.buildFailure)
        }
        #endif
    }

    // MARK: Event haptics (Phase 13.1 — custom patterns per event)

    static func playEvent(_ event: HapticEvent) {
        #if targetEnvironment(macCatalyst)
        #else
        switch event {
        case .taskComplete:
            // Gentle success notification
            UINotificationFeedbackGenerator().notificationOccurred(.success)

        case .buildFailure:
            // Heavy thud × 2 then error notification
            let heavy = UIImpactFeedbackGenerator(style: .heavy)
            heavy.prepare()
            heavy.impactOccurred()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { heavy.impactOccurred() }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                UINotificationFeedbackGenerator().notificationOccurred(.error)
            }

        case .flowState:
            // Heartbeat: strong–soft, pause, strong–soft
            let g = UIImpactFeedbackGenerator(style: .heavy)
            g.impactOccurred(intensity: 0.9)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { g.impactOccurred(intensity: 0.4) }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.70) { g.impactOccurred(intensity: 0.9) }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.85) { g.impactOccurred(intensity: 0.4) }

        case .friendVisit:
            // Soft ascending triple tap
            let g = UIImpactFeedbackGenerator(style: .soft)
            g.prepare()
            for i in 0..<3 {
                DispatchQueue.main.asyncAfter(deadline: .now() + Double(i) * 0.15) {
                    g.impactOccurred(intensity: 0.4 + Double(i) * 0.2)
                }
            }

        case .levelUp:
            // Ascending burst: light → medium → heavy → success
            let styles: [UIImpactFeedbackGenerator.FeedbackStyle] = [.light, .medium, .heavy, .rigid]
            for (i, style) in styles.enumerated() {
                DispatchQueue.main.asyncAfter(deadline: .now() + Double(i) * 0.08) {
                    UIImpactFeedbackGenerator(style: style).impactOccurred()
                }
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                UINotificationFeedbackGenerator().notificationOccurred(.success)
            }

        case .dailyReward:
            // Warm tap + delayed success
            UIImpactFeedbackGenerator(style: .medium).impactOccurred(intensity: 0.6)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
                UINotificationFeedbackGenerator().notificationOccurred(.success)
            }

        case .breakReminder:
            // Soft nudge × 3 at 400ms intervals
            let g = UIImpactFeedbackGenerator(style: .soft)
            g.prepare()
            for i in 0..<3 {
                DispatchQueue.main.asyncAfter(deadline: .now() + Double(i) * 0.4) {
                    g.impactOccurred(intensity: 0.4)
                }
            }

        case .avatarRaid:
            // Rapid celebratory alternating taps
            let g = UIImpactFeedbackGenerator(style: .rigid)
            g.prepare()
            for i in 0..<6 {
                DispatchQueue.main.asyncAfter(deadline: .now() + Double(i) * 0.06) {
                    g.impactOccurred(intensity: i % 2 == 0 ? 0.9 : 0.5)
                }
            }

        case .pomodoroStart:
            // Sharp double tap — "start the clock"
            let g = UIImpactFeedbackGenerator(style: .rigid)
            g.impactOccurred()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { g.impactOccurred() }

        case .pomodoroEnd:
            // Success ring
            UINotificationFeedbackGenerator().notificationOccurred(.success)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                UIImpactFeedbackGenerator(style: .medium).impactOccurred(intensity: 0.8)
            }

        case .eyeStrainAlert:
            // Very soft single tap
            UIImpactFeedbackGenerator(style: .soft).impactOccurred(intensity: 0.3)
        }
        #endif
    }
}
