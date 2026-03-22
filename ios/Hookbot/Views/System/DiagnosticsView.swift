import SwiftUI

struct DiagnosticsView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var checks: [HealthCheck] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                if isLoading && checks.isEmpty {
                    LoadingStateView(message: "Running diagnostics...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchDiagnostics() }
                } else if checks.isEmpty {
                    EmptyStateView(icon: "stethoscope", message: "No diagnostics available")
                } else {
                    // Summary
                    HStack(spacing: 10) {
                        let passing = checks.filter { $0.status == "ok" || $0.status == "pass" }.count
                        StatCardView(value: "\(passing)/\(checks.count)", label: "PASSING", color: .green, icon: "checkmark.circle")
                        let failing = checks.filter { $0.status != "ok" && $0.status != "pass" }.count
                        StatCardView(value: "\(failing)", label: "ISSUES", color: failing > 0 ? .orange : .gray, icon: "exclamationmark.triangle")
                    }

                    ForEach(checks) { check in
                        checkRow(check)
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Diagnostics")
        .onAppear { fetchDiagnostics() }
        .refreshable { fetchDiagnostics() }
    }

    // MARK: - Row

    private func checkRow(_ check: HealthCheck) -> some View {
        HStack(spacing: 12) {
            Image(systemName: statusIcon(check.status))
                .font(.system(size: 16))
                .foregroundColor(statusColor(check.status))
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 3) {
                Text(check.name)
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                if let detail = check.detail, !detail.isEmpty {
                    Text(detail)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundColor(.gray)
                        .lineLimit(2)
                }
            }

            Spacer()

            Text(check.status.uppercased())
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(statusColor(check.status))
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Capsule().fill(statusColor(check.status).opacity(0.15)))
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func statusIcon(_ status: String) -> String {
        switch status.lowercased() {
        case "ok", "pass": return "checkmark.circle.fill"
        case "warn", "warning": return "exclamationmark.triangle.fill"
        case "fail", "error": return "xmark.circle.fill"
        default: return "questionmark.circle"
        }
    }

    private func statusColor(_ status: String) -> Color {
        switch status.lowercased() {
        case "ok", "pass": return .green
        case "warn", "warning": return .orange
        case "fail", "error": return .red
        default: return .gray
        }
    }

    // MARK: - API

    private func fetchDiagnostics() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/diagnostics") else { return }
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
                    errorMessage = "Failed to load diagnostics"
                    return
                }
                checks = (try? JSONDecoder().decode([HealthCheck].self, from: data)) ?? []
            }
        }.resume()
    }
}

// MARK: - Model

struct HealthCheck: Codable, Identifiable {
    var id: String { name }
    let name: String
    let status: String
    let detail: String?
}
