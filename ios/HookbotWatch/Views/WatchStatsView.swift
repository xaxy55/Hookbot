import SwiftUI

struct WatchStatsView: View {
    @EnvironmentObject var network: WatchNetworkService

    var body: some View {
        ScrollView {
            VStack(spacing: 8) {
                if let stats = network.stats {
                    // Title & Level
                    Text(stats.title.uppercased())
                        .font(.system(size: 11, weight: .black, design: .monospaced))
                        .foregroundColor(.yellow)

                    Text("LEVEL \(stats.level)")
                        .font(.system(size: 16, weight: .heavy, design: .monospaced))
                        .foregroundColor(.white)

                    // XP Progress bar
                    VStack(spacing: 2) {
                        XPProgressBar(
                            current: stats.total_xp - stats.xp_for_current_level,
                            total: stats.xp_for_next_level - stats.xp_for_current_level
                        )
                        Text("\(stats.total_xp) XP")
                            .font(.system(size: 9, weight: .medium, design: .monospaced))
                            .foregroundColor(.gray)
                    }

                    Divider().background(Color.gray.opacity(0.3))

                    // Stats grid
                    HStack(spacing: 12) {
                        StatBlock(value: "\(stats.total_tool_uses)", label: "TOOLS")
                        StatBlock(value: "\(stats.current_streak)d", label: "STREAK")
                    }

                    HStack(spacing: 12) {
                        StatBlock(value: "\(stats.achievements_earned)", label: "BADGES")
                        StatBlock(value: "\(stats.longest_streak)d", label: "BEST")
                    }
                } else {
                    // No connection
                    VStack(spacing: 8) {
                        Image(systemName: "wifi.slash")
                            .font(.system(size: 24))
                            .foregroundColor(.gray)
                        Text("NO SERVER")
                            .font(.system(size: 10, weight: .bold, design: .monospaced))
                            .foregroundColor(.gray)
                        Text("Configure server\nin Settings")
                            .font(.system(size: 9, design: .monospaced))
                            .foregroundColor(.gray.opacity(0.6))
                            .multilineTextAlignment(.center)
                    }
                    .padding(.top, 20)
                }
            }
            .padding(.horizontal, 8)
        }
        .background(Color.black)
    }
}

// MARK: - XP Progress Bar

struct XPProgressBar: View {
    let current: Int
    let total: Int

    private var progress: Double {
        guard total > 0 else { return 0 }
        return min(1.0, Double(current) / Double(total))
    }

    var body: some View {
        GeometryReader { geo in
            ZStack(alignment: .leading) {
                // Background
                RoundedRectangle(cornerRadius: 3)
                    .fill(Color.white.opacity(0.1))

                // Fill
                RoundedRectangle(cornerRadius: 3)
                    .fill(
                        LinearGradient(
                            colors: [.purple, .yellow],
                            startPoint: .leading,
                            endPoint: .trailing
                        )
                    )
                    .frame(width: geo.size.width * progress)
            }
        }
        .frame(height: 6)
    }
}

// MARK: - Stat Block

struct StatBlock: View {
    let value: String
    let label: String

    var body: some View {
        VStack(spacing: 1) {
            Text(value)
                .font(.system(size: 14, weight: .bold, design: .monospaced))
                .foregroundColor(.white)
            Text(label)
                .font(.system(size: 8, weight: .medium, design: .monospaced))
                .foregroundColor(.gray)
        }
        .frame(maxWidth: .infinity)
    }
}
