import CoreBluetooth
import NearbyInteraction
import Combine
import UIKit

// Proximity discovery — BLE RSSI + UWB (NearbyInteraction) when available (Phase 13.5)
// When paired phones approach each other, avatars react: wave, run, celebrate.

// MARK: - Proximity Level

enum ProximityLevel: Int, Comparable, Equatable {
    case unknown    = 0
    case far        = 1   // > 5m  (RSSI < -80 dBm)
    case approaching = 2  // 2–5m  (RSSI -80..-65)
    case nearby     = 3   // 0.5–2m (RSSI -65..-45) — wave trigger
    case close      = 4   // < 0.5m (RSSI > -45)   — high-five trigger

    static func < (lhs: ProximityLevel, rhs: ProximityLevel) -> Bool {
        lhs.rawValue < rhs.rawValue
    }

    var displayName: String {
        switch self {
        case .unknown:    return "Searching…"
        case .far:        return "Far away"
        case .approaching: return "Approaching"
        case .nearby:     return "Nearby"
        case .close:      return "Right next to you!"
        }
    }
}

// MARK: - Service

@MainActor
final class ProximityService: NSObject, ObservableObject {

    static let shared = ProximityService()

    @Published private(set) var proximityLevel: ProximityLevel = .unknown
    @Published private(set) var uwbDistance: Float? = nil
    @Published private(set) var peerFound = false

    private weak var avatarEngine: AvatarEngine?

    // BLE
    private var central: CBCentralManager?
    private var peripheral: CBPeripheralManager?
    private let hookbotServiceUUID = CBUUID(string: "F1A2B3C4-D5E6-7890-ABCD-EF1234567890")
    private var trackedPeripheral: CBPeripheral?

    // UWB
    @available(iOS 14.0, *)
    private var niSession: NISession? {
        get { _niSession as? NISession }
        set { _niSession = newValue }
    }
    private var _niSession: AnyObject?

    private override init() { super.init() }

    func configure(engine: AvatarEngine) {
        self.avatarEngine = engine
    }

    // MARK: - Start / Stop

    func start() {
        central = CBCentralManager(delegate: self, queue: .main)
        peripheral = CBPeripheralManager(delegate: self, queue: .main)
        if NISession.deviceCapabilities.supportsPreciseDistanceMeasurement {
            startNISession()
        }
    }

    func stop() {
        central?.stopScan()
        peripheral?.stopAdvertising()
        central = nil
        peripheral = nil
        if #available(iOS 14.0, *) { niSession?.invalidate() }
        proximityLevel = .unknown
        peerFound = false
    }

    // MARK: - UWB (NearbyInteraction)

    @available(iOS 14.0, *)
    private func startNISession() {
        let session = NISession()
        session.delegate = self
        niSession = session

        // Share our discovery token via MultipeerConnectivity
        guard let token = session.discoveryToken,
              let tokenData = try? NSKeyedArchiver.archivedData(withRootObject: token, requiringSecureCoding: true) else { return }
        let base64 = tokenData.base64EncodedString()
        MultipeerService.shared.send(.niTokenData(base64: base64))
    }

    @available(iOS 14.0, *)
    func handlePeerNIToken(base64: String) {
        guard let data = Data(base64Encoded: base64),
              let token = try? NSKeyedUnarchiver.unarchivedObject(ofClass: NIDiscoveryToken.self, from: data) else { return }
        let config = NINearbyPeerConfiguration(peerToken: token)
        niSession?.run(config)
    }

    // MARK: - RSSI Proximity Update (BLE fallback)

    private func updateFromRSSI(_ rssi: Int) {
        let newLevel: ProximityLevel
        switch rssi {
        case ..<(-80): newLevel = .far
        case -80 ..< -65: newLevel = .approaching
        case -65 ..< -45: newLevel = .nearby
        default: newLevel = .close
        }
        applyProximityChange(to: newLevel)
    }

