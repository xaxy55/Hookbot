import WidgetKit
import SwiftUI

// MARK: - Status Widget (Home Screen)

struct HookbotStatusWidget: Widget {
    let kind = "HookbotStatus"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: HookbotWidgetProvider()) { entry in
            HookbotStatusWidgetView(entry: entry)
                .containerBackground(.black, for: .widget)
        }
        .configurationDisplayName("Hookbot Status")
        .description("See your hookbot's live mood and current tool.")
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}

struct HookbotStatusWidgetView: View {
    let entry: HookbotWidgetEntry

    var glowColor: Color {
        switch entry.state {
        case "error":     return .red
        case "success":   return .green
        case "thinking":  return .purple
        case "waiting":   return .orange
        case "taskcheck": return .cyan
        default:          return .blue
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Circle()
                    .fill(entry.isConnected ? Color.green : Color.red)
                    .frame(width: 7, height: 7)
                Text(entry.deviceName.uppercased())
                    .font(.system(size: 9, weight: .bold, design: .monospaced))
                    .foregroundStyle(.gray)
                    .lineLimit(1)
                Spacer()
            }

            Spacer()

            // State indicator
            Text(entry.displayName.uppercased())
                .font(.system(size: 18, weight: .heavy, design: .monospaced))
                .foregroundStyle(glowColor)
                .minimumScaleFactor(0.6)
                .lineLimit(1)

            if !entry.toolName.isEmpty {
                Text(entry.toolName)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.gray)
                    .lineLimit(1)
            }

            Spacer()

            HStack {
                Label("\(entry.xp) XP", systemImage: "star.fill")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.yellow)
                Spacer()
                Label("\(entry.streak)d", systemImage: "flame.fill")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.orange)
            }
        }
        .padding(12)
        .background(
            RadialGradient(
                colors: [glowColor.opacity(0.3), .clear],
                center: .center,
                startRadius: 10,
                endRadius: 80
            )
        )
    }
}

// MARK: - XP / Streak Widget

struct HookbotXPWidget: Widget {
    let kind = "HookbotXP"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: HookbotWidgetProvider()) { entry in
            HookbotXPWidgetView(entry: entry)
                .containerBackground(.black, for: .widget)
        }
        .configurationDisplayName("Hookbot XP & Streak")
        .description("Track your XP bar and coding streak at a glance.")
        .supportedFamilies([.systemSmall, .accessoryCircular, .accessoryRectangular])
    }
}

struct HookbotXPWidgetView: View {
    let entry: HookbotWidgetEntry
    @Environment(\.widgetFamily) var family

    var body: some View {
        switch family {
        case .accessoryCircular:
            accessoryCircular
        case .accessoryRectangular:
            accessoryRectangular
        default:
            homeScreenXP
        }
    }

    private var accessoryCircular: some View {
        ZStack {
            AccessoryWidgetBackground()
            VStack(spacing: 2) {
                Image(systemName: "flame.fill")
                    .font(.system(size: 14, weight: .bold))
                    .foregroundStyle(.orange)
                Text("\(entry.streak)")
                    .font(.system(size: 16, weight: .black, design: .monospaced))
            }
        }
    }

    private var accessoryRectangular: some View {
        HStack(spacing: 8) {
            Image(systemName: "flame.fill").foregroundStyle(.orange)
            VStack(alignment: .leading, spacing: 1) {
                Text("\(entry.streak)-day streak")
                    .font(.system(size: 12, weight: .bold, design: .monospaced))
                Text("\(entry.xp) XP")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.gray)
            }
        }
    }

    private var homeScreenXP: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("STREAK", systemImage: "flame.fill")
                .font(.system(size: 9, weight: .bold, design: .monospaced))
                .foregroundStyle(.orange)

            Text("\(entry.streak) days")
                .font(.system(size: 22, weight: .black, design: .monospaced))
                .foregroundStyle(.white)

            Label("XP BAR", systemImage: "star.fill")
                .font(.system(size: 9, weight: .bold, design: .monospaced))
                .foregroundStyle(.yellow)

            // XP bar (0-1000 per level)
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 3)
                        .fill(Color(white: 0.15))
                    RoundedRectangle(cornerRadius: 3)
                        .fill(Color.yellow)
                        .frame(width: geo.size.width * CGFloat(entry.xp % 1000) / 1000.0)
                }
            }
            .frame(height: 6)

            Text("\(entry.xp % 1000) / 1000 XP")
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .padding(12)
    }
}

// MARK: - Lock Screen Widget

