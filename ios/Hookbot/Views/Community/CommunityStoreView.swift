import SwiftUI

struct CommunityStoreView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var plugins: [CommunityPlugin] = []
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var statusMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                if isLoading && plugins.isEmpty {
                    LoadingStateView(message: "Loading plugins...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchPlugins() }
                } else if plugins.isEmpty {
                    EmptyStateView(icon: "shippingbox", message: "No community plugins available")
                } else {
                    ForEach(plugins) { plugin in
                        pluginCard(plugin)
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
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Community Store")
        .onAppear { fetchPlugins() }
        .refreshable { fetchPlugins() }
    }

    // MARK: - Plugin Card

    private func pluginCard(_ plugin: CommunityPlugin) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Image(systemName: "puzzlepiece.extension")
                    .font(.system(size: 20))
                    .foregroundColor(.cyan)

                VStack(alignment: .leading, spacing: 2) {
                    Text(plugin.name)
                        .font(.system(size: 14, weight: .bold, design: .monospaced))
                        .foregroundColor(.white)

                    Text("by \(plugin.author)")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundColor(.gray)
                }

                Spacer()

                if plugin.installed {
                    Text("INSTALLED")
                        .font(.system(size: 10, weight: .bold, design: .monospaced))
                        .foregroundColor(.green)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Capsule().fill(Color.green.opacity(0.15)))
                }
            }

            if !plugin.description.isEmpty {
                Text(plugin.description)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundColor(.white.opacity(0.7))
                    .lineLimit(3)
            }

            HStack(spacing: 16) {
                // Rating
                HStack(spacing: 4) {
                    Image(systemName: "star.fill")
                        .font(.system(size: 11))
                        .foregroundColor(.orange)
                    Text(String(format: "%.1f", plugin.rating))
                        .font(.system(size: 12, weight: .bold, design: .monospaced))
                        .foregroundColor(.orange)
                }

                // Downloads
                HStack(spacing: 4) {
                    Image(systemName: "arrow.down.circle")
                        .font(.system(size: 11))
                        .foregroundColor(.gray)
                    Text("\(plugin.downloads)")
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundColor(.gray)
                }

                Spacer()

                if !plugin.installed {
                    Button {
                        installPlugin(plugin)
                    } label: {
                        Text("Install")
                            .font(.system(size: 12, weight: .bold, design: .monospaced))
                            .foregroundColor(.black)
                            .padding(.horizontal, 16)
                            .padding(.vertical, 6)
                            .background(Capsule().fill(Color.cyan))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - API

    private func fetchPlugins() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/community/plugins") else { return }
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
                    errorMessage = "Failed to load plugins"
                    return
                }
                plugins = (try? JSONDecoder().decode([CommunityPlugin].self, from: data)) ?? []
            }
        }.resume()
    }

    private func installPlugin(_ plugin: CommunityPlugin) {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/community/plugins/\(plugin.id)/install") else { return }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    statusMessage = "Failed to install \(plugin.name)"
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                    return
                }
                statusMessage = "\(plugin.name) installed"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                fetchPlugins()
            }
        }.resume()
    }
}

// MARK: - Model

struct CommunityPlugin: Codable, Identifiable {
    let id: String
    let name: String
    let author: String
    let description: String
    let rating: Double
    let downloads: Int
    let installed: Bool
}