    // MARK: - UWB Distance Update

    private func updateFromDistance(_ meters: Float) {
        uwbDistance = meters
        let newLevel: ProximityLevel
        switch meters {
        case ..<0.4: newLevel = .close
        case 0.4..<1.5: newLevel = .nearby
        case 1.5..<4.0: newLevel = .approaching
        default: newLevel = .far
        }
        applyProximityChange(to: newLevel)
    }

    // MARK: - React

    private func applyProximityChange(to newLevel: ProximityLevel) {
        let old = proximityLevel
        guard newLevel != old else { return }
        proximityLevel = newLevel

        guard let engine = avatarEngine else { return }

        switch (old, newLevel) {
        case (_, .approaching) where old <= .far:
            // Friend approaching — wave
            SocialService.shared.handleIncomingRaid(
                from: MultipeerService.shared.peerName,
                emote: .wave
            )

        case (_, .nearby):
            // Getting close — excited bounce
            engine.setState(.success)
            HapticManager.playEvent(.taskComplete)

        case (_, .close):
            // Side by side — celebrate, trigger high-five
            engine.setState(.success)
            HapticManager.playEvent(.friendVisit)
            MultipeerService.shared.send(.interaction(type: "highFive"))

        case (.close, _) where newLevel < .nearby,
             (.nearby, _) where newLevel <= .far:
            // Parting — return to idle
            engine.setState(.idle)

        default:
            break
        }
    }
}

// MARK: - CBCentralManagerDelegate

extension ProximityService: CBCentralManagerDelegate {

    nonisolated func centralManagerDidUpdateState(_ central: CBCentralManager) {
        guard central.state == .poweredOn else { return }
        central.scanForPeripherals(
            withServices: [CBUUID(string: "F1A2B3C4-D5E6-7890-ABCD-EF1234567890")],
            options: [CBCentralManagerScanOptionAllowDuplicatesKey: true]
        )
    }

    nonisolated func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral, advertisementData: [String: Any], rssi RSSI: NSNumber) {
        let rssiValue = RSSI.intValue
        Task { @MainActor in
            peerFound = true
            updateFromRSSI(rssiValue)
        }
    }

    nonisolated func centralManager(_ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?) {
        Task { @MainActor in
            peerFound = false
            applyProximityChange(to: .unknown)
        }
    }
}

// MARK: - CBPeripheralManagerDelegate

extension ProximityService: CBPeripheralManagerDelegate {

    nonisolated func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        guard peripheral.state == .poweredOn else { return }
        let service = CBMutableService(
            type: CBUUID(string: "F1A2B3C4-D5E6-7890-ABCD-EF1234567890"),
            primary: true
        )
        peripheral.add(service)
        peripheral.startAdvertising([
            CBAdvertisementDataServiceUUIDsKey: [CBUUID(string: "F1A2B3C4-D5E6-7890-ABCD-EF1234567890")],
            CBAdvertisementDataLocalNameKey: "Hookbot"
        ])
    }
}

// MARK: - NISessionDelegate

@available(iOS 14.0, *)
extension ProximityService: NISessionDelegate {

    nonisolated func session(_ session: NISession, didUpdate nearbyObjects: [NINearbyObject]) {
        guard let first = nearbyObjects.first,
              let dist = first.distance else { return }
        Task { @MainActor in
            updateFromDistance(dist)
        }
    }

    nonisolated func session(_ session: NISession, didRemove nearbyObjects: [NINearbyObject], reason: NINearbyObject.RemovalReason) {
        Task { @MainActor in
            applyProximityChange(to: .far)
        }
    }

    nonisolated func sessionWasSuspended(_ session: NISession) {}
    nonisolated func sessionSuspensionEnded(_ session: NISession) {}

    nonisolated func session(_ session: NISession, didInvalidateWith error: Error) {
        Task { @MainActor in
            applyProximityChange(to: .unknown)
        }
    }
}
