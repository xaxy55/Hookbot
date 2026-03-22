import SwiftUI
import AVFoundation

/// Device claiming flow — BLE auto-discover, camera QR scan, or manual code entry (Phase 14.3)
struct ClaimDeviceView: View {
    @EnvironmentObject var engine: AvatarEngine
    @EnvironmentObject var network: NetworkService
    @Environment(\.dismiss) var dismiss
    @StateObject private var claimService = DeviceClaimService()

    @State private var manualCode = ""
    @State private var deviceName = ""
    @State private var showManualEntry = false
    @State private var showCamera = false
    @FocusState private var codeFieldFocused: Bool

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 24) {
                    header

                    switch claimService.claimState {
                    case .idle:
                        idleContent
                    case .claiming:
                        claimingContent
                    case .success(let deviceId, let name):
                        successContent(deviceId: deviceId, name: name)
                    case .failed(let error):
                        failedContent(error: error)
                    }
                }
                .padding()
            }
            .background(Color.black)
            .navigationTitle("Claim Device")
            .navigationBarTitleDisplayMode(.inline)
            .preferredColorScheme(.dark)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                        .font(.system(size: 14, design: .monospaced))
                }
            }
            .onDisappear {
                claimService.stopScan()
            }
            .onChange(of: claimService.claimState) { _, newState in
                if case .success(let deviceId, _) = newState, !deviceId.isEmpty {
                    engine.config.deviceId = deviceId
                    if let data = try? JSONEncoder().encode(engine.config) {
                        UserDefaults.standard.set(data, forKey: "hookbot_config")
                    }
                    network.startPolling()
                }
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        VStack(spacing: 8) {
            Image(systemName: "wave.3.right.circle.fill")
                .font(.system(size: 50))
                .foregroundStyle(.green)
            Text("Add a Hookbot device to your account.")
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
        }
    }

    // MARK: - Idle Content

    private var idleContent: some View {
        VStack(spacing: 16) {
            // BLE scan
            actionCard(
                icon: "antenna.radiowaves.left.and.right",
                title: "Scan nearby devices",
                subtitle: "Find unclaimed Hookbots via Bluetooth"
            ) {
                claimService.startScan()
            }

            // Discovered devices list
            if !claimService.discoveredDevices.isEmpty {
                discoveredDevicesList
            }

            if claimService.isScanning && claimService.discoveredDevices.isEmpty {
                scanningIndicator
            }

            // Camera scan
            actionCard(
                icon: "qrcode.viewfinder",
                title: "Scan QR code",
                subtitle: "Point camera at device screen"
            ) {
                showCamera = true
            }

            // Manual entry
            actionCard(
                icon: "keyboard",
                title: "Enter claim code",
                subtitle: "Type the 6-character code from device"
            ) {
                showManualEntry = true
            }

            if showManualEntry {
                manualEntryForm
            }

            if let error = claimService.errorMessage {
                Text(error)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(.orange)
                    .padding(12)
                    .frame(maxWidth: .infinity)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Color.orange.opacity(0.1)))
            }
        }
        .sheet(isPresented: $showCamera) {
            QRScannerView { code in
                showCamera = false
                manualCode = code.uppercased()
                submitClaim(code: code)
            }
        }
    }

    // MARK: - Discovered Devices

    private var discoveredDevicesList: some View {
        VStack(spacing: 8) {
            HStack {
                Text("Nearby Devices")
                    .font(.system(size: 13, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)
                Spacer()
                if claimService.isScanning {
                    ProgressView()
                        .controlSize(.small)
                        .tint(.green)
                }
            }

            ForEach(claimService.discoveredDevices) { device in
                Button {
                    manualCode = device.claimCode
                    deviceName = device.deviceName
                    submitClaim(code: device.claimCode)
                } label: {
                    HStack(spacing: 12) {
                        Image(systemName: "cpu")
                            .font(.system(size: 20))
                            .foregroundStyle(.green)
                            .frame(width: 32)

                        VStack(alignment: .leading, spacing: 3) {
                            Text(device.deviceName)
                                .font(.system(size: 14, weight: .semibold, design: .monospaced))
                                .foregroundStyle(.white)
                            HStack(spacing: 8) {
                                Text(device.claimCode)
                                    .font(.system(size: 12, weight: .bold, design: .monospaced))
                                    .foregroundStyle(.green)
                                signalStrength(rssi: device.rssi)
                            }
                        }

                        Spacer()

                        Text("Claim")
                            .font(.system(size: 12, weight: .bold, design: .monospaced))
                            .foregroundStyle(.black)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(Capsule().fill(Color.green))
                    }
                    .padding(14)
                    .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
                }
                .buttonStyle(.plain)
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 16).fill(Color(white: 0.05)))
    }

    private func signalStrength(rssi: Int) -> some View {
        HStack(spacing: 2) {
            ForEach(0..<3) { i in
                RoundedRectangle(cornerRadius: 1)
                    .fill(rssi > -60 + (i * 15) ? Color.green : Color(white: 0.3))
                    .frame(width: 3, height: CGFloat(6 + i * 3))
            }
        }
    }

    private var scanningIndicator: some View {
        VStack(spacing: 12) {
            ProgressView()
                .controlSize(.large)
                .tint(.green)
            Text("Scanning for nearby Hookbots...")
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)

            Button("Stop Scanning") {
                claimService.stopScan()
            }
            .font(.system(size: 12, design: .monospaced))
            .foregroundStyle(.red)
        }
        .padding(24)
        .frame(maxWidth: .infinity)
        .background(RoundedRectangle(cornerRadius: 16).fill(Color(white: 0.05)))
    }

    // MARK: - Manual Entry

    private var manualEntryForm: some View {
        VStack(spacing: 12) {
            TextField("Enter 6-char code", text: $manualCode)
                .keyboardType(.asciiCapable)
                .textInputAutocapitalization(.characters)
                .autocorrectionDisabled()
                .font(.system(size: 24, weight: .bold, design: .monospaced))
                .multilineTextAlignment(.center)
                .padding()
                .background(Color(white: 0.1))
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .focused($codeFieldFocused)
                .onAppear { codeFieldFocused = true }
                .onChange(of: manualCode) { _, newValue in
                    // Filter to valid claim code chars (A-Z, 2-9)
                    let filtered = String(newValue.uppercased().filter { c in
                        (c >= "A" && c <= "Z" && c != "I" && c != "L" && c != "O") ||
                        (c >= "2" && c <= "9")
                    }.prefix(6))
                    if filtered != manualCode {
                        manualCode = filtered
                    }
                }

            TextField("Device name (optional)", text: $deviceName)
                .font(.system(size: 14, design: .monospaced))
                .padding()
                .background(Color(white: 0.1))
                .clipShape(RoundedRectangle(cornerRadius: 12))

            Button {
                submitClaim(code: manualCode)
            } label: {
                Text("Claim Device")
                    .font(.system(size: 16, weight: .bold, design: .monospaced))
                    .foregroundStyle(.black)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.green)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .disabled(manualCode.count < 6)
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 16).fill(Color(white: 0.08)))
    }

    // MARK: - Claiming

    private var claimingContent: some View {
        VStack(spacing: 16) {
            ProgressView()
                .controlSize(.large)
                .tint(.green)
            Text("Claiming device...")
                .font(.system(size: 14, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .padding(40)
    }

    // MARK: - Success

    private func successContent(deviceId: String, name: String) -> some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 50))
                .foregroundStyle(.green)

            Text("Device Claimed!")
                .font(.system(size: 20, weight: .bold, design: .monospaced))
                .foregroundStyle(.green)

            if !name.isEmpty {
                infoRow(label: "Name", value: name)
            }
            infoRow(label: "Device ID", value: String(deviceId.prefix(8)) + "...")

            Button {
                dismiss()
            } label: {
                Text("Done")
                    .font(.system(size: 16, weight: .bold, design: .monospaced))
                    .foregroundStyle(.black)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.green)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
        }
        .padding(20)
        .background(RoundedRectangle(cornerRadius: 16).fill(Color(white: 0.08)))
    }

    // MARK: - Failed

    private func failedContent(error: String) -> some View {
        VStack(spacing: 12) {
            Image(systemName: "xmark.circle.fill")
                .font(.system(size: 40))
                .foregroundStyle(.red)
            Text("Claim Failed")
                .font(.system(size: 18, weight: .bold, design: .monospaced))
            Text(error)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)

            Button("Try Again") {
                claimService.reset()
                manualCode = ""
                deviceName = ""
            }
            .buttonStyle(.borderedProminent)
            .tint(.green)
        }
    }

    // MARK: - Helpers

    private func submitClaim(code: String) {
        claimService.claimDevice(
            code: code,
            name: deviceName.isEmpty ? nil : deviceName,
            serverURL: engine.config.serverURL,
            apiKey: engine.config.apiKey
        )
    }

    private func actionCard(icon: String, title: String, subtitle: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 16) {
                Image(systemName: icon)
                    .font(.system(size: 24))
                    .foregroundStyle(.green)
                    .frame(width: 40)

                VStack(alignment: .leading, spacing: 3) {
                    Text(title)
                        .font(.system(size: 15, weight: .semibold, design: .monospaced))
                        .foregroundStyle(.white)
                    Text(subtitle)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.gray)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .foregroundStyle(.gray)
            }
            .padding(16)
            .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
        }
        .buttonStyle(.plain)
    }

    private func infoRow(label: String, value: String) -> some View {
        HStack {
            Text(label)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
            Spacer()
            Text(value)
                .font(.system(size: 12, weight: .bold, design: .monospaced))
                .foregroundStyle(.white)
        }
        .padding(.horizontal)
    }
}

