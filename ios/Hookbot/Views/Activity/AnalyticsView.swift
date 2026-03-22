import SwiftUI
import Charts

struct AnalyticsView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var stats: GamificationStats?
    @State private var chartData: [DailyXP] = []
    @State private var toolData: [ToolUsage] = []
    @State private var selectedDays = 30
    @State private var isLoading = false
    @State private var errorMessage: String?

    private let dayOptions = [7, 14, 30, 90]

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if isLoading && stats == nil {
                    LoadingStateView(message: "Loading analytics...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchAll() }
                } else {
                    if let stats {
                        statsCards(stats)
                    }

                    dayPicker

                    if !chartData.isEmpty {
                        xpChart
                    }

                    if !toolData.isEmpty {
                        toolChart
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Analytics")
        .onAppear { fetchAll() }
        .refreshable { fetchAll() }
    }

    // MARK: - Stats Cards

    private func statsCards(_ s: GamificationStats) -> some View {
        LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 10) {
            StatCardView(value: "\(s.totalXP)", label: "TOTAL XP", color: .orange, icon: "star.fill")
            StatCardView(value: "\(s.level)", label: "LEVEL", color: .cyan, icon: "arrow.up.circle")
            StatCardView(value: "\(s.toolUses)", label: "TOOL USES", color: .green, icon: "wrench")
            StatCardView(value: "\(s.streak)", label: "STREAK", color: .pink, icon: "flame")
        }
    }

    // MARK: - Day Picker

    private var dayPicker: some View {
        HStack(spacing: 8) {
            ForEach(dayOptions, id: \.self) { days in
                Button {
                    selectedDays = days
                    fetchAnalytics()
                } label: {
                    Text("\(days)d")
                        .font(.system(size: 12, weight: .semibold, design: .monospaced))
                        .foregroundColor(selectedDays == days ? .black : .white)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 7)
                        .background(
                            Capsule().fill(selectedDays == days ? Color.cyan : Color(white: 0.15))
                        )
                }
                .buttonStyle(.plain)
            }
            Spacer()
        }
    }

    // MARK: - XP Chart

    private var xpChart: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("XP OVER TIME")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            Chart(chartData) { item in
                BarMark(
                    x: .value("Day", item.date),
                    y: .value("XP", item.xp)
                )
                .foregroundStyle(Color.cyan.gradient)
                .cornerRadius(4)
            }
            .chartXAxis {
                AxisMarks(values: .automatic) { _ in
                    AxisGridLine(stroke: StrokeStyle(lineWidth: 0.5)).foregroundStyle(Color(white: 0.2))
                    AxisValueLabel().foregroundStyle(.gray).font(.system(size: 9, design: .monospaced))
                }
            }
            .chartYAxis {
                AxisMarks { _ in
                    AxisGridLine(stroke: StrokeStyle(lineWidth: 0.5)).foregroundStyle(Color(white: 0.2))
                    AxisValueLabel().foregroundStyle(.gray).font(.system(size: 9, design: .monospaced))
                }
            }
            .frame(height: 200)
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - Tool Chart

    private var toolChart: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("TOOL DISTRIBUTION")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            Chart(toolData) { item in
                BarMark(
                    x: .value("Uses", item.count),
                    y: .value("Tool", item.tool)
                )
                .foregroundStyle(Color.orange.gradient)
                .cornerRadius(4)
            }
            .chartXAxis {
                AxisMarks { _ in
                    AxisGridLine(stroke: StrokeStyle(lineWidth: 0.5)).foregroundStyle(Color(white: 0.2))
                    AxisValueLabel().foregroundStyle(.gray).font(.system(size: 9, design: .monospaced))
                }
            }
            .chartYAxis {
                AxisMarks { _ in
                    AxisValueLabel().foregroundStyle(.white).font(.system(size: 11, design: .monospaced))
                }
            }
            .frame(height: CGFloat(toolData.count * 32 + 20))
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - API

    private func fetchAll() {
        fetchStats()
        fetchAnalytics()
    }

    private func fetchStats() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/gamification/stats") else { return }
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
                    errorMessage = "Failed to load stats"
                    return
                }
                stats = try? JSONDecoder().decode(GamificationStats.self, from: data)
            }
        }.resume()
    }

    private func fetchAnalytics() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/gamification/analytics?days=\(selectedDays)") else { return }

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, _, _ in
            DispatchQueue.main.async {
                guard let data,
                      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }

                if let daily = json["daily_xp"] as? [[String: Any]] {
                    chartData = daily.compactMap { d in
                        guard let date = d["date"] as? String,
                              let xp = d["xp"] as? Int else { return nil }
                        return DailyXP(date: date, xp: xp)
                    }
                }

                if let tools = json["tool_distribution"] as? [[String: Any]] {
                    toolData = tools.compactMap { t in
                        guard let tool = t["tool"] as? String,
                              let count = t["count"] as? Int else { return nil }
                        return ToolUsage(tool: tool, count: count)
                    }
                }
            }
        }.resume()
    }
}

// MARK: - Models

struct GamificationStats: Codable {
    let totalXP: Int
    let level: Int
    let toolUses: Int
    let streak: Int

    enum CodingKeys: String, CodingKey {
        case totalXP = "total_xp"
        case level
        case toolUses = "tool_uses"
        case streak
    }
}

struct DailyXP: Identifiable {
    var id: String { date }
    let date: String
    let xp: Int
}

struct ToolUsage: Identifiable {
    var id: String { tool }
    let tool: String
    let count: Int
}
