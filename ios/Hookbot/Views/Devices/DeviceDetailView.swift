import SwiftUI

struct DeviceDetailView: View {
    @EnvironmentObject var engine: AvatarEngine
    let device: DeviceWithStatus

    @State private var selectedTab: DetailTab = .status
    @State private var config = DeviceConfig()
    @State private var history: [StatusHistoryEntry] = []
    @State private var isLoading = false
    @State private var isSaving = false
    @State private var statusMessage: String?
    @State private var errorMessage: String?

    enum DetailTab: String, CaseIterable {
        case status = "Status"
        case config = "Config"
        case history = "History"
    }

    private let states = ["idle", "coding", "reviewing", "meeting", "break", "away"]

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                tabPicker

                if isLoading && selectedTab == .history && history.isEmpty {
                    LoadingStateView()
                } else {
                    switch selectedTab {
                    case .status:
                        statusTab
                    case .config:
                        configTab
                    case .history:
                        historyTab
                    }
                }

                if let statusMessage {
                    Text(statusMessage)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundColor(.green)
                        .padding(10)
                        .frame(maxWidth: .infinity)
                        .background(RoundedRectangle(cornerRadius: 8).fill(Color.green.opacity(0.1)))
                }

                if let errorMessage {
                    Text(errorMessage)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundColor(.orange)
                        .padding(10)
                        .frame(maxWidth: .infinity)
                        .background(RoundedRectangle(cornerRadius: 8).fill(Color.orange.opacity(0.1)))
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle(device.name)
        .onAppear {
            fetchConfig()
            fetchHistory()
        }
        .refreshable {
            fetchConfig()
            fetchHistory()
        }
    }

    // MARK: - Tab Picker

