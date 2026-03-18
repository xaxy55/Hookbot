import SwiftUI

struct WatchMainView: View {
    @EnvironmentObject var engine: AvatarEngine

    var body: some View {
        VStack(spacing: 4) {
            // Avatar - takes most of the screen
            WatchAvatarView()
                .environmentObject(engine)
                .frame(maxWidth: .infinity, maxHeight: .infinity)

            // State indicator
            Text(engine.currentState.displayName.uppercased())
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(stateColor)

            // Quick state buttons
            HStack(spacing: 8) {
                WatchStateButton(state: .idle, emoji: "😏", engine: engine)
                WatchStateButton(state: .thinking, emoji: "🤔", engine: engine)
                WatchStateButton(state: .success, emoji: "😈", engine: engine)
                WatchStateButton(state: .error, emoji: "💀", engine: engine)
            }
            .padding(.bottom, 4)
        }
        .background(glowBackground)
    }

    private var stateColor: Color {
        switch engine.currentState {
        case .idle:      return Color(red: 0.55, green: 0, blue: 0)
        case .thinking:  return Color(red: 0.29, green: 0, blue: 0.5)
        case .waiting:   return Color(red: 0.72, green: 0.53, blue: 0.04)
        case .success:   return Color(red: 0, green: 0.39, blue: 0)
        case .taskcheck: return Color(red: 0, green: 0.33, blue: 0.33)
        case .error:     return Color.red
        }
    }

    private var glowBackground: some View {
        let led = engine.ledColor
        return RadialGradient(
            gradient: Gradient(colors: [
                Color(red: Double(led.r), green: Double(led.g), blue: Double(led.b)).opacity(0.3),
                .clear
            ]),
            center: .center,
            startRadius: 10,
            endRadius: 100
        )
        .ignoresSafeArea()
    }
}

struct WatchStateButton: View {
    let state: AvatarState
    let emoji: String
    let engine: AvatarEngine

    var body: some View {
        Button {
            engine.setState(state)
        } label: {
            Text(emoji)
                .font(.system(size: 16))
        }
        .buttonStyle(.plain)
        .frame(width: 30, height: 30)
        .background(
            Circle()
                .fill(engine.currentState == state ? Color.white.opacity(0.2) : Color.clear)
        )
    }
}
