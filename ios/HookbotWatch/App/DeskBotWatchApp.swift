import SwiftUI
import WatchConnectivity

@main
struct HookbotWatchApp: App {
    @StateObject private var engine = AvatarEngine()
    @StateObject private var connectivity = WatchConnectivityManager()

    var body: some Scene {
        WindowGroup {
            WatchMainView()
                .environmentObject(engine)
                .environmentObject(connectivity)
                .onAppear {
                    connectivity.engine = engine
                    connectivity.activate()
                }
        }
    }
}

// MARK: - Watch Connectivity (receives state from iPhone)

final class WatchConnectivityManager: NSObject, ObservableObject, WCSessionDelegate {
    weak var engine: AvatarEngine?
    private var session: WCSession?

    func activate() {
        guard WCSession.isSupported() else { return }
        session = WCSession.default
        session?.delegate = self
        session?.activate()
    }

    func session(_ session: WCSession, activationDidCompleteWith activationState: WCSessionActivationState, error: Error?) {
        print("[Watch] Session activated: \(activationState.rawValue)")
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
}
