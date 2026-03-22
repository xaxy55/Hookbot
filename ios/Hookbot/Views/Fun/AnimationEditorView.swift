import SwiftUI

struct AnimationEditorView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var keyframes: [AnimKeyframe] = [
        AnimKeyframe(color: "#00FFFF", brightness: 100, durationMs: 500),
        AnimKeyframe(color: "#FF6600", brightness: 80, durationMs: 500),
    ]
    @State private var animationName = ""
    @State private var isPlaying = false
    @State private var statusMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                // Name field
                VStack(alignment: .leading, spacing: 6) {
                    Text("ANIMATION NAME")
                        .font(.system(size: 10, weight: .bold, design: .monospaced))
                        .foregroundColor(.gray)
                    TextField("My Animation", text: $animationName)
                        .font(.system(size: 14, design: .monospaced))
                        .foregroundColor(.white)
                        .padding(10)
                        .background(RoundedRectangle(cornerRadius: 8).fill(Color(white: 0.12)))
                }

                // Keyframes
                VStack(alignment: .leading, spacing: 10) {
                    HStack {
                        Text("KEYFRAMES")
                            .font(.system(size: 11, weight: .bold, design: .monospaced))
                            .foregroundColor(.gray)
                        Spacer()
                        Button {
                            keyframes.append(AnimKeyframe(color: "#FFFFFF", brightness: 100, durationMs: 500))
                        } label: {
                            Image(systemName: "plus.circle.fill")
                                .foregroundColor(.cyan)
                        }
                    }

                    ForEach(keyframes.indices, id: \.self) { index in
                        keyframeRow(index: index)
                    }
                }
                .padding(16)
                .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))

                // Preview bar
                HStack(spacing: 2) {
                    ForEach(keyframes.indices, id: \.self) { i in
                        let kf = keyframes[i]
                        RoundedRectangle(cornerRadius: 4)
                            .fill(colorFromHex(kf.color).opacity(Double(kf.brightness) / 100.0))
                            .frame(height: 20)
                    }
                }
                .padding(12)
                .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))

                // Action buttons
                HStack(spacing: 10) {
                    Button {
                        playAnimation()
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "play.fill")
                            Text("Preview")
                                .font(.system(size: 13, weight: .bold, design: .monospaced))
                        }
                        .foregroundColor(.black)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 12)
                        .background(RoundedRectangle(cornerRadius: 10).fill(Color.cyan))
                    }
                    .buttonStyle(.plain)
                    .disabled(isPlaying)

                    Button {
                        saveAnimation()
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "square.and.arrow.down")
                            Text("Save")
                                .font(.system(size: 13, weight: .bold, design: .monospaced))
                        }
                        .foregroundColor(.cyan)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 12)
                        .background(
                            RoundedRectangle(cornerRadius: 10)
                                .stroke(Color.cyan, lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
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
        .navigationTitle("Animation Editor")
    }

    // MARK: - Keyframe Row

    private func keyframeRow(index: Int) -> some View {
        HStack(spacing: 10) {
            Text("\(index + 1)")
                .font(.system(size: 12, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)
                .frame(width: 20)

            Circle()
                .fill(colorFromHex(keyframes[index].color))
                .frame(width: 20, height: 20)

            TextField("#HEX", text: $keyframes[index].color)
                .font(.system(size: 12, design: .monospaced))
                .foregroundColor(.white)
                .padding(6)
                .frame(width: 80)
                .background(RoundedRectangle(cornerRadius: 6).fill(Color(white: 0.12)))

            Text("\(keyframes[index].brightness)%")
                .font(.system(size: 11, design: .monospaced))
                .foregroundColor(.white)
                .frame(width: 40)

            Slider(value: Binding(
                get: { Double(keyframes[index].brightness) },
                set: { keyframes[index].brightness = Int($0) }
            ), in: 0...100)
            .tint(.cyan)

            Text("\(keyframes[index].durationMs)ms")
                .font(.system(size: 10, design: .monospaced))
                .foregroundColor(.gray)
                .frame(width: 48)

            if keyframes.count > 1 {
                Button {
                    keyframes.remove(at: index)
                } label: {
                    Image(systemName: "minus.circle")
                        .foregroundColor(.red.opacity(0.7))
                }
            }
        }
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

    private func playAnimation() {
        guard !engine.config.serverURL.isEmpty, !engine.config.deviceId.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/devices/\(engine.config.deviceId)/animation") else { return }
        isPlaying = true

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        let body: [String: Any] = [
            "animation": "custom",
            "keyframes": keyframes.map { ["color": $0.color, "brightness": $0.brightness, "duration_ms": $0.durationMs] }
        ]
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                isPlaying = false
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    statusMessage = "Failed to preview"
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                    return
                }
                statusMessage = "Playing preview"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
            }
        }.resume()
    }

    private func saveAnimation() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/animations") else { return }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        let body: [String: Any] = [
            "name": animationName.isEmpty ? "Untitled" : animationName,
            "keyframes": keyframes.map { ["color": $0.color, "brightness": $0.brightness, "duration_ms": $0.durationMs] }
        ]
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 || http.statusCode == 201 else {
                    statusMessage = "Failed to save"
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                    return
                }
                statusMessage = "Animation saved"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
            }
        }.resume()
    }
}

// MARK: - Model

struct AnimKeyframe: Identifiable {
    let id = UUID()
    var color: String
    var brightness: Int
    var durationMs: Int
}
