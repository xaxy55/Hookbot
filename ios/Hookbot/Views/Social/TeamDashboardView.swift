import SwiftUI

struct TeamDashboardView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var members: [TeamMember] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                if isLoading && members.isEmpty {
                    LoadingStateView(message: "Loading team...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchTeam() }
                } else if members.isEmpty {
                    EmptyStateView(icon: "person.3", message: "No team members found")
                } else {
                    ForEach(members) { member in
                        memberRow(member)
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Team Dashboard")
        .onAppear { fetchTeam() }
        .refreshable { fetchTeam() }
    }

    // MARK: - Row

    private func memberRow(_ member: TeamMember) -> some View {
        HStack(spacing: 12) {
            ZStack(alignment: .bottomTrailing) {
                Image(systemName: "person.circle.fill")
                    .font(.system(size: 32))
                    .foregroundColor(.cyan)

                Circle()
                    .fill(member.isOnline ? Color.green : Color.gray)
                    .frame(width: 10, height: 10)
                    .overlay(Circle().stroke(Color.black, lineWidth: 2))
            }

            VStack(alignment: .leading, spacing: 3) {
                Text(member.name)
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                Text(member.status)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundColor(.cyan)
            }

            Spacer()

            if let state = member.currentState {
                Text(state.capitalized)
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(.white)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Capsule().fill(Color(white: 0.15)))
            }
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - API

    private func fetchTeam() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/social/team") else { return }
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
                    errorMessage = "Failed to load team"
                    return
                }
                members = (try? JSONDecoder().decode([TeamMember].self, from: data)) ?? []
            }
        }.resume()
    }
}

// MARK: - Model

struct TeamMember: Codable, Identifiable {
    var id: String { name }
    let name: String
    let status: String
    let isOnline: Bool
    let currentState: String?

    enum CodingKeys: String, CodingKey {
        case name, status
        case isOnline = "is_online"
        case currentState = "current_state"
    }
}
