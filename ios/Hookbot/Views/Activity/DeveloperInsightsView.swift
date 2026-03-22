import SwiftUI

struct DeveloperInsightsView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var digest: WeeklyDigest?
    @State private var burnout: BurnoutCheck?
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if isLoading && digest == nil {
                    LoadingStateView(message: "Loading insights...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchAll() }
                } else {
                    if let digest {
                        weeklyCard(digest)
                    }
                    if let burnout {
                        burnoutCard(burnout)
                    }
                    if digest == nil && burnout == nil {
                        EmptyStateView(icon: "brain.head.profile", message: "No insights available yet")
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Developer Insights")
        .onAppear { fetchAll() }
        .refreshable { fetchAll() }
    }

    // MARK: - Weekly Card

    private func weeklyCard(_ d: WeeklyDigest) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 8) {
                Image(systemName: "calendar.badge.clock")
                    .foregroundColor(.cyan)
                Text("WEEKLY SUMMARY")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundColor(.gray)
            }

            HStack(spacing: 10) {
                StatCardView(value: "\(d.totalXP)", label: "XP EARNED", color: .orange)
                StatCardView(value: "\(d.sessionsCount)", label: "SESSIONS", color: .cyan)
            }

            HStack(spacing: 10) {
                StatCardView(value: "\(d.toolUses)", label: "TOOL USES", color: .green)
                StatCardView(value: d.topTool, label: "TOP TOOL", color: .white)
            }

            if !d.summary.isEmpty {
                Text(d.summary)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundColor(.white.opacity(0.8))
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(RoundedRectangle(cornerRadius: 8).fill(Color(white: 0.12)))
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - Burnout Card

    private func burnoutCard(_ b: BurnoutCheck) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 8) {
                Image(systemName: "heart.text.square")
                    .foregroundColor(burnoutColor(b.riskLevel))
                Text("BURNOUT CHECK")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundColor(.gray)
            }

            HStack(spacing: 10) {
                VStack(spacing: 4) {
                    Text(b.riskLevel.uppercased())
                        .font(.system(size: 18, weight: .bold, design: .monospaced))
                        .foregroundColor(burnoutColor(b.riskLevel))
                    Text("RISK LEVEL")
                        .font(.system(size: 10, weight: .medium, design: .monospaced))
                        .foregroundColor(.gray)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)
                .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))

                StatCardView(value: "\(b.flowScore)%", label: "FLOW STATE", color: .cyan)
            }

            if !b.recommendation.isEmpty {
                Text(b.recommendation)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundColor(.white.opacity(0.8))
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(RoundedRectangle(cornerRadius: 8).fill(Color(white: 0.12)))
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func burnoutColor(_ level: String) -> Color {
        switch level.lowercased() {
        case "low": return .green
        case "medium": return .orange
        case "high": return .red
        default: return .gray
        }
    }

    // MARK: - API

    private func fetchAll() {
        fetchDigest()
        fetchBurnout()
    }

    private func fetchDigest() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/gamification/weekly-digest") else { return }
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
                      let http = response as? HTTPURLResponse, http.statusCode == 200 else { return }
                digest = try? JSONDecoder().decode(WeeklyDigest.self, from: data)
            }
        }.resume()
    }

    private func fetchBurnout() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/gamification/burnout-check") else { return }

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, _, _ in
            DispatchQueue.main.async {
                guard let data else { return }
                burnout = try? JSONDecoder().decode(BurnoutCheck.self, from: data)
            }
        }.resume()
    }
}

// MARK: - Models

struct WeeklyDigest: Codable {
    let totalXP: Int
    let sessionsCount: Int
    let toolUses: Int
    let topTool: String
    let summary: String

    enum CodingKeys: String, CodingKey {
        case totalXP = "total_xp"
        case sessionsCount = "sessions_count"
        case toolUses = "tool_uses"
        case topTool = "top_tool"
        case summary
    }
}

struct BurnoutCheck: Codable {
    let riskLevel: String
    let flowScore: Int
    let recommendation: String

    enum CodingKeys: String, CodingKey {
        case riskLevel = "risk_level"
        case flowScore = "flow_score"
        case recommendation
    }
}