    private var tabPicker: some View {
        HStack(spacing: 8) {
            ForEach(DetailTab.allCases, id: \.self) { tab in
                Button {
                    withAnimation(.easeInOut(duration: 0.2)) { selectedTab = tab }
                } label: {
                    Text(tab.rawValue)
                        .font(.system(size: 13, weight: .semibold, design: .monospaced))
                        .foregroundColor(selectedTab == tab ? .white : .gray)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)
                        .background(
                            RoundedRectangle(cornerRadius: 8)
                                .fill(selectedTab == tab ? Color(white: 0.2) : .clear)
                        )
                }
                .buttonStyle(.plain)
            }
            Spacer()
        }
    }

    // MARK: - Status Tab

    @ViewBuilder
    private var statusTab: some View {
        VStack(spacing: 12) {
            // Info cards
            HStack(spacing: 10) {
                infoCard(label: "STATE", value: device.latestStatus?.state ?? "unknown", color: .cyan)
                infoCard(label: "FIRMWARE", value: device.firmwareVersion ?? "n/a", color: .white)
            }
            HStack(spacing: 10) {
                infoCard(label: "UPTIME", value: formatUptime(device.latestStatus?.uptime), color: .green)
                infoCard(label: "FREE HEAP", value: formatHeap(device.latestStatus?.freeHeap), color: .orange)
            }

            // State buttons
            VStack(alignment: .leading, spacing: 10) {
                Text("SET STATE")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundColor(.gray)

                LazyVGrid(columns: [
                    GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())
                ], spacing: 10) {
                    ForEach(states, id: \.self) { state in
                        Button {
                            sendState(state)
                        } label: {
                            Text(state.capitalized)
                                .font(.system(size: 12, weight: .semibold, design: .monospaced))
                                .foregroundColor(device.latestStatus?.state == state ? .black : .white)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 10)
                                .background(
                                    RoundedRectangle(cornerRadius: 8)
                                        .fill(device.latestStatus?.state == state ? Color.cyan : Color(white: 0.15))
                                )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding(16)
            .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
        }
    }

    // MARK: - Config Tab

    @ViewBuilder
    private var configTab: some View {
        VStack(spacing: 12) {
            configField(label: "Device Name", text: Binding(
                get: { config.name ?? "" },
                set: { config.name = $0 }
            ))

            configField(label: "Screensaver (mins)", text: Binding(
                get: { config.screensaverMins.map { String($0) } ?? "" },
                set: { config.screensaverMins = Int($0) }
            ))

            toggleRow(label: "Sound Enabled", isOn: Binding(
                get: { config.soundEnabled ?? false },
                set: { config.soundEnabled = $0 }
            ))

            toggleRow(label: "Do Not Disturb", isOn: Binding(
                get: { config.doNotDisturb ?? false },
                set: { config.doNotDisturb = $0 }
            ))

            HStack(spacing: 10) {
                Button {
                    saveConfig()
                } label: {
                    Text("Save")
                        .font(.system(size: 13, weight: .bold, design: .monospaced))
                        .foregroundColor(.black)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 12)
                        .background(RoundedRectangle(cornerRadius: 10).fill(Color.cyan))
                }
                .buttonStyle(.plain)
                .disabled(isSaving)

                Button {
                    pushConfig()
                } label: {
                    Text("Push")
                        .font(.system(size: 13, weight: .bold, design: .monospaced))
                        .foregroundColor(.cyan)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 12)
                        .background(
                            RoundedRectangle(cornerRadius: 10)
                                .stroke(Color.cyan, lineWidth: 1)
                        )
                }
                .buttonStyle(.plain)
                .disabled(isSaving)
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - History Tab

    @ViewBuilder
    private var historyTab: some View {
        if history.isEmpty {
            EmptyStateView(icon: "clock", message: "No status history yet")
        } else {
            VStack(spacing: 8) {
                ForEach(history) { entry in
                    HStack(spacing: 10) {
                        Circle()
                            .fill(stateColor(entry.state))
                            .frame(width: 8, height: 8)

                        VStack(alignment: .leading, spacing: 2) {
                            Text(entry.state.capitalized)
                                .font(.system(size: 13, weight: .semibold, design: .monospaced))
                                .foregroundColor(.white)
                            if let tool = entry.tool {
                                Text(tool)
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundColor(.cyan)
                            }
                        }

                        Spacer()

                        if let ts = entry.createdAt {
                            Text(String(ts.prefix(16)))
                                .font(.system(size: 10, design: .monospaced))
                                .foregroundColor(.gray)
                        }
                    }
                    .padding(12)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.08)))
                }
            }
        }
    }

    // MARK: - Components

    private func infoCard(label: String, value: String, color: Color) -> some View {
        VStack(spacing: 4) {
            Text(label)
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)
            Text(value)
                .font(.system(size: 16, weight: .bold, design: .monospaced))
                .foregroundColor(color)
        }
        .frame(maxWidth: .infinity)
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func configField(label: String, text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label.uppercased())
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)
            TextField(label, text: text)
                .font(.system(size: 14, design: .monospaced))
                .foregroundColor(.white)
                .padding(10)
                .background(RoundedRectangle(cornerRadius: 8).fill(Color(white: 0.12)))
        }
    }

    private func toggleRow(label: String, isOn: Binding<Bool>) -> some View {
        HStack {
            Text(label)
                .font(.system(size: 13, design: .monospaced))
                .foregroundColor(.white)
            Spacer()
            Toggle("", isOn: isOn)
                .tint(.cyan)
                .labelsHidden()
        }
        .padding(.vertical, 4)
    }

    private func stateColor(_ state: String) -> Color {
        switch state {
        case "idle": return .green
        case "coding": return .cyan
        case "meeting": return .orange
        case "break": return .yellow
        case "away": return .gray
        default: return .white
        }
    }

    private func formatUptime(_ seconds: Int?) -> String {
        guard let s = seconds else { return "n/a" }
        let h = s / 3600
        let m = (s % 3600) / 60
        return "\(h)h \(m)m"
    }

    private func formatHeap(_ bytes: Int?) -> String {
        guard let b = bytes else { return "n/a" }
        return "\(b / 1024)KB"
    }

    // MARK: - API

    private func sendState(_ state: String) {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/devices/\(device.id)/state") else { return }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }
        request.httpBody = try? JSONSerialization.data(withJSONObject: ["state": state])

        URLSession.shared.dataTask(with: request) { data, response, _ in
            DispatchQueue.main.async {
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    errorMessage = "Failed to set state"
                    return
                }
                statusMessage = "State set to \(state)"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
            }
        }.resume()
    }

    private func fetchConfig() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/devices/\(device.id)/config") else { return }

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, _, _ in
            DispatchQueue.main.async {
                guard let data else { return }
                if let decoded = try? JSONDecoder().decode(DeviceConfig.self, from: data) {
                    config = decoded
                }
            }
        }.resume()
    }

    private func saveConfig() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/devices/\(device.id)/config") else { return }
        isSaving = true

        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }
        request.httpBody = try? JSONEncoder().encode(config)

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                isSaving = false
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    errorMessage = "Failed to save config"
                    return
                }
                statusMessage = "Config saved"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
            }
        }.resume()
    }

    private func pushConfig() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/devices/\(device.id)/config/push") else { return }
        isSaving = true

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                isSaving = false
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    errorMessage = "Failed to push config"
                    return
                }
                statusMessage = "Config pushed to device"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
            }
        }.resume()
    }

    private func fetchHistory() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/devices/\(device.id)/history") else { return }
        isLoading = true

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, _, _ in
            DispatchQueue.main.async {
                isLoading = false
                guard let data else { return }
                if let decoded = try? JSONDecoder().decode([StatusHistoryEntry].self, from: data) {
                    history = decoded
                }
            }
        }.resume()
    }
}
