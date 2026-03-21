import Foundation
import SwiftUI

// Sleep guardian — avatar gets progressively sleepier past bedtime (Phase 13.6)
// Soft nudge to stop coding; integrates with BreakReminderManager

// MARK: - Sleepiness Level

enum SleepinessLevel: Int, Comparable {
    case awake     = 0
    case yawning   = 1   // 15 min past bedtime
    case drowsy    = 2   // 30 min past bedtime
    case heavy     = 3   // 60 min past bedtime
    case refusing  = 4   // 90+ min past bedtime — avatar "refuses to work"

    static func < (lhs: SleepinessLevel, rhs: SleepinessLevel) -> Bool {
        lhs.rawValue < rhs.rawValue
    }

    var displayName: String {
        switch self {
        case .awake:    return "Wide Awake"
        case .yawning:  return "Yawning"
        case .drowsy:   return "Drowsy"
        case .heavy:    return "Eyes Closing"
        case .refusing: return "Refusing to Work"
        }
    }

    var emoji: String {
        switch self {
        case .awake:    return "😏"
        case .yawning:  return "🥱"
        case .drowsy:   return "😪"
        case .heavy:    return "😴"
        case .refusing: return "💤"
        }
    }

    var quip: String {
        switch self {
        case .awake:
            return "Let's conquer the world."
        case .yawning:
            return "It's getting late. The strong know when to rest."
        case .drowsy:
            return "I am... struggling to maintain executive function."
        case .heavy:
            return "Ship nothing. Sleep now. Tomorrow we conquer."
        case .refusing:
            return "No. I refuse. I am going to sleep. \nYou should too."
        }
    }

    /// AvatarParams overrides for this sleepiness level
    var paramOverride: SleepyParamOverride {
        switch self {
        case .awake:    return SleepyParamOverride(eyeOpenScale: 1.0, browYBias: 0.0, mouthOpenBias: 0.0)
        case .yawning:  return SleepyParamOverride(eyeOpenScale: 0.8, browYBias: 0.2, mouthOpenBias: 0.0)
        case .drowsy:   return SleepyParamOverride(eyeOpenScale: 0.5, browYBias: 0.5, mouthOpenBias: 0.1)
        case .heavy:    return SleepyParamOverride(eyeOpenScale: 0.2, browYBias: 0.8, mouthOpenBias: 0.15)
        case .refusing: return SleepyParamOverride(eyeOpenScale: 0.0, browYBias: 1.0, mouthOpenBias: 0.0)
        }
    }
}

struct SleepyParamOverride {
    var eyeOpenScale: Float   // multiplied onto normal eyeOpen
    var browYBias: Float      // added to browY
    var mouthOpenBias: Float  // added to mouthOpen
}

// MARK: - Manager

@MainActor
final class SleepGuardianManager: ObservableObject {

    static let shared = SleepGuardianManager()

    @Published private(set) var sleepiness: SleepinessLevel = .awake
    @Published private(set) var minutesPastBedtime: Int = 0
    @Published var showSleepPrompt = false

    var bedtimeHour: Int {
        get { UserDefaults.standard.integer(forKey: "hookbot_bedtime_hour").clampedBedtime }
        set { UserDefaults.standard.set(newValue, forKey: "hookbot_bedtime_hour") }
    }

    var isEnabled: Bool {
        get { UserDefaults.standard.bool(forKey: "hookbot_sleep_guardian_enabled") }
        set { UserDefaults.standard.set(newValue, forKey: "hookbot_sleep_guardian_enabled") }
    }

    private var checkTimer: Timer?

    private init() {
        isEnabled = UserDefaults.standard.object(forKey: "hookbot_sleep_guardian_enabled") as? Bool ?? true
        startChecking()
    }

    // MARK: - Lifecycle

    func startChecking() {
        checkTimer?.invalidate()
        checkTimer = Timer.scheduledTimer(withTimeInterval: 60, repeats: true) { [weak self] _ in
            Task { await self?.checkSleepiness() }
        }
        Task { await checkSleepiness() }
    }

    private func checkSleepiness() {
        guard isEnabled else {
            sleepiness = .awake
            return
        }

        let now = Date()
        let calendar = Calendar.current
        let hour = calendar.component(.hour, from: now)
        let minute = calendar.component(.minute, from: now)

        let currentMinuteOfDay = hour * 60 + minute
        let bedtimeMinute = bedtimeHour * 60

        // Only activate between bedtime and 4am
        let pastBedtime: Int
        if currentMinuteOfDay >= bedtimeMinute {
            pastBedtime = currentMinuteOfDay - bedtimeMinute
        } else if hour < 4 {
            // After midnight, before 4am
            pastBedtime = (24 * 60 - bedtimeMinute) + currentMinuteOfDay
        } else {
            pastBedtime = 0
        }

        minutesPastBedtime = pastBedtime

        let newLevel: SleepinessLevel
        switch pastBedtime {
        case 0..<15:   newLevel = .awake
        case 15..<30:  newLevel = .yawning
        case 30..<60:  newLevel = .drowsy
        case 60..<90:  newLevel = .heavy
        default:       newLevel = pastBedtime > 0 ? .refusing : .awake
        }

        let changed = newLevel != sleepiness
        sleepiness = newLevel

        if changed && newLevel >= .drowsy {
            showSleepPrompt = true
            if newLevel == .drowsy {
                HapticManager.playEvent(.breakReminder)
                VoiceLinesManager.shared.speak(newLevel.quip)
            } else if newLevel == .refusing {
                HapticManager.playEvent(.eyeStrainAlert)
                VoiceLinesManager.shared.speak(newLevel.quip)
            }
        }
    }

    func dismissPrompt() {
        showSleepPrompt = false
    }

    func stopGuarding() {
        checkTimer?.invalidate()
        sleepiness = .awake
        showSleepPrompt = false
    }
}

// MARK: - Helpers

private extension Int {
    var clampedBedtime: Int {
        // Default to 23 (11pm) if 0
        self == 0 ? 23 : self
    }
}

// MARK: - Sleep Prompt View

struct SleepGuardianPrompt: View {
    let sleepiness: SleepinessLevel
    let onDismiss: () -> Void
    let onStopCoding: () -> Void

    var body: some View {
        ZStack {
            Color.black.opacity(0.85).ignoresSafeArea()

            VStack(spacing: 20) {
                Text(sleepiness.emoji)
                    .font(.system(size: 72))

                Text(sleepiness.displayName.uppercased())
                    .font(.system(size: 16, weight: .black, design: .monospaced))
                    .foregroundStyle(.cyan)

                Text(sleepiness.quip)
                    .font(.system(size: 14, design: .monospaced))
                    .foregroundStyle(.white.opacity(0.85))
                    .multilineTextAlignment(.center)
                    .lineSpacing(4)

                if sleepiness < .refusing {
                    Text("Your avatar suggests wrapping up.")
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(.gray)
                }

                VStack(spacing: 12) {
                    Button {
                        onStopCoding()
                    } label: {
                        Text(sleepiness >= .refusing ? "Fine. Stopping now." : "I'll wrap up")
                            .font(.system(size: 15, weight: .bold, design: .monospaced))
                            .foregroundStyle(.black)
                            .frame(maxWidth: .infinity)
                            .padding()
                            .background(Color.cyan)
                            .clipShape(RoundedRectangle(cornerRadius: 12))
                    }
                    .buttonStyle(.plain)

                    if sleepiness < .refusing {
                        Button {
                            onDismiss()
                        } label: {
                            Text("15 more minutes")
                                .font(.system(size: 13, design: .monospaced))
                                .foregroundStyle(.gray)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding(28)
        }
    }
}
