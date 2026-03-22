import SwiftUI

struct IntegrationsView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var integrations: [IntegrationStatus] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    private let defaultIntegrations = [
        ("Slack", "message.fill", "slack"),
        ("Discord", "bubble.left.fill", "discord"),
        ("GitHub", "chevron.left.forwardslash.chevron.right", "github"),
    ]

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                if isLoading && integrations.isEmpty {
                    LoadingStateView(message: "Loading integrations...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchIntegrations() }
                } else if integrations.isEmpty {
                    // Show defaults
                    ForEach(defaultIntegrations, id: \.2) { name, icon, key in
                        integrationRow(name: name, icon: icon, connected: false, webhookURL: nil)
                    }
                } else {
                    ForEach(integrations) { integration in
                        integrationRow(
                            name: integration.name,
                            icon: iconFor(integration.name),
                            connected: integration.connected,
                            webhookURL: integration.webhookURL
                        )
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Integrations")
        .onAppear { fetchIntegrations() }
        .refreshable { fetchIntegrations() }
    }

    // MARK: - Row

    private func integrationRow(name: String, icon: String, connected: Bool, webhookURL: String?) -> some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 18))
                .foregroundColor(connected ? .cyan : .gray)
                .frame(width: 36, height: 36)
                .background(Circle().fill(connected ? Color.cyan.opacity(0.15) : Color(white: 0.12)))

            VStack(alignment: .leading, spacing: 3) {
                Text(name)
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                if let url = webhookURL, !url.isEmpty {
                    Text(url.prefix(40) + (url.count > 40 ? "..." : ""))
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundColor(.gray)
                        .lineLimit(1)
                }
            }

            Spacer()

            HStack(spacing: 6) {
                Circle()
                    .fill(connected ? Color.green : Color.gray)
                    .frame(width: 8, height: 8)
                Text(connected ? "Active" : "Inactive")
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(connected ? .green : .gray)
            }
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func iconFor(_ name: String) -> String {
        switch name.lowercased() {
        case "slack": return "message.fill"
        case "discord": return "bubble.left.fill"
        case "github": return "chevron.left.forwardslash.chevron.right"
        default: return "link"
        }
    }

    // MARK: - API

    private func fetchIntegrations() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/integrations") else { return }
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
                    errorMessage = "Failed to load integrations"
                    return
                }
                integrations = (try? JSONDecoder().decode([IntegrationStatus].self, from: data)) ?? []
            }
        }.resume()
    }
}

// MARK: - Model

struct IntegrationStatus: Codable, Identifiable {
    var id: String { name }
    let name: String
    let connected: Bool
    let webhookURL: String?

    enum CodingKeys: String, CodingKey {
        case name, connected
        case webhookURL = "webhook_url"
    }
}
