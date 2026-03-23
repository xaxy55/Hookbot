import SwiftUI
import WatchConnectivity

@main
struct HookbotWatchApp: App {
    @StateObject private var engine = AvatarEngine()
    @StateObject private var connectivity = WatchConnectivityManager()
    @StateObject private var network = WatchNetworkService()

    var body: some Scene {
        WindowGroup {
            WatchTabView()
                .environmentObject(engine)
                .environmentObject(connectivity)
                .environmentObject(network)
                .onAppear {
                    connectivity.engine = engine
                    connectivity.activate()

                    network.engine = engine
                    if !network.serverURL.isEmpty {
                        network.startPolling()
                    }

                    // Wire up haptic feedback on state changes
                    engine.onHaptic = { state in
                        HapticManager.shared.playStateHaptic(state)
                    }

                    // Wire up escalation haptics for waiting state
                    engine.onPlayEscalationBeep = { secondsWaiting in
                        HapticManager.shared.playWaitingEscalation(secondsWaiting: secondsWaiting)
                    }
                }
        }
    }
}

// MARK: - Watch Connectivity (receives state from iPhone)

final class WatchConnectivityManager: NSObject, ObservableObject, WCSessionDelegate {
    weak var engine: AvatarEngine?
    @Published var isReachable = false
    private var session: WCSession?

    func activate() {
        guard WCSession.isSupported() else { return }
        session = WCSession.default
        session?.delegate = self
        session?.activate()
    }

    func session(_ session: WCSession, activationDidCompleteWith activationState: WCSessionActivationState, error: Error?) {
        DispatchQueue.main.async {
            self.isReachable = session.isReachable
        }
    }

    func sessionReachabilityDidChange(_ session: WCSession) {
        DispatchQueue.main.async {
            self.isReachable = session.isReachable
        }
    }

    func session(_ session: WCSession, didReceiveMessage message: [String: Any]) {
        guard let stateStr = message["state"] as? String,
              let state = AvatarState(rawValue: stateStr) else { return }

        DispatchQueue.main.async { [weak self] in
            if let toolName = message["tool"] as? String {
                self?.engine?.setTool(name: toolName, detail: message["detail"] as? String ?? "")
            }
            self?.engine?.setState(state)
        }
    }

    func session(_ session: WCSession, didReceiveApplicationContext applicationContext: [String: Any]) {
        // Receive server config from iPhone app
        DispatchQueue.main.async {
            if let serverURL = applicationContext["serverURL"] as? String {
                UserDefaults.standard.set(serverURL, forKey: "hookbot_watch_server")
            }
            if let deviceId = applicationContext["deviceId"] as? String {
                UserDefaults.standard.set(deviceId, forKey: "hookbot_watch_device")
            }
        }
    }
}
