import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.dismiss) var dismiss

    @State private var soundEnabled: Bool = true
    @State private var serverURL: String = ""
    @State private var deviceName: String = "hookbot-ios"
    @State private var accessories: AccessoriesConfig = AccessoriesConfig()
    @State private var testResult: ServerTestResult?
    @State private var isTesting = false

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
                    Toggle("Cigar", isOn: $accessories.cigar)
                    Toggle("Crown", isOn: $accessories.crown)
                    Toggle("Devil Horns", isOn: $accessories.horns)
                    Toggle("Halo", isOn: $accessories.halo)
                    Toggle("Glasses", isOn: $accessories.glasses)
                    Toggle("Monocle", isOn: $accessories.monocle)
                    Toggle("Bow Tie", isOn: $accessories.bowtie)
                }

                Section("Network") {
                    if let ip = NetworkService.localIPAddress() {
                        LabeledContent("Local IP") {
                            Text(ip).font(.system(.body, design: .monospaced))
                        }
                        LabeledContent("HTTP Server") {
                            Text("http://\(ip):8080").font(.system(.caption, design: .monospaced))
                        }
                    }
                    LabeledContent("API Endpoints") {
                        VStack(alignment: .trailing) {
                            Text("POST /state").font(.system(.caption2, design: .monospaced))
                            Text("GET /status").font(.system(.caption2, design: .monospaced))
                            Text("POST /config").font(.system(.caption2, design: .monospaced))
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
                deviceName = engine.config.deviceName
                accessories = engine.config.accessories
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
        engine.config.soundEnabled = soundEnabled
        engine.config.serverURL = serverURL
        engine.config.deviceName = deviceName
        engine.config.accessories = accessories

        if let data = try? JSONEncoder().encode(engine.config) {
            UserDefaults.standard.set(data, forKey: "hookbot_config")
        }
    }
}

private struct ServerTestResult {
    let success: Bool
    let message: String
    let detail: String?
}
