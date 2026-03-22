import SwiftUI

struct DeskLightsView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var lights: [LightZone] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                if isLoading && lights.isEmpty {
                    LoadingStateView(message: "Loading lights...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchLights() }
                } else if lights.isEmpty {
                    EmptyStateView(icon: "lightbulb", message: "No lights configured")
                } else {
                    ForEach(lights) { light in
                        lightRow(light)
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Desk Lights")
        .onAppear { fetchLights() }
        .refreshable { fetchLights() }
    }

    // MARK: - Row

    private func lightRow(_ light: LightZone) -> some View {
        HStack(spacing: 12) {
            Circle()
                .fill(colorFromHex(light.color))
                .frame(width: 24, height: 24)
                .overlay(Circle().stroke(Color.white.opacity(0.2), lineWidth: 1))

            VStack(alignment: .leading, spacing: 3) {
                Text(light.name)
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                Text("Zone \(light.zone)")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundColor(.gray)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 3) {
                Text("\(light.brightness)%")
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                    .foregroundColor(.cyan)

                GeometryReader { geo in
                    ZStack(alignment: .leading) {
                        RoundedRectangle(cornerRadius: 2)
                            .fill(Color(white: 0.2))
                        RoundedRectangle(cornerRadius: 2)
                            .fill(colorFromHex(light.color))
                            .frame(width: geo.size.width * CGFloat(light.brightness) / 100.0)
                    }
                }
                .frame(width: 60, height: 4)
            }
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func colorFromHex(_ hex: String) -> Color {
        var h = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        if h.hasPrefix("#") { h.removeFirst() }
        guard h.count == 6, let val = UInt64(h, radix: 16) else { return .white }
        return Color(
            red: Double((val >> 16) & 0xFF) / 255,
            green: Double((val >> 8) & 0xFF) / 255,
            blue: Double(val & 0xFF) / 255
        )
    }

    // MARK: - API

    private func fetchLights() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/desk-lights") else { return }
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
                    errorMessage = "Failed to load lights"
                    return
                }
                lights = (try? JSONDecoder().decode([LightZone].self, from: data)) ?? []
            }
        }.resume()
    }
}

// MARK: - Model

struct LightZone: Codable, Identifiable {
    var id: String { name }
    let name: String
    let zone: Int
    let color: String
    let brightness: Int
}
