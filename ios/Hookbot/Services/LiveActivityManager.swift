#if !targetEnvironment(macCatalyst)
import ActivityKit
#endif
import Foundation

// Live Activity / Dynamic Island — your avatar lives on the home screen (Phase 13.1)
// Shows real-time state: coding, idle, in meeting, error
// HookbotActivityAttributes is defined in Shared/HookbotActivityAttributes.swift

// MARK: - Manager

@MainActor
final class LiveActivityManager: ObservableObject {

    static let shared = LiveActivityManager()

    @Published private(set) var isRunning = false

    #if !targetEnvironment(macCatalyst)
    private var currentActivity: Activity<HookbotActivityAttributes>?
    #endif

    private init() {}

    // MARK: - Start

    func startActivity(deviceName: String, state: AvatarState, xp: Int, streak: Int) {
        #if !targetEnvironment(macCatalyst)
        guard ActivityAuthorizationInfo().areActivitiesEnabled else { return }

        // End any existing activity first
        stopActivity()

        let attributes = HookbotActivityAttributes(deviceName: deviceName)
        let contentState = HookbotActivityAttributes.ContentState(
            state: state.rawValue,
            displayName: state.displayName,
            xp: xp,
            streak: streak,
            toolName: ""
        )

        do {
            currentActivity = try Activity<HookbotActivityAttributes>.request(
                attributes: attributes,
                content: .init(state: contentState, staleDate: nil),
                pushType: nil
            )
            isRunning = true
        } catch {
            print("[LiveActivity] Failed to start: \(error)")
        }
        #endif
    }

    // MARK: - Update

    func updateActivity(state: AvatarState, toolName: String = "", xp: Int = 0, streak: Int = 0) {
        #if !targetEnvironment(macCatalyst)
        guard let activity = currentActivity else {
            // Auto-start if we have stored config
            let deviceName = UserDefaults.standard.string(forKey: "hookbot_device_name") ?? "hookbot"
            startActivity(deviceName: deviceName, state: state, xp: xp, streak: streak)
            return
        }

        let contentState = HookbotActivityAttributes.ContentState(
            state: state.rawValue,
            displayName: state.displayName,
            xp: xp,
            streak: streak,
            toolName: toolName
        )

        Task {
            await activity.update(
                ActivityContent(state: contentState, staleDate: Date().addingTimeInterval(60))
            )
        }
        #endif
    }

    // MARK: - Stop

    func stopActivity() {
        #if !targetEnvironment(macCatalyst)
        guard let activity = currentActivity else { return }
        Task {
            let finalState = HookbotActivityAttributes.ContentState(
                state: AvatarState.idle.rawValue,
                displayName: AvatarState.idle.displayName,
                xp: 0,
                streak: 0,
                toolName: ""
            )
            await activity.end(
                ActivityContent(state: finalState, staleDate: nil),
                dismissalPolicy: .after(Date().addingTimeInterval(10))
            )
            await MainActor.run {
                self.isRunning = false
                self.currentActivity = nil
            }
        }
        #endif
    }

    // MARK: - Dynamic Island color

    static func glowColor(for state: AvatarState) -> (r: Double, g: Double, b: Double) {
        switch state {
        case .idle:      return (0, 0, 0.7)
        case .thinking:  return (0.5, 0, 0.8)
        case .waiting:   return (0.9, 0.6, 0)
        case .success:   return (0, 0.8, 0.3)
        case .taskcheck: return (0, 0.6, 0.6)
        case .error:     return (1.0, 0, 0)
        }
    }
}
