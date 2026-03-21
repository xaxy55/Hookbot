import CoreBluetooth
import Foundation

/// Discovers unclaimed Hookbot devices via BLE and claims them via the server API.
@MainActor
final class DeviceClaimService: NSObject, ObservableObject {

    // MARK: - Published State

    @Published var discoveredDevices: [DiscoveredDevice] = []
    @Published var isScanning = false
    @Published var claimState: ClaimState = .idle
    @Published var errorMessage: String?

    // MARK: - Types

    struct DiscoveredDevice: Identifiable, Equatable {
        let id: UUID              // CBPeripheral identifier
        let name: String          // BLE advertised name (e.g. "Hookbot-AB12")
        var claimCode: String     // 6-char code from CLAIM_INFO characteristic
        var deviceName: String    // hostname from claim info JSON
        var rssi: Int
        var claimed: Bool

        static func == (lhs: DiscoveredDevice, rhs: DiscoveredDevice) -> Bool {
            lhs.id == rhs.id
        }
    }

    enum ClaimState: Equatable {
        case idle
        case claiming
        case success(deviceId: String, name: String)
        case failed(String)
    }

    // MARK: - BLE UUIDs (must match firmware ble_prov.cpp)

    private let hookbotServiceUUID = CBUUID(string: "4fafc201-1fb5-459e-8fcc-c5c9c331914b")
    private let claimInfoUUID = CBUUID(string: "beb5483e-36e1-4688-b7f5-ea07361b26aa")

    // MARK: - Private

    private var central: CBCentralManager?
    private var connectedPeripherals: [UUID: CBPeripheral] = [:]
    private let session: URLSession

    // MARK: - Init

    override init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 15
        self.session = URLSession(configuration: config)
        super.init()
    }

    // MARK: - Scanning

    func startScan() {
        discoveredDevices.removeAll()
        errorMessage = nil
        isScanning = true
        if central == nil {
            central = CBCentralManager(delegate: self, queue: .main)
        } else if central?.state == .poweredOn {
            beginScanning()
        }
    }

    func stopScan() {
        isScanning = false
        central?.stopScan()
        // Disconnect any peripherals we connected to for reading claim info
        for (_, peripheral) in connectedPeripherals {
            central?.cancelPeripheralConnection(peripheral)
        }
        connectedPeripherals.removeAll()
    }

    private func beginScanning() {
        central?.scanForPeripherals(
            withServices: [hookbotServiceUUID],
            options: [CBCentralManagerScanOptionAllowDuplicatesKey: false]
        )
    }

    // MARK: - Claim via API

    func claimDevice(code: String, name: String?, serverURL: String, apiKey: String) {
        let trimmedCode = code.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard trimmedCode.count == 6 else {
            claimState = .failed("Claim code must be 6 characters")
            return
        }

        claimState = .claiming
        errorMessage = nil

        let urlStr = "\(serverURL)/api/devices/claim"
        guard let url = URL(string: urlStr) else {
            claimState = .failed("Invalid server URL")
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !apiKey.isEmpty {
            request.setValue(apiKey, forHTTPHeaderField: "X-API-Key")
        }

        var body: [String: Any] = ["claim_code": trimmedCode]
        if let name, !name.isEmpty {
            body["name"] = name
        }
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)

        session.dataTask(with: request) { [weak self] data, response, error in
            DispatchQueue.main.async {
                guard let self else { return }

                if let error {
                    self.claimState = .failed(error.localizedDescription)
                    return
                }

                guard let http = response as? HTTPURLResponse, let data else {
                    self.claimState = .failed("No response from server")
                    return
                }

                guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                    self.claimState = .failed("Invalid server response")
                    return
                }

                if http.statusCode == 200, json["ok"] as? Bool == true {
                    let deviceId = json["device_id"] as? String ?? ""
                    let deviceName = json["name"] as? String ?? ""
                    self.claimState = .success(deviceId: deviceId, name: deviceName)
                } else {
                    let errorMsg = json["error"] as? String ?? "Claim failed (HTTP \(http.statusCode))"
                    self.claimState = .failed(errorMsg)
                }
            }
        }.resume()
    }

    func reset() {
        claimState = .idle
        errorMessage = nil
    }

    // MARK: - Parse Claim Info JSON

    private func parseClaimInfo(_ data: Data, for peripheralId: UUID, rssi: Int, bleName: String) {
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }

        let claimCode = json["claim_code"] as? String ?? ""
        let claimed = json["claimed"] as? Bool ?? false
        let deviceName = json["name"] as? String ?? bleName

        // Only show unclaimed devices
        guard !claimed, !claimCode.isEmpty else { return }

        let device = DiscoveredDevice(
            id: peripheralId,
            name: bleName,
            claimCode: claimCode,
            deviceName: deviceName,
            rssi: rssi,
            claimed: claimed
        )

        if let idx = discoveredDevices.firstIndex(where: { $0.id == peripheralId }) {
            discoveredDevices[idx] = device
        } else {
            discoveredDevices.append(device)
        }
    }
}

// MARK: - CBCentralManagerDelegate

extension DeviceClaimService: CBCentralManagerDelegate {

    nonisolated func centralManagerDidUpdateState(_ central: CBCentralManager) {
        Task { @MainActor in
            switch central.state {
            case .poweredOn:
                if isScanning {
                    beginScanning()
                }
            case .poweredOff:
                errorMessage = "Bluetooth is turned off"
                isScanning = false
            case .unauthorized:
                errorMessage = "Bluetooth permission denied"
                isScanning = false
            default:
                break
            }
        }
    }

    nonisolated func centralManager(
        _ central: CBCentralManager,
        didDiscover peripheral: CBPeripheral,
        advertisementData: [String: Any],
        rssi RSSI: NSNumber
    ) {
        Task { @MainActor in
            // Connect to read the claim info characteristic
            if connectedPeripherals[peripheral.identifier] == nil {
                connectedPeripherals[peripheral.identifier] = peripheral
                peripheral.delegate = self
                central.connect(peripheral, options: nil)
            }
        }
    }

    nonisolated func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        Task { @MainActor in
            peripheral.discoverServices([hookbotServiceUUID])
        }
    }

    nonisolated func centralManager(_ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?) {
        Task { @MainActor in
            connectedPeripherals.removeValue(forKey: peripheral.identifier)
        }
    }

    nonisolated func centralManager(_ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?) {
        Task { @MainActor in
            connectedPeripherals.removeValue(forKey: peripheral.identifier)
        }
    }
}

// MARK: - CBPeripheralDelegate

extension DeviceClaimService: CBPeripheralDelegate {

    nonisolated func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        Task { @MainActor in
            guard let service = peripheral.services?.first(where: { $0.uuid == hookbotServiceUUID }) else { return }
            peripheral.discoverCharacteristics([claimInfoUUID], for: service)
        }
    }

    nonisolated func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
        Task { @MainActor in
            guard let characteristic = service.characteristics?.first(where: { $0.uuid == claimInfoUUID }) else { return }
            peripheral.readValue(for: characteristic)
            // Subscribe for updates
            peripheral.setNotifyValue(true, for: characteristic)
        }
    }

    nonisolated func peripheral(_ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?) {
        Task { @MainActor in
            guard characteristic.uuid == claimInfoUUID,
                  let data = characteristic.value else { return }

            let bleName = peripheral.name ?? "Hookbot"
            parseClaimInfo(data, for: peripheral.identifier, rssi: -50, bleName: bleName)
        }
    }
}
