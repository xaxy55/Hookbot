import SwiftUI

struct StandingDeskView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var deskState: DeskState?
    @State private var isLoading = false
    @State private var isToggling = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if isLoading && deskState == nil {
                    LoadingStateView(message: "Loading desk status...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchDesk() }
                } else if let desk = deskState {
                    deskContent(desk)
                } else {
                    EmptyStateView(icon: "desktopcomputer", message: "No desk data available")
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Standing Desk")
        .onAppear { fetchDesk() }
        .refreshable { fetchDesk() }
    }

    // MARK: - Content

    @ViewBuilder
    private func deskContent(_ desk: DeskState) -> some View {
        // Current position
        VStack(spacing: 16) {
            Image(systemName: desk.isStanding ? "figure.stand" : "figure.seated.side")
                .font(.system(size: 48))
                .foregroundColor(desk.isStanding ? .green : .cyan)

            Text(desk.isStanding ? "STANDING" : "SITTING")
                .font(.system(size: 22, weight: .bold, design: .monospaced))
                .foregroundColor(desk.isStanding ? .green : .cyan)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 24)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))

        // Toggle button
        Button {
            toggleDesk()
        } label: {
            HStack(spacing: 10) {
                Image(systemName: desk.isStanding ? "arrow.down" : "arrow.up")
                Text(desk.isStanding ? "Sit Down" : "Stand Up")
                    .font(.system(size: 15, weight: .bold, design: .monospaced))
            }
            .foregroundColor(.black)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 14)
            .background(RoundedRectangle(cornerRadius: 12).fill(desk.isStanding ? Color.cyan : Color.green))
        }
        .buttonStyle(.plain)
        .disabled(isToggling)
        .opacity(isToggling ? 0.5 : 1)

        // Daily progress
        VStack(alignment: .leading, spacing: 10) {
            Text("DAILY STANDING TIME")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            HStack {
                Text(formatMinutes(desk.standingMinutesToday))
                    .font(.system(size: 20, weight: .bold, design: .monospaced))
                    .foregroundColor(.green)

                Text("/ \(formatMinutes(desk.goalMinutes))")
                    .font(.system(size: 14, design: .monospaced))
                    .foregroundColor(.gray)

                Spacer()

                Text("\(min(100, desk.goalMinutes > 0 ? (desk.standingMinutesToday * 100 / desk.goalMinutes) : 0))%")
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                    .foregroundColor(.green)
            }

            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 4)
                        .fill(Color(white: 0.15))
                    RoundedRectangle(cornerRadius: 4)
                        .fill(Color.green)
                        .frame(width: geo.size.width * min(1.0, CGFloat(desk.standingMinutesToday) / max(1, CGFloat(desk.goalMinutes))))
                }
            }
            .frame(height: 8)
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func formatMinutes(_ mins: Int) -> String {
        let h = mins / 60
        let m = mins % 60
        if h > 0 { return "\(h)h \(m)m" }
        return "\(m)m"
    }

    // MARK: - API

    private func fetchDesk() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/desk") else { return }
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
                    errorMessage = "Failed to load desk state"
                    return
                }
                deskState = try? JSONDecoder().decode(DeskState.self, from: data)
            }
        }.resume()
    }

    private func toggleDesk() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/desk/toggle") else { return }
        isToggling = true

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                isToggling = false
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else { return }
                fetchDesk()
            }
        }.resume()
    }
}

// MARK: - Model

struct DeskState: Codable {
    let isStanding: Bool
    let standingMinutesToday: Int
    let goalMinutes: Int

    enum CodingKeys: String, CodingKey {
        case isStanding = "is_standing"
        case standingMinutesToday = "standing_minutes_today"
        case goalMinutes = "goal_minutes"
    }
}
