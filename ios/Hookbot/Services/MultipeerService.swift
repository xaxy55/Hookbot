import MultipeerConnectivity
import Combine
import UIKit

// Real-time P2P networking for shared AR, proximity, and Pong (Phase 13.5)

// MARK: - Message Types

enum HookbotPeerMessage: Codable {
    case avatarState(rawValue: String)
    case arAnchorMatrix([Float])          // row-major simd_float4x4
    case interaction(type: String)        // "highFive", "dance", "wave"
    case interactionAck(type: String)
    case pongPaddleY(normalized: Float)   // 0..1 normalized paddle Y
    case pongBallHandoff(velX: Float, velY: Float, ballY: Float, myScore: Int, theirScore: Int)
    case pongEvent(event: String)         // "start", "pause", "goal", "end"
    case niTokenData(base64: String)      // NIDiscoveryToken for UWB
    case proximityPing
}

// MARK: - Service

@MainActor
final class MultipeerService: NSObject, ObservableObject {

    static let shared = MultipeerService()

    @Published private(set) var isConnected = false
    @Published private(set) var peerName = ""
    @Published var latestMessage: HookbotPeerMessage?

    // Typed message streams
    var onPongPaddleY: ((Float) -> Void)?
    var onPongBallHandoff: ((Float, Float, Float, Int, Int) -> Void)?
    var onPongEvent: ((String) -> Void)?
    var onInteraction: ((String) -> Void)?
    var onARMatrix: (([Float]) -> Void)?

    private let serviceType = "hookbot-ar"
    private var peerID: MCPeerID
    private var session: MCSession!
    private var advertiser: MCNearbyServiceAdvertiser!
    private var browser: MCNearbyServiceBrowser!

    private override init() {
        let name = UserDefaults.standard.string(forKey: "hookbot_device_name")
            ?? UIDevice.current.name
        peerID = MCPeerID(displayName: name)
        super.init()
        setupSession()
    }

    private func setupSession() {
        session = MCSession(peer: peerID, securityIdentity: nil, encryptionPreference: .required)
        session.delegate = self
        advertiser = MCNearbyServiceAdvertiser(peer: peerID, discoveryInfo: nil, serviceType: serviceType)
        advertiser.delegate = self
        browser = MCNearbyServiceBrowser(peer: peerID, serviceType: serviceType)
        browser.delegate = self
    }

    // MARK: - Discovery

    func startDiscovery() {
        advertiser.startAdvertisingPeer()
        browser.startBrowsingForPeers()
    }

    func stopDiscovery() {
        advertiser.stopAdvertisingPeer()
        browser.stopBrowsingForPeers()
    }

    func disconnect() {
        session.disconnect()
        stopDiscovery()
        isConnected = false
        peerName = ""
    }

    // MARK: - Send

    func send(_ message: HookbotPeerMessage, reliable: Bool = true) {
        guard isConnected,
              let data = try? JSONEncoder().encode(message) else { return }
        try? session.send(
            data,
            toPeers: session.connectedPeers,
            with: reliable ? .reliable : .unreliable
        )
    }

    // MARK: - Handle incoming

    private func handle(_ message: HookbotPeerMessage) {
        latestMessage = message
        switch message {
        case .pongPaddleY(let y):
            onPongPaddleY?(y)
        case .pongBallHandoff(let vx, let vy, let by, let ms, let ts):
            onPongBallHandoff?(vx, vy, by, ms, ts)
        case .pongEvent(let event):
            onPongEvent?(event)
        case .interaction(let type):
            onInteraction?(type)
        case .arAnchorMatrix(let m):
            onARMatrix?(m)
        default:
            break
        }
    }
}

// MARK: - MCSessionDelegate

extension MultipeerService: MCSessionDelegate {

    nonisolated func session(_ session: MCSession, peer peerID: MCPeerID, didChange state: MCSessionState) {
        Task { @MainActor in
            switch state {
            case .connected:
                isConnected = true
                peerName = peerID.displayName
            case .notConnected:
                if session.connectedPeers.isEmpty {
                    isConnected = false
                    peerName = ""
                }
            default:
                break
            }
        }
    }

    nonisolated func session(_ session: MCSession, didReceive data: Data, fromPeer peerID: MCPeerID) {
        guard let message = try? JSONDecoder().decode(HookbotPeerMessage.self, from: data) else { return }
        Task { @MainActor in
            handle(message)
        }
    }

    nonisolated func session(_ session: MCSession, didReceive stream: InputStream, withName streamName: String, fromPeer peerID: MCPeerID) {}
    nonisolated func session(_ session: MCSession, didStartReceivingResourceWithName resourceName: String, fromPeer peerID: MCPeerID, with progress: Progress) {}
    nonisolated func session(_ session: MCSession, didFinishReceivingResourceWithName resourceName: String, fromPeer peerID: MCPeerID, at localURL: URL?, withError error: Error?) {}
}

// MARK: - MCNearbyServiceAdvertiserDelegate

extension MultipeerService: MCNearbyServiceAdvertiserDelegate {
    nonisolated func advertiser(_ advertiser: MCNearbyServiceAdvertiser, didReceiveInvitationFromPeer peerID: MCPeerID, withContext context: Data?, invitationHandler: @escaping (Bool, MCSession?) -> Void) {
        Task { @MainActor in
            invitationHandler(true, session)
        }
    }
}

// MARK: - MCNearbyServiceBrowserDelegate

extension MultipeerService: MCNearbyServiceBrowserDelegate {
    nonisolated func browser(_ browser: MCNearbyServiceBrowser, foundPeer peerID: MCPeerID, withDiscoveryInfo info: [String: String]?) {
        Task { @MainActor in
            browser.invitePeer(peerID, to: session, withContext: nil, timeout: 30)
        }
    }
    nonisolated func browser(_ browser: MCNearbyServiceBrowser, lostPeer peerID: MCPeerID) {}
}