// MARK: - QR Scanner View

struct QRScannerView: UIViewControllerRepresentable {
    let onCodeScanned: (String) -> Void

    func makeUIViewController(context: Context) -> QRScannerViewController {
        let vc = QRScannerViewController()
        vc.onCodeScanned = onCodeScanned
        return vc
    }

    func updateUIViewController(_ uiViewController: QRScannerViewController, context: Context) {}
}

final class QRScannerViewController: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    var onCodeScanned: ((String) -> Void)?
    private var captureSession: AVCaptureSession?
    private var previewLayer: AVCaptureVideoPreviewLayer?
    private var hasScanned = false

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        setupCamera()
        setupOverlay()
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        previewLayer?.frame = view.bounds
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        if captureSession?.isRunning == false {
            DispatchQueue.global(qos: .userInitiated).async { [weak self] in
                self?.captureSession?.startRunning()
            }
        }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        if captureSession?.isRunning == true {
            captureSession?.stopRunning()
        }
    }

    private func setupCamera() {
        let session = AVCaptureSession()
        guard let device = AVCaptureDevice.default(for: .video),
              let input = try? AVCaptureDeviceInput(device: device) else {
            showNoCameraLabel()
            return
        }

        if session.canAddInput(input) {
            session.addInput(input)
        }

        let output = AVCaptureMetadataOutput()
        if session.canAddOutput(output) {
            session.addOutput(output)
            output.setMetadataObjectsDelegate(self, queue: .main)
            output.metadataObjectTypes = [.qr]
        }

        let layer = AVCaptureVideoPreviewLayer(session: session)
        layer.videoGravity = .resizeAspectFill
        layer.frame = view.bounds
        view.layer.addSublayer(layer)

        self.captureSession = session
        self.previewLayer = layer

        DispatchQueue.global(qos: .userInitiated).async {
            session.startRunning()
        }
    }

    private func setupOverlay() {
        // Crosshair / guide frame
        let guideSize: CGFloat = 220
        let guide = UIView(frame: CGRect(
            x: (view.bounds.width - guideSize) / 2,
            y: (view.bounds.height - guideSize) / 2,
            width: guideSize,
            height: guideSize
        ))
        guide.layer.borderColor = UIColor.systemGreen.cgColor
        guide.layer.borderWidth = 2
        guide.layer.cornerRadius = 16
        guide.autoresizingMask = [.flexibleTopMargin, .flexibleBottomMargin, .flexibleLeftMargin, .flexibleRightMargin]
        view.addSubview(guide)

        let label = UILabel()
        label.text = "Point at device QR code"
        label.textColor = .white
        label.font = .monospacedSystemFont(ofSize: 13, weight: .medium)
        label.textAlignment = .center
        label.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            label.topAnchor.constraint(equalTo: guide.bottomAnchor, constant: 24)
        ])
    }

    private func showNoCameraLabel() {
        let label = UILabel()
        label.text = "Camera not available"
        label.textColor = .gray
        label.font = .monospacedSystemFont(ofSize: 14, weight: .regular)
        label.textAlignment = .center
        label.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: view.centerYAnchor)
        ])
    }

    // MARK: - AVCaptureMetadataOutputObjectsDelegate

    func metadataOutput(
        _ output: AVCaptureMetadataOutput,
        didOutput metadataObjects: [AVMetadataObject],
        from connection: AVCaptureConnection
    ) {
        guard !hasScanned,
              let object = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
              object.type == .qr,
              let code = object.stringValue else { return }

        // Accept raw 6-char codes or hookbot:// URLs containing claim codes
        let claimCode: String
        if code.count == 6 {
            claimCode = code
        } else if code.lowercased().hasPrefix("hookbot://claim/") {
            claimCode = String(code.dropFirst("hookbot://claim/".count).prefix(6))
        } else {
            return // Not a valid claim code
        }

        hasScanned = true
        captureSession?.stopRunning()

        // Haptic feedback
        let generator = UINotificationFeedbackGenerator()
        generator.notificationOccurred(.success)

        onCodeScanned?(claimCode)
    }
}
