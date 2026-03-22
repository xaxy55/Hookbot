import SwiftUI

struct LogsView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var logStats: LogStats?
    @State private var isLoading = false
    @State private var isPruning = false
    @State private var errorMessage: String?
    @State private var statusMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if isLoading && logStats == nil {
                    LoadingStateView(message: "Loading log stats...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchStats() }
                } else if let stats = logStats {
                    logsContent(stats)
                } else {
                    EmptyStateView(icon: "doc.text", message: "No log data available")
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
        .navigationTitle("Logs")
        .onAppear { fetchStats() }
        .refreshable { fetchStats() }
    }

    // MARK: - Content

    @ViewBuilder
    private func logsContent(_ stats: LogStats) -> some View {
        // Stats cards
        LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 10) {
            StatCardView(value: "\(stats.totalEntries)", label: "TOTAL ENTRIES", color: .cyan, icon: "doc.text")
            StatCardView(value: formatBytes(stats.totalSize), label: "TOTAL SIZE", color: .orange, icon: "externaldrive")
            StatCardView(value: "\(stats.errorCount)", label: "ERRORS", color: stats.errorCount > 0 ? .red : .green, icon: "xmark.circle")
            StatCardView(value: "\(stats.warnCount)", label: "WARNINGS", color: stats.warnCount > 0 ? .orange : .green, icon: "exclamationmark.triangle")
        }

        // Breakdown
        if !stats.breakdown.isEmpty {
            VStack(alignment: .leading, spacing: 10) {
                Text("BREAKDOWN BY SOURCE")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundColor(.gray)

                ForEach(stats.breakdown, id: \.source) { item in
                    HStack {
                        Text(item.source)
                            .font(.system(size: 13, design: .monospaced))
                            .foregroundColor(.white)
                        Spacer()
                        Text("\(item.count)")
                            .font(.system(size: 13, weight: .bold, design: .monospaced))
                            .foregroundColor(.cyan)
                    }
                    .padding(.vertical, 4)
                }
            }
            .padding(16)
            .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
        }

        // Prune button
        Button {
            pruneLogs()
        } label: {
            HStack(spacing: 8) {
                Image(systemName: "trash")
                Text("Prune Old Logs")
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
            }
            .foregroundColor(.orange)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 12)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(Color.orange, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .disabled(isPruning)
        .opacity(isPruning ? 0.5 : 1)
    }

    private func formatBytes(_ bytes: Int) -> String {
        if bytes < 1024 { return "\(bytes)B" }
        if bytes < 1024 * 1024 { return "\(bytes / 1024)KB" }
        return String(format: "%.1fMB", Double(bytes) / (1024.0 * 1024.0))
    }

    // MARK: - API

    private func fetchStats() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/logs/stats") else { return }
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
                    errorMessage = "Failed to load log stats"
                    return
                }
                logStats = try? JSONDecoder().decode(LogStats.self, from: data)
            }
        }.resume()
    }

    private func pruneLogs() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/logs/prune") else { return }
        isPruning = true

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                isPruning = false
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    statusMessage = "Failed to prune logs"
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                    return
                }
                statusMessage = "Logs pruned"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                fetchStats()
            }
        }.resume()
    }
}

// MARK: - Models

struct LogStats: Codable {
    let totalEntries: Int
    let totalSize: Int
    let errorCount: Int
    let warnCount: Int
    let breakdown: [LogBreakdown]

    enum CodingKeys: String, CodingKey {
        case totalEntries = "total_entries"
        case totalSize = "total_size"
        case errorCount = "error_count"
        case warnCount = "warn_count"
        case breakdown
    }
}

struct LogBreakdown: Codable {
    let source: String
    let count: Int
}
