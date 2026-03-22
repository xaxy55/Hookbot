import SwiftUI

struct GlobalWallView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var events: [GlobalEvent] = []
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var newMessage = ""
    @State private var isPosting = false

    var body: some View {
        VStack(spacing: 0) {
            // Messages list
            ScrollView {
                VStack(spacing: 8) {
                    if isLoading && events.isEmpty {
                        LoadingStateView(message: "Loading wall...")
                    } else if let errorMessage {
                        ErrorStateView(message: errorMessage) { fetchEvents() }
                    } else if events.isEmpty {
                        EmptyStateView(icon: "bubble.left.and.bubble.right", message: "No messages yet\nBe the first to post!")
                    } else {
                        ForEach(events) { event in
                            eventRow(event)
                        }
                    }
                }
                .padding()
            }
            .background(Color.black)

            // Compose bar
            HStack(spacing: 10) {
                TextField("Write something...", text: $newMessage)
                    .font(.system(size: 13, design: .monospaced))
                    .foregroundColor(.white)
                    .padding(10)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.12)))

                Button {
                    postMessage()
                } label: {
                    Image(systemName: "arrow.up.circle.fill")
                        .font(.system(size: 28))
                        .foregroundColor(newMessage.isEmpty ? .gray : .cyan)
                }
                .disabled(newMessage.isEmpty || isPosting)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(Color(white: 0.06))
        }
        .navigationTitle("Global Wall")
        .onAppear { fetchEvents() }
        .refreshable { fetchEvents() }
    }

    // MARK: - Row

    private func eventRow(_ event: GlobalEvent) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Image(systemName: "person.circle.fill")
                    .font(.system(size: 18))
                    .foregroundColor(.cyan)

                Text(event.author)
                    .font(.system(size: 12, weight: .semibold, design: .monospaced))
                    .foregroundColor(.cyan)

                Spacer()

                if let ts = event.createdAt {
                    Text(relativeTime(ts))
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundColor(.gray)
                }
            }

            Text(event.message)
                .font(.system(size: 13, design: .monospaced))
                .foregroundColor(.white)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func relativeTime(_ ts: String) -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        guard let date = formatter.date(from: ts) ?? ISO8601DateFormatter().date(from: ts) else {
            return String(ts.prefix(16))
        }
        let diff = Date().timeIntervalSince(date)
        if diff < 60 { return "now" }
        if diff < 3600 { return "\(Int(diff / 60))m" }
        if diff < 86400 { return "\(Int(diff / 3600))h" }
        return "\(Int(diff / 86400))d"
    }

    // MARK: - API

    private func fetchEvents() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/social/global-events") else { return }
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
                    errorMessage = "Failed to load wall"
                    return
                }
                events = (try? JSONDecoder().decode([GlobalEvent].self, from: data)) ?? []
            }
        }.resume()
    }

    private func postMessage() {
        guard !engine.config.serverURL.isEmpty, !newMessage.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/social/global-events") else { return }
        isPosting = true

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        let body: [String: Any] = [
            "message": newMessage,
            "device_id": engine.config.deviceId
        ]
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                isPosting = false
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 || http.statusCode == 201 else { return }
                newMessage = ""
                fetchEvents()
            }
        }.resume()
    }
}

// MARK: - Model

struct GlobalEvent: Codable, Identifiable {
    var id: String { "\(author)-\(createdAt ?? UUID().uuidString)" }
    let author: String
    let message: String
    let createdAt: String?

    enum CodingKeys: String, CodingKey {
        case author, message
        case createdAt = "created_at"
    }
}
