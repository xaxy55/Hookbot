import SwiftUI

struct ActivityFeedView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var activities: [ActivityEntry] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 8) {
                if isLoading && activities.isEmpty {
                    LoadingStateView(message: "Loading activity...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchActivity() }
                } else if activities.isEmpty {
                    EmptyStateView(icon: "list.bullet", message: "No activity yet")
                } else {
                    ForEach(activities) { entry in
                        activityRow(entry)
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Activity")
        .onAppear { fetchActivity() }
        .refreshable { fetchActivity() }
    }

    // MARK: - Row

    private func activityRow(_ entry: ActivityEntry) -> some View {
        HStack(spacing: 12) {
            Image(systemName: toolIcon(entry.toolName))
                .font(.system(size: 16))
                .foregroundColor(.cyan)
                .frame(width: 32, height: 32)
                .background(Circle().fill(Color.cyan.opacity(0.15)))

            VStack(alignment: .leading, spacing: 3) {
                Text(entry.toolName)
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                Text(entry.event)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundColor(.cyan)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Capsule().fill(Color.cyan.opacity(0.12)))
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 3) {
                Text("+\(entry.xpEarned) XP")
                    .font(.system(size: 12, weight: .bold, design: .monospaced))
                    .foregroundColor(.orange)

                Text(relativeTime(entry.createdAt))
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundColor(.gray)
            }
        }
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - Helpers

    private func toolIcon(_ tool: String) -> String {
        switch tool.lowercased() {
        case "read": return "eye"
        case "write": return "pencil"
        case "edit": return "pencil.line"
        case "bash": return "terminal"
        case "glob", "grep": return "magnifyingglass"
        case "agent": return "brain"
        case "websearch": return "globe"
        default: return "wrench"
        }
    }

    private func relativeTime(_ ts: String?) -> String {
        guard let ts, !ts.isEmpty else { return "" }
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        guard let date = formatter.date(from: ts) ?? ISO8601DateFormatter().date(from: ts) else {
            return String(ts.prefix(16))
        }
        let diff = Date().timeIntervalSince(date)
        if diff < 60 { return "just now" }
        if diff < 3600 { return "\(Int(diff / 60))m ago" }
        if diff < 86400 { return "\(Int(diff / 3600))h ago" }
        return "\(Int(diff / 86400))d ago"
    }

    // MARK: - API

    private func fetchActivity() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/gamification/activity?limit=50") else { return }
        isLoading = true
        errorMessage = nil

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                isLoading = false
                if let error {
                    errorMessage = error.localizedDescription
                    return
                }
                guard let data,
                      let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    errorMessage = "Failed to load activity"
                    return
                }
                if let decoded = try? JSONDecoder().decode([ActivityEntry].self, from: data) {
                    activities = decoded
                }
            }
        }.resume()
    }
}

// MARK: - Model

struct ActivityEntry: Codable, Identifiable {
    var id: String { "\(toolName)-\(createdAt ?? UUID().uuidString)" }
    let toolName: String
    let event: String
    let xpEarned: Int
    let createdAt: String?

    enum CodingKeys: String, CodingKey {
        case toolName = "tool_name"
        case event
        case xpEarned = "xp_earned"
        case createdAt = "created_at"
    }
}
