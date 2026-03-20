import SwiftUI

@main
struct HookbotApp: App {
    @StateObject private var engine = AvatarEngine()
    @StateObject private var network = NetworkService()
    @StateObject private var sound = SoundManager()
    @StateObject private var auth = AuthService()

    var body: some Scene {
        WindowGroup {
            Group {
                if auth.isAuthenticated {
                    MainView(auth: auth)
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
                // Auto-authenticate if we have saved credentials
                if auth.checkExistingAuth(config: engine.config) {
                    auth.isAuthenticated = true
                    network.start(engine: engine)
                }
                WatchBridge.shared.activate()
            }
        }
    }

    private func setupBindings() {
        engine.onPlaySound = { [weak sound] state in
            sound?.playStateSound(state)
            WatchBridge.shared.sendState(state)
        }
        engine.onPlayEscalationBeep = { [weak sound] secs in
            sound?.playEscalationBeep(secs: secs)
        }
        engine.onHaptic = { state in
            HapticManager.playStateHaptic(state)
        }
    }

    private func loadConfig() {
        if let data = UserDefaults.standard.data(forKey: "hookbot_config"),
           let config = try? JSONDecoder().decode(RuntimeConfig.self, from: data) {
            engine.config = config
        }
    }
}
