import SwiftUI

struct StreamDeckView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var buttons: [StreamDeckButton] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    private let columns = [
        GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())
    ]

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if isLoading && buttons.isEmpty {
                    LoadingStateView(message: "Loading Stream Deck...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchButtons() }
                } else if buttons.isEmpty {
                    EmptyStateView(icon: "keyboard", message: "No buttons configured")
                } else {
                    LazyVGrid(columns: columns, spacing: 12) {
                        ForEach(buttons) { button in
                            buttonCell(button)
                        }
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Stream Deck")
        .onAppear { fetchButtons() }
        .refreshable { fetchButtons() }
    }

    // MARK: - Button Cell

    private func buttonCell(_ button: StreamDeckButton) -> some View {
        VStack(spacing: 6) {
            Image(systemName: button.icon ?? "square")
                .font(.system(size: 22))
                .foregroundColor(.cyan)

            Text(button.label)
                .font(.system(size: 10, weight: .medium, design: .monospaced))
                .foregroundColor(.white)
                .lineLimit(2)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .frame(height: 80)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(Color(white: 0.15), lineWidth: 1)
        )
    }

    // MARK: - API

    private func fetchButtons() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/stream-deck") else { return }
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
                    errorMessage = "Failed to load Stream Deck config"
                    return
                }
                buttons = (try? JSONDecoder().decode([StreamDeckButton].self, from: data)) ?? []
            }
        }.resume()
    }
}

// MARK: - Model

struct StreamDeckButton: Codable, Identifiable {
    var id: String { "\(position)-\(label)" }
    let position: Int
    let label: String
    let action: String
    let icon: String?
}
