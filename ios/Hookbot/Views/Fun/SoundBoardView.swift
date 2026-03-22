import SwiftUI

struct SoundBoardView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var playingSound: String?
    @State private var statusMessage: String?

    private let sounds: [(name: String, emoji: String, state: String)] = [
        ("Level Up", "star.fill", "levelup"),
        ("Achievement", "trophy.fill", "achievement"),
        ("Error", "xmark.octagon", "error"),
        ("Notification", "bell.fill", "notification"),
        ("Victory", "crown.fill", "victory"),
        ("Boot Up", "power", "boot"),
        ("Deploy", "paperplane.fill", "deploy"),
        ("Commit", "checkmark.circle", "commit"),
        ("Break Time", "cup.and.saucer.fill", "break"),
        ("Focus", "brain", "focus"),
        ("Alert", "exclamationmark.triangle", "alert"),
        ("Party", "party.popper.fill", "party"),
    ]

    private let columns = [
        GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())
    ]

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                LazyVGrid(columns: columns, spacing: 12) {
                    ForEach(sounds, id: \.state) { sound in
                        soundButton(sound)
                    }
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
        .navigationTitle("Sound Board")
    }

    // MARK: - Button

    private func soundButton(_ sound: (name: String, emoji: String, state: String)) -> some View {
        Button {
            triggerSound(sound.state, name: sound.name)
        } label: {
            VStack(spacing: 8) {
                Image(systemName: sound.emoji)
                    .font(.system(size: 24))
                    .foregroundColor(playingSound == sound.state ? .green : .cyan)

                Text(sound.name)
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(.white)
                    .lineLimit(1)
            }
            .frame(maxWidth: .infinity)
            .frame(height: 80)
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(playingSound == sound.state ? Color.cyan.opacity(0.15) : Color(white: 0.08))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .stroke(playingSound == sound.state ? Color.cyan.opacity(0.4) : Color.clear, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .disabled(playingSound != nil)
    }

    // MARK: - API

    private func triggerSound(_ state: String, name: String) {
        guard !engine.config.serverURL.isEmpty, !engine.config.deviceId.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/devices/\(engine.config.deviceId)/state") else { return }
        playingSound = state

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }
        request.httpBody = try? JSONSerialization.data(withJSONObject: ["state": state])

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                playingSound = nil
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    statusMessage = "Failed to play \(name)"
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                    return
                }
                statusMessage = "Playing \(name)"
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
            }
        }.resume()
    }
}
