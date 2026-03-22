import SwiftUI

struct LightShowView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var activePreset: String?
    @State private var statusMessage: String?

    private let presets: [(name: String, icon: String, animation: String, color: Color)] = [
        ("Rainbow", "rainbow", "rainbow", .purple),
        ("Pulse", "waveform", "pulse", .cyan),
        ("Chase", "arrow.right.circle", "chase", .green),
        ("Sparkle", "sparkles", "sparkle", .yellow),
        ("Breathe", "wind", "breathe", .blue),
        ("Fire", "flame", "fire", .orange),
        ("Ocean", "water.waves", "ocean", .teal),
        ("Matrix", "rectangle.grid.1x2", "matrix", .green),
        ("Strobe", "bolt.fill", "strobe", .white),
        ("Aurora", "aqi.medium", "aurora", .cyan),
        ("Sunset", "sun.horizon", "sunset", .orange),
        ("Party", "party.popper", "party", .pink),
    ]

    private let columns = [
        GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())
    ]

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                Text("LED ANIMATION PRESETS")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundColor(.gray)
                    .frame(maxWidth: .infinity, alignment: .leading)

                LazyVGrid(columns: columns, spacing: 12) {
                    ForEach(presets, id: \.animation) { preset in
                        presetButton(preset)
                    }
                }

                // Stop button
                Button {
                    triggerAnimation("off")
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "stop.fill")
                        Text("Stop Animation")
                            .font(.system(size: 14, weight: .bold, design: .monospaced))
                    }
                    .foregroundColor(.white)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 12)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.15)))
                }
                .buttonStyle(.plain)

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
        .navigationTitle("Light Show")
    }

    // MARK: - Preset Button

    private func presetButton(_ preset: (name: String, icon: String, animation: String, color: Color)) -> some View {
        Button {
            triggerAnimation(preset.animation)
        } label: {
            VStack(spacing: 8) {
                Image(systemName: preset.icon)
                    .font(.system(size: 22))
                    .foregroundColor(activePreset == preset.animation ? .white : preset.color)

                Text(preset.name)
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .frame(height: 80)
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(activePreset == preset.animation ? preset.color.opacity(0.3) : Color(white: 0.08))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .stroke(activePreset == preset.animation ? preset.color : Color.clear, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }

    // MARK: - API

    private func triggerAnimation(_ animation: String) {
        guard !engine.config.serverURL.isEmpty, !engine.config.deviceId.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/devices/\(engine.config.deviceId)/animation") else { return }

        activePreset = animation == "off" ? nil : animation

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }
        request.httpBody = try? JSONSerialization.data(withJSONObject: ["animation": animation])

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    statusMessage = "Failed to trigger animation"
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                    return
                }
                statusMessage = animation == "off" ? "Animation stopped" : "Playing \(animation)"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
            }
        }.resume()
    }
}
