import SwiftUI

struct WatchMainView: View {
    @EnvironmentObject var engine: AvatarEngine
    @EnvironmentObject var network: WatchNetworkService
    @EnvironmentObject var connectivity: WatchConnectivityManager

    var body: some View {
        VStack(spacing: 2) {
            // Connection indicator
            HStack(spacing: 4) {
                Circle()
                    .fill(connectionColor)
                    .frame(width: 6, height: 6)
                Text(connectionLabel)
                    .font(.system(size: 9, weight: .medium, design: .monospaced))
                    .foregroundColor(.gray)
                Spacer()
                if let stats = network.stats {
                    Text("L\(stats.level)")
                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                        .foregroundColor(.yellow)
                }
            }
            .padding(.horizontal, 4)

            // Avatar — takes most of the screen
            WatchAvatarView()
                .environmentObject(engine)
                .frame(maxWidth: .infinity, maxHeight: .infinity)

            // State indicator
            Text(engine.currentState.displayName.uppercased())
                .font(.system(size: 11, weight: .black, design: .monospaced))
                .foregroundColor(stateColor)

            // Tool name if active
            if !engine.currentTool.name.isEmpty &&
                (engine.currentState == .thinking || engine.currentState == .taskcheck) {
                Text(engine.currentTool.name)
                    .font(.system(size: 9, weight: .medium, design: .monospaced))
                    .foregroundColor(.white.opacity(0.6))
                    .lineLimit(1)
            }

            // Quick state buttons with haptics
            HStack(spacing: 6) {
                WatchStateButton(state: .idle, emoji: "😏", engine: engine, network: network)
                WatchStateButton(state: .thinking, emoji: "🤔", engine: engine, network: network)
                WatchStateButton(state: .success, emoji: "😈", engine: engine, network: network)
                WatchStateButton(state: .error, emoji: "💀", engine: engine, network: network)
            }
            .padding(.bottom, 2)
        }
        .background(glowBackground)
    }

    private var connectionColor: Color {
        if network.isConnected { return .green }
        if connectivity.isReachable { return .blue }
        return .red
    }

    private var connectionLabel: String {
        if network.isConnected { return "SERVER" }
        if connectivity.isReachable { return "iPHONE" }
        return "OFFLINE"
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
                Color(red: Double(led.r), green: Double(led.g), blue: Double(led.b)).opacity(0.4),
                .black
            ]),
            center: .center,
            startRadius: 5,
            endRadius: 80
        )
        .ignoresSafeArea()
    }
}

struct WatchStateButton: View {
    let state: AvatarState
    let emoji: String
    let engine: AvatarEngine
    let network: WatchNetworkService

    var body: some View {
        Button {
            engine.setState(state)
            network.sendState(state)
            HapticManager.shared.playStateHaptic(state)
        } label: {
            Text(emoji)
                .font(.system(size: 18))
        }
        .buttonStyle(.plain)
        .frame(width: 32, height: 32)
        .background(
            Circle()
                .fill(engine.currentState == state ? Color.white.opacity(0.25) : Color.white.opacity(0.05))
        )
    }
}