struct HookbotLockScreenWidget: Widget {
    let kind = "HookbotLockScreen"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: HookbotWidgetProvider()) { entry in
            HookbotLockScreenView(entry: entry)
                .containerBackground(.clear, for: .widget)
        }
        .configurationDisplayName("Hookbot Lock Screen")
        .description("Hookbot mood and streak on your lock screen.")
        .supportedFamilies([.accessoryCircular, .accessoryRectangular, .accessoryInline])
    }
}

struct HookbotLockScreenView: View {
    let entry: HookbotWidgetEntry
    @Environment(\.widgetFamily) var family

    var body: some View {
        switch family {
        case .accessoryInline:
            Label("\(entry.displayName) · \(entry.streak)d", systemImage: "cpu")
        case .accessoryCircular:
            ZStack {
                AccessoryWidgetBackground()
                VStack(spacing: 1) {
                    Text(stateEmoji)
                        .font(.system(size: 18))
                    Text("\(entry.streak)d")
                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                }
            }
        default:
            HStack(spacing: 8) {
                Text(stateEmoji).font(.system(size: 20))
                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.displayName)
                        .font(.system(size: 12, weight: .bold, design: .monospaced))
                    Text("\(entry.streak)-day streak · \(entry.xp) XP")
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var stateEmoji: String {
        switch entry.state {
        case "idle":      return "😏"
        case "thinking":  return "🤔"
        case "waiting":   return "😤"
        case "success":   return "😎"
        case "taskcheck": return "✅"
        case "error":     return "🤬"
        default:          return "🤖"
        }
    }
}

// MARK: - Live Activity Widget

struct HookbotLiveActivityWidget: Widget {
    let kind = "HookbotLiveActivity"

    var body: some WidgetConfiguration {
        ActivityConfiguration(for: HookbotActivityAttributes.self) { context in
            // Lock screen / Notification banner
            HookbotLiveActivityBannerView(
                state: context.state.state,
                displayName: context.state.displayName,
                toolName: context.state.toolName,
                deviceName: context.attributes.deviceName,
                xp: context.state.xp,
                streak: context.state.streak
            )
            .containerBackground(.black.opacity(0.9), for: .widget)

        } dynamicIsland: { context in
            DynamicIsland {
                // Expanded
                DynamicIslandExpandedRegion(.leading) {
                    HStack(spacing: 6) {
                        Circle()
                            .fill(dynamicIslandColor(context.state.state))
                            .frame(width: 10, height: 10)
                        Text(context.state.displayName.uppercased())
                            .font(.system(size: 12, weight: .bold, design: .monospaced))
                            .foregroundStyle(.white)
                    }
                }
                DynamicIslandExpandedRegion(.trailing) {
                    Label("\(context.state.streak)d", systemImage: "flame.fill")
                        .font(.system(size: 12, weight: .bold, design: .monospaced))
                        .foregroundStyle(.orange)
                }
                DynamicIslandExpandedRegion(.bottom) {
                    if !context.state.toolName.isEmpty {
                        HStack {
                            Image(systemName: "wrench.and.screwdriver")
                                .font(.caption2)
                            Text(context.state.toolName)
                                .font(.system(size: 11, design: .monospaced))
                            Spacer()
                            Text("\(context.state.xp) XP")
                                .font(.system(size: 11, weight: .semibold, design: .monospaced))
                                .foregroundStyle(.yellow)
                        }
                        .foregroundStyle(.gray)
                    }
                }
            } compactLeading: {
                Circle()
                    .fill(dynamicIslandColor(context.state.state))
                    .frame(width: 10, height: 10)
            } compactTrailing: {
                Text(context.state.displayName.prefix(4).uppercased())
                    .font(.system(size: 10, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)
            } minimal: {
                Circle()
                    .fill(dynamicIslandColor(context.state.state))
            }
        }
    }

    private func dynamicIslandColor(_ state: String) -> Color {
        switch state {
        case "error":     return .red
        case "success":   return .green
        case "thinking":  return .purple
        case "waiting":   return .orange
        case "taskcheck": return .cyan
        default:          return .blue
        }
    }
}

struct HookbotLiveActivityBannerView: View {
    let state: String
    let displayName: String
    let toolName: String
    let deviceName: String
    let xp: Int
    let streak: Int

    var glowColor: Color {
        switch state {
        case "error":     return .red
        case "success":   return .green
        case "thinking":  return .purple
        case "waiting":   return .orange
        default:          return .blue
        }
    }

    var body: some View {
        HStack(spacing: 12) {
            Circle()
                .fill(glowColor)
                .frame(width: 12, height: 12)

            VStack(alignment: .leading, spacing: 2) {
                Text(displayName.uppercased())
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)
                if !toolName.isEmpty {
                    Text(toolName)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.gray)
                }
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 2) {
                Label("\(streak)d", systemImage: "flame.fill")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundStyle(.orange)
                Text("\(xp) XP")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.yellow)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }
}
