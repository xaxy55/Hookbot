import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.dismiss) var dismiss

    @EnvironmentObject var network: NetworkService
    @State private var soundEnabled: Bool = true
    @State private var serverURL: String = ""
    @State private var apiKey: String = ""
    @State private var deviceId: String = ""
    #if targetEnvironment(macCatalyst)
    @State private var deviceName: String = "hookbot-mac"
    #else
    @State private var deviceName: String = "hookbot-ios"
    #endif
    @State private var accessories: AccessoriesConfig = AccessoriesConfig()
    @State private var testResult: ServerTestResult?
    @State private var isTesting = false
    @State private var showCigarAgeGate = false

    var body: some View {
        NavigationStack {
            Form {
                Section("Device") {
                    TextField("Device Name", text: $deviceName)
                        .font(.system(.body, design: .monospaced))
                    TextField("Management Server URL", text: $serverURL)
                        .font(.system(.body, design: .monospaced))
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    TextField("API Key", text: $apiKey)
                        .font(.system(.body, design: .monospaced))
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    TextField("Device ID (auto-assigned on register)", text: $deviceId)
                        .font(.system(.body, design: .monospaced))
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()

                    Button {
                        testServerConnection()
                    } label: {
                        HStack {
                            if isTesting {
                                ProgressView()
                                    .controlSize(.small)
                                Text("Testing...")
                            } else {
                                Image(systemName: "bolt.horizontal")
                                Text("Test Connection")
                            }
                        }
                    }
                    .disabled(serverURL.isEmpty || isTesting)

                    if let result = testResult {
                        HStack(spacing: 8) {
                            Image(systemName: result.success ? "checkmark.circle.fill" : "xmark.circle.fill")
                                .foregroundColor(result.success ? .green : .red)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(result.message)
                                    .font(.system(.caption, design: .monospaced))
                                if let detail = result.detail {
                                    Text(detail)
                                        .font(.system(.caption2, design: .monospaced))
                                        .foregroundColor(.secondary)
                                }
                            }
                        }
                    }
                }

                Section("Sound") {
                    Toggle("Sound Effects", isOn: $soundEnabled)
                }

                Section("Accessories") {
                    Toggle("Top Hat", isOn: $accessories.topHat)
                    Toggle("Crown", isOn: $accessories.crown)
                    Toggle("Devil Horns", isOn: $accessories.horns)
                    Toggle("Halo", isOn: $accessories.halo)
                    Toggle("Glasses", isOn: $accessories.glasses)
                    Toggle("Monocle", isOn: $accessories.monocle)
                    Toggle("Bow Tie", isOn: $accessories.bowtie)
                }

                Section {
                    Toggle("Cigar", isOn: Binding(
                        get: { accessories.cigar },
                        set: { newValue in
                            if newValue {
                                showCigarAgeGate = true
                            } else {
                                accessories.cigar = false
                            }
                        }
                    ))
                } header: {
                    Text("Mature Content")
                } footer: {
                    Text("Enabling the cigar requires age verification. This accessory may not be suitable for younger users.")
                        .font(.caption2)
                }

                Section("Connection") {
                    HStack(spacing: 8) {
                        Image(systemName: network.isConnected ? "circle.fill" : "circle")
                            .foregroundColor(network.isConnected ? .green : .red)
                            .font(.caption2)
                        Text(network.isConnected ? "Connected (polling)" : "Not connected")
                            .font(.system(.body, design: .monospaced))
                    }
                    if let ip = NetworkService.localIPAddress() {
                        LabeledContent("Local IP") {
                            Text(ip).font(.system(.body, design: .monospaced))
                        }
                    }
                }
            }
            .navigationTitle("Settings")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") {
                        save()
                        dismiss()
                    }
                }
            }
            .onAppear {
                soundEnabled = engine.config.soundEnabled
                serverURL = engine.config.serverURL
                apiKey = engine.config.apiKey
                deviceId = engine.config.deviceId
                deviceName = engine.config.deviceName
                accessories = engine.config.accessories
            }
            .alert("Age Verification", isPresented: $showCigarAgeGate) {
                Button("I am 17 or older") {
                    accessories.cigar = true
                }
                Button("Cancel", role: .cancel) { }
            } message: {
                Text("The cigar accessory is intended for users aged 17 and older. By enabling this, you confirm that you meet the age requirement.")
            }
        }
    }

    // MARK: - Test Connection

    private func testServerConnection() {
        let urlString = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !urlString.isEmpty else { return }

        // Test /api/health first, fall back to /api/devices
        guard let url = URL(string: "\(urlString)/api/health") else {
            testResult = ServerTestResult(success: false, message: "Invalid URL", detail: nil)
            return
        }

        isTesting = true
        testResult = nil

        var request = URLRequest(url: url)
        request.timeoutInterval = 5
        if !apiKey.isEmpty {
            request.setValue(apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                isTesting = false

                if let error {
                    testResult = ServerTestResult(
                        success: false,
                        message: "Connection failed",
                        detail: error.localizedDescription
                    )
                    return
                }

                guard let http = response as? HTTPURLResponse else {
                    testResult = ServerTestResult(success: false, message: "No response", detail: nil)
                    return
                }

                if http.statusCode == 200 {
                    var detail = "HTTP \(http.statusCode)"
                    if let data, let body = String(data: data, encoding: .utf8), !body.isEmpty {
                        detail += " - \(String(body.prefix(80)))"
                    }
                    testResult = ServerTestResult(
                        success: true,
                        message: "Server reachable",
                        detail: detail
                    )
                } else if http.statusCode == 404 {
                    // /api/health not found, try base URL
                    testResult = ServerTestResult(
                        success: true,
                        message: "Server reachable (no /api/health)",
                        detail: "HTTP \(http.statusCode) - server responds but health endpoint not found"
                    )
                } else {
                    testResult = ServerTestResult(
                        success: false,
                        message: "Server error",
                        detail: "HTTP \(http.statusCode)"
                    )
                }
            }
        }.resume()
    }

    private func save() {
        let serverChanged = engine.config.serverURL != serverURL || engine.config.deviceId != deviceId

        engine.config.soundEnabled = soundEnabled
        engine.config.serverURL = serverURL
        engine.config.apiKey = apiKey
        engine.config.deviceId = deviceId
        engine.config.deviceName = deviceName
        engine.config.accessories = accessories

        if let data = try? JSONEncoder().encode(engine.config) {
            UserDefaults.standard.set(data, forKey: "hookbot_config")
        }

        // Restart polling if server or device ID changed
        if serverChanged {
            network.startPolling()
        }
    }
}

private struct ServerTestResult {
    let success: Bool
    let message: String
    let detail: String?
}
