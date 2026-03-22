import SwiftUI

struct HomeAssistantView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var haConfig: HAConfig?
    @State private var isLoading = false
    @State private var isSyncing = false
    @State private var errorMessage: String?
    @State private var statusMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if isLoading && haConfig == nil {
                    LoadingStateView(message: "Loading Home Assistant...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchConfig() }
                } else if let config = haConfig {
                    haContent(config)
                } else {
                    EmptyStateView(icon: "house", message: "Home Assistant not configured")
                }

                if let statusMessage {
                    Text(statusMessage)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundColor(.green)
                        .padding(10)
                        .frame(maxWidth: .infinity)
                        .background(RoundedRectangle(cornerRadius: 8).fill(Color.green.opacity(0.1)))
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Home Assistant")
        .onAppear { fetchConfig() }
        .refreshable { fetchConfig() }
    }

    // MARK: - Content

    @ViewBuilder
    private func haContent(_ config: HAConfig) -> some View {
        // Connection status
        HStack(spacing: 12) {
            Image(systemName: config.connected ? "checkmark.circle.fill" : "xmark.circle.fill")
                .font(.system(size: 24))
                .foregroundColor(config.connected ? .green : .red)

            VStack(alignment: .leading, spacing: 3) {
                Text(config.connected ? "Connected" : "Disconnected")
                    .font(.system(size: 16, weight: .bold, design: .monospaced))
                    .foregroundColor(config.connected ? .green : .red)

                if !config.url.isEmpty {
                    Text(config.url)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundColor(.gray)
                        .lineLimit(1)
                }
            }

            Spacer()
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))

        // Sync button
        Button {
            syncEntities()
        } label: {
            HStack(spacing: 8) {
                Image(systemName: "arrow.triangle.2.circlepath")
                Text("Sync Entities")
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
            }
            .foregroundColor(.black)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 12)
            .background(RoundedRectangle(cornerRadius: 10).fill(Color.cyan))
        }
        .buttonStyle(.plain)
        .disabled(isSyncing)
        .opacity(isSyncing ? 0.5 : 1)

        // Entities list
        if !config.entities.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                Text("ENTITIES (\(config.entities.count))")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundColor(.gray)

                ForEach(config.entities, id: \.entityId) { entity in
                    HStack(spacing: 10) {
                        Image(systemName: entityIcon(entity.domain))
                            .font(.system(size: 14))
                            .foregroundColor(.cyan)
                            .frame(width: 24)

                        VStack(alignment: .leading, spacing: 2) {
                            Text(entity.name)
                                .font(.system(size: 13, weight: .semibold, design: .monospaced))
                                .foregroundColor(.white)
                            Text(entity.entityId)
                                .font(.system(size: 10, design: .monospaced))
                                .foregroundColor(.gray)
                        }

                        Spacer()

                        Text(entity.state)
                            .font(.system(size: 11, weight: .medium, design: .monospaced))
                            .foregroundColor(entity.state == "on" ? .green : .gray)
                    }
                    .padding(10)
                    .background(RoundedRectangle(cornerRadius: 8).fill(Color(white: 0.06)))
                }
            }
            .padding(16)
            .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
        }
    }

    private func entityIcon(_ domain: String) -> String {
        switch domain {
        case "light": return "lightbulb"
        case "switch": return "power"
        case "sensor": return "gauge"
        case "binary_sensor": return "circle.dotted"
        case "climate": return "thermometer"
        case "media_player": return "hifispeaker"
        default: return "square.grid.2x2"
        }
    }

    // MARK: - API

    private func fetchConfig() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/home-assistant") else { return }
        isLoading = true
        errorMessage = nil

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                isLoading = false
                if let error { errorMessage = error.localizedDescription; return }
                guard let data,
                      let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    errorMessage = "Failed to load Home Assistant config"
                    return
                }
                haConfig = try? JSONDecoder().decode(HAConfig.self, from: data)
            }
        }.resume()
    }

    private func syncEntities() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/home-assistant/sync") else { return }
        isSyncing = true

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                isSyncing = false
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else { return }
                statusMessage = "Entities synced"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                fetchConfig()
            }
        }.resume()
    }
}

// MARK: - Models

struct HAConfig: Codable {
    let connected: Bool
    let url: String
    let entities: [HAEntity]
}

struct HAEntity: Codable {
    let entityId: String
    let name: String
    let domain: String
    let state: String

    enum CodingKeys: String, CodingKey {
        case entityId = "entity_id"
        case name, domain, state
    }
}
