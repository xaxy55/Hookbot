import SwiftUI

// Real-time emoji reactions to a friend's avatar (Phase 13.4)

struct AvatarEmotesView: View {
    let targetFriend: Friend
    @EnvironmentObject var engine: AvatarEngine
    @StateObject private var social = SocialService.shared
    @Environment(\.dismiss) var dismiss
    @State private var recentlySent: AvatarEmote?
    @State private var showConfirmation = false

    var body: some View {
        NavigationStack {
            VStack(spacing: 24) {
                Text("Send an emote to")
                    .font(.system(size: 13, design: .monospaced))
                    .foregroundStyle(.gray)
                Text(targetFriend.username)
                    .font(.system(size: 24, weight: .black, design: .monospaced))
                    .foregroundStyle(.white)

                LazyVGrid(columns: Array(repeating: .init(.flexible()), count: 3), spacing: 16) {
                    ForEach(AvatarEmote.allCases, id: \.self) { emote in
                        EmoteButton(emote: emote, isRecent: recentlySent == emote) {
                            sendEmote(emote)
                        }
                    }
                }
                .padding()

                if showConfirmation, let sent = recentlySent {
                    HStack {
                        Text(sent.emoji)
                        Text("Sent \(sent.displayName) to \(targetFriend.username)!")
                            .font(.system(size: 13, design: .monospaced))
                            .foregroundStyle(.green)
                    }
                    .padding(12)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Color.green.opacity(0.1)))
                }

                Spacer()
            }
            .padding()
            .background(Color.black)
            .navigationTitle("Emotes")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .preferredColorScheme(.dark)
    }

    private func sendEmote(_ emote: AvatarEmote) {
        recentlySent = emote
        HapticManager.playEvent(.taskComplete)

        Task {
            let _ = await social.sendEmote(emote, to: targetFriend.id)
            await MainActor.run {
                withAnimation { showConfirmation = true }
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                    withAnimation { showConfirmation = false }
                }
            }
        }
    }
}

struct EmoteButton: View {
    let emote: AvatarEmote
    let isRecent: Bool
    let action: () -> Void
    @State private var isPressed = false

    var body: some View {
        Button {
            withAnimation(.spring(response: 0.2)) { isPressed = true }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
                withAnimation { isPressed = false }
            }
            action()
        } label: {
            VStack(spacing: 8) {
                Text(emote.emoji)
                    .font(.system(size: 44))
                    .scaleEffect(isPressed ? 1.3 : 1.0)
                Text(emote.displayName)
                    .font(.system(size: 11, weight: .semibold, design: .monospaced))
                    .foregroundStyle(isRecent ? .cyan : .gray)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 16)
            .background(
                RoundedRectangle(cornerRadius: 14)
                    .fill(isRecent ? Color.cyan.opacity(0.15) : Color(white: 0.1))
                    .overlay(
                        RoundedRectangle(cornerRadius: 14)
                            .stroke(isRecent ? Color.cyan.opacity(0.5) : Color.clear, lineWidth: 1)
                    )
            )
        }
        .buttonStyle(.plain)
    }
}
