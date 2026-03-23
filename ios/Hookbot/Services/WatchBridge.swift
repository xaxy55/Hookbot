#if !targetEnvironment(macCatalyst)
import WatchConnectivity

// Forwards state changes from the iPhone to the Apple Watch

final class WatchBridge: NSObject, WCSessionDelegate {
    static let shared = WatchBridge()
    private var session: WCSession?

    func activate() {
        guard WCSession.isSupported() else { return }
        session = WCSession.default
        session?.delegate = self
        session?.activate()
    }

    func sendState(_ state: AvatarState, tool: String = "", detail: String = "") {
        guard let session, session.isReachable else { return }
        var message: [String: Any] = ["state": state.rawValue]
        if !tool.isEmpty { message["tool"] = tool }
        if !detail.isEmpty { message["detail"] = detail }
        session.sendMessage(message, replyHandler: nil) { error in
            print("[WatchBridge] Send failed: \(error)")
        }
    }

    // WCSessionDelegate
    func session(_ session: WCSession, activationDidCompleteWith activationState: WCSessionActivationState, error: Error?) {
        print("[WatchBridge] Activated: \(activationState.rawValue)")
    }

    func sessionDidBecomeInactive(_ session: WCSession) {}
    func sessionDidDeactivate(_ session: WCSession) {
        session.activate()
    }
}
#else
// Stub for Mac Catalyst where WatchConnectivity is unavailable
final class WatchBridge {
    static let shared = WatchBridge()
    func activate() {}
    func sendState(_ state: AvatarState, tool: String = "", detail: String = "") {}
}
#endif
