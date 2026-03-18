import SwiftUI

struct WatchActivityView: View {
    @EnvironmentObject var network: WatchNetworkService

    var body: some View {
        ScrollView {
            VStack(spacing: 4) {
                Text("ACTIVITY")
                    .font(.system(size: 10, weight: .black, design: .monospaced))
                    .foregroundColor(.gray)
                    .frame(maxWidth: .infinity, alignment: .leading)

                if network.recentActivity.isEmpty {
                    VStack(spacing: 6) {
                        Text("NO ACTIVITY")
                            .font(.system(size: 10, weight: .bold, design: .monospaced))
                            .foregroundColor(.gray)
                        Text("Start coding to\nsee events here")
                            .font(.system(size: 9, design: .monospaced))
                            .foregroundColor(.gray.opacity(0.6))
                            .multilineTextAlignment(.center)
                    }
                    .padding(.top, 20)
                } else {
                    ForEach(network.recentActivity) { entry in
                        ActivityRow(entry: entry)
                    }
                }
            }
            .padding(.horizontal, 4)
        }
        .background(Color.black)
    }
}

struct ActivityRow: View {
    let entry: ActivityEntry

    var body: some View {
        HStack(spacing: 6) {
            // Tool icon
            Text(toolEmoji)
                .font(.system(size: 12))

            VStack(alignment: .leading, spacing: 1) {
                Text(entry.tool_name)
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)
                    .lineLimit(1)

                Text(relativeTime)
                    .font(.system(size: 8, design: .monospaced))
                    .foregroundColor(.gray)
            }

            Spacer()

            // XP earned
            Text("+\(entry.xp_earned)")
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(.yellow)
        }
        .padding(.vertical, 3)
        .padding(.horizontal, 6)
        .background(
            RoundedRectangle(cornerRadius: 6)
                .fill(Color.white.opacity(0.05))
        )
    }

    private var toolEmoji: String {
        switch entry.tool_name {
        case "Read":  return "👁"
        case "Write": return "✍️"
        case "Edit":  return "✏️"
        case "Bash":  return "⚡"
        case "Grep":  return "🔍"
        case "Glob":  return "📂"
        case "Agent": return "🤖"
        default:      return "⚙️"
        }
    }

    private var relativeTime: String {
        // Parse ISO date and compute relative time
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        guard let date = formatter.date(from: entry.created_at) else {
            // Try without fractional seconds
            formatter.formatOptions = [.withInternetDateTime]
            guard let date = formatter.date(from: entry.created_at) else {
                return entry.created_at
            }
            return relativeString(from: date)
        }
        return relativeString(from: date)
    }

    private func relativeString(from date: Date) -> String {
        let seconds = Int(-date.timeIntervalSinceNow)
        if seconds < 60 { return "\(seconds)s ago" }
        if seconds < 3600 { return "\(seconds / 60)m ago" }
        if seconds < 86400 { return "\(seconds / 3600)h ago" }
        return "\(seconds / 86400)d ago"
    }
}
