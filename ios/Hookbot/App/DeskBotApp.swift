import SwiftUI
<<<<<<< Updated upstream
import UserNotifications
=======
>>>>>>> Stashed changes

@main
struct HookbotApp: App {
    @StateObject private var engine = AvatarEngine()
    @StateObject private var network = NetworkService()
    @StateObject private var sound = SoundManager()
<<<<<<< Updated upstream
    @StateObject private var auth = AuthService()

    var body: some Scene {
        WindowGroup {
            Group {
                if auth.isAuthenticated {
                    HookbotTabView(auth: auth)
                        .environmentObject(engine)
                        .environmentObject(network)
                        .environmentObject(sound)
                } else {
                    LoginView(auth: auth)
                        .environmentObject(engine)
                        .environmentObject(network)
                }
            }
            .onAppear {
                loadConfig()
                setupBindings()
                setupManagers()
                if auth.checkExistingAuth(config: engine.config) {
                    auth.isAuthenticated = true
                    network.start(engine: engine)
                }
                WatchBridge.shared.activate()
            }
=======

    var body: some Scene {
        WindowGroup {
            MainView()
                .environmentObject(engine)
                .environmentObject(network)
                .environmentObject(sound)
                .onAppear {
                    setupBindings()
                    network.startServer(engine: engine)
                    loadConfig()
                    WatchBridge.shared.activate()
                }
>>>>>>> Stashed changes
        }
    }

    private func setupBindings() {
        engine.onPlaySound = { [weak sound] state in
            sound?.playStateSound(state)
            WatchBridge.shared.sendState(state)
<<<<<<< Updated upstream

            // Voice lines (13.3)
            VoiceLinesManager.shared.playQuip(for: state)

            // Live Activity update (13.1)
            let xp = UserDefaults.standard.integer(forKey: "hookbot_xp")
            let streak = UserDefaults.standard.integer(forKey: "hookbot_streak")
            LiveActivityManager.shared.updateActivity(state: state, xp: xp, streak: streak)

            // Evolution tracking (13.5)
            switch state {
            case .success, .taskcheck:
                AvatarEvolutionEngine.shared.recordSuccess()
            case .error:
                AvatarEvolutionEngine.shared.recordError()
            default: break
            }
=======
>>>>>>> Stashed changes
        }
        engine.onPlayEscalationBeep = { [weak sound] secs in
            sound?.playEscalationBeep(secs: secs)
        }
        engine.onHaptic = { state in
            HapticManager.playStateHaptic(state)
        }
<<<<<<< Updated upstream

        // Observe set-state requests from Siri Shortcuts / Focus mode
        NotificationCenter.default.addObserver(
            forName: .hookbotSetAvatarState,
            object: nil,
            queue: .main
        ) { notification in
            if let state = notification.object as? AvatarState {
                engine.setState(state)
            }
        }
    }

    private func setupManagers() {
        // Notifications (13.1)
        NotificationManager.shared.requestPermission()
        NotificationManager.shared.registerCategories()
        NotificationManager.shared.scheduleDailyReward()

        // Social (13.2)
        SocialService.shared.configure(engine: engine)
        SocialService.shared.start()

        // Seasonal events (13.5)
        SeasonalEventsManager.shared.refresh()

        // Siri Shortcuts (13.1)
        HookbotShortcuts.updateAppShortcutParameters()
=======
>>>>>>> Stashed changes
    }

    private func loadConfig() {
        if let data = UserDefaults.standard.data(forKey: "hookbot_config"),
<<<<<<< Updated upstream
           var config = try? JSONDecoder().decode(RuntimeConfig.self, from: data) {
            // Normalize serverURL to ensure it has an https scheme
            if !config.serverURL.isEmpty,
               !config.serverURL.hasPrefix("https://"),
               !config.serverURL.hasPrefix("http://") {
                config.serverURL = "https://\(config.serverURL)"
            }
=======
           let config = try? JSONDecoder().decode(RuntimeConfig.self, from: data) {
>>>>>>> Stashed changes
            engine.config = config
        }
    }
}
