import SwiftUI

@main
struct HookbotApp: App {
    @StateObject private var engine = AvatarEngine()
    @StateObject private var network = NetworkService()
    @StateObject private var sound = SoundManager()

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
