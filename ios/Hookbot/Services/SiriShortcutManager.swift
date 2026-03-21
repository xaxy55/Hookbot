import AppIntents
import SwiftUI

// Siri Shortcuts via AppIntents framework (Phase 13.1)
// "Hey Siri, how's my hookbot?" / "Start focus mode" / "Send my buddy a wave"

// MARK: - How's My Hookbot?

struct HookbotStatusIntent: AppIntent {
    static var title: LocalizedStringResource = "How's My Hookbot?"
    static var description = IntentDescription("Check your hookbot's current state and mood.")
    static var openAppWhenRun = false

    @Dependency
    var engine: AvatarEngine

    func perform() async throws -> some ReturnsValue & ProvidesDialog {
        let state = engine.currentState
        let dialog: String
        switch state {
        case .idle:      dialog = "Your hookbot is scheming in the background. All quiet."
        case .thinking:  dialog = "Your hookbot is plotting. Claude is busy thinking."
        case .waiting:   dialog = "Your hookbot is waiting and getting impatient."
        case .success:   dialog = "Success! Your hookbot just conquered a task."
        case .taskcheck: dialog = "Your hookbot approved a task. Looking good."
        case .error:     dialog = "Uh oh — your hookbot is furious. There's an error."
        }
        return .result(value: dialog, dialog: IntentDialog(stringLiteral: dialog))
    }
}

// MARK: - Start Focus Mode

struct StartHookbotFocusIntent: AppIntent {
    static var title: LocalizedStringResource = "Start Hookbot Focus Mode"
    static var description = IntentDescription("Put your hookbot in focus mode — DND on, distractions off.")
    static var openAppWhenRun = false

    @Dependency
    var engine: AvatarEngine

    func perform() async throws -> some ReturnsValue & ProvidesDialog {
        await MainActor.run {
            FocusModeManager.shared.enterFocusMode()
        }
        let dialog = "Focus mode activated. Your hookbot is in the zone."
        return .result(value: true, dialog: IntentDialog(stringLiteral: dialog))
    }
}

// MARK: - Send Buddy a Wave

struct SendBuddyWaveIntent: AppIntent {
    static var title: LocalizedStringResource = "Send My Buddy a Wave"
    static var description = IntentDescription("Send a quick wave reaction to your paired hookbot friend.")
    static var openAppWhenRun = false

    @Dependency
    var socialService: SocialService

    func perform() async throws -> some ReturnsValue & ProvidesDialog {
        let sent = await socialService.sendEmote(.wave, to: nil) // sends to first friend
        let dialog = sent
            ? "Wave sent! Your avatar is waving at your buddy."
            : "Couldn't reach your buddy right now. Are they online?"
        return .result(value: sent, dialog: IntentDialog(stringLiteral: dialog))
    }
}

// MARK: - Check Streak

struct CheckStreakIntent: AppIntent {
    static var title: LocalizedStringResource = "Check My Coding Streak"
    static var description = IntentDescription("Find out your current coding streak and XP.")
    static var openAppWhenRun = false

    @Dependency
    var engine: AvatarEngine

    func perform() async throws -> some ReturnsValue & ProvidesDialog {
        // Pull streak from UserDefaults (populated by AvatarEvolutionEngine)
        let streak = UserDefaults.standard.integer(forKey: "hookbot_streak")
        let xp = UserDefaults.standard.integer(forKey: "hookbot_xp")
        let dialog = streak > 0
            ? "You're on a \(streak)-day streak with \(xp) XP. Keep it up!"
            : "No active streak yet. Start coding to build your streak!"
        return .result(value: streak, dialog: IntentDialog(stringLiteral: dialog))
    }
}

// MARK: - Shortcut Phrases

struct HookbotShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: HookbotStatusIntent(),
            phrases: [
                "How's my hookbot?",
                "Check my hookbot",
                "What's my \(.applicationName) doing?"
            ],
            shortTitle: "Hookbot Status",
            systemImageName: "cpu"
        )
        AppShortcut(
            intent: StartHookbotFocusIntent(),
            phrases: [
                "Start \(.applicationName) focus mode",
                "Enable hookbot focus",
                "Hookbot, focus time"
            ],
            shortTitle: "Focus Mode",
            systemImageName: "moon.fill"
        )
        AppShortcut(
            intent: SendBuddyWaveIntent(),
            phrases: [
                "Send my buddy a wave",
                "Wave to my hookbot friend",
                "Hey \(.applicationName), wave to my buddy"
            ],
            shortTitle: "Wave to Buddy",
            systemImageName: "hand.wave.fill"
        )
        AppShortcut(
            intent: CheckStreakIntent(),
            phrases: [
                "What's my coding streak?",
                "Check my \(.applicationName) streak",
                "How many days is my streak?"
            ],
            shortTitle: "Coding Streak",
            systemImageName: "flame.fill"
        )
    }
}
