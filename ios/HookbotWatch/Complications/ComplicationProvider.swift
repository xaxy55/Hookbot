import WidgetKit
import SwiftUI

struct HookbotComplicationEntry: TimelineEntry {
    let date: Date
    let state: String
    let level: Int
    let xp: Int
    let streak: Int
}

struct HookbotComplicationProvider: TimelineProvider {
    func placeholder(in context: Context) -> HookbotComplicationEntry {
        HookbotComplicationEntry(date: .now, state: "idle", level: 1, xp: 0, streak: 0)
    }

    func getSnapshot(in context: Context, completion: @escaping (HookbotComplicationEntry) -> Void) {
        let entry = HookbotComplicationEntry(
            date: .now,
            state: UserDefaults.standard.string(forKey: "hookbot_last_state") ?? "idle",
            level: UserDefaults.standard.integer(forKey: "hookbot_level"),
            xp: UserDefaults.standard.integer(forKey: "hookbot_xp"),
            streak: UserDefaults.standard.integer(forKey: "hookbot_streak")
        )
        completion(entry)
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<HookbotComplicationEntry>) -> Void) {
        let entry = HookbotComplicationEntry(
            date: .now,
            state: UserDefaults.standard.string(forKey: "hookbot_last_state") ?? "idle",
            level: UserDefaults.standard.integer(forKey: "hookbot_level"),
            xp: UserDefaults.standard.integer(forKey: "hookbot_xp"),
            streak: UserDefaults.standard.integer(forKey: "hookbot_streak")
        )
        // Refresh every 5 minutes
        let nextUpdate = Calendar.current.date(byAdding: .minute, value: 5, to: .now)!
        let timeline = Timeline(entries: [entry], policy: .after(nextUpdate))
        completion(timeline)
    }
}

// MARK: - Complication Views

struct HookbotComplicationCircular: View {
    let entry: HookbotComplicationEntry

    var body: some View {
        ZStack {
            AccessoryWidgetBackground()
            VStack(spacing: 0) {
                Text(stateEmoji)
                    .font(.system(size: 16))
                Text("L\(max(1, entry.level))")
                    .font(.system(size: 8, weight: .bold, design: .monospaced))
                    .foregroundColor(.white)
            }
        }
    }

    private var stateEmoji: String {
        switch entry.state {
        case "thinking":  return "🤔"
        case "waiting":   return "😤"
        case "success":   return "😈"
        case "error":     return "💀"
        case "taskcheck": return "✅"
        default:          return "😏"
        }
    }
}

struct HookbotComplicationRectangular: View {
    let entry: HookbotComplicationEntry

    var body: some View {
        HStack(spacing: 4) {
            Text(stateEmoji)
                .font(.system(size: 20))

            VStack(alignment: .leading, spacing: 1) {
                Text("HOOKBOT")
                    .font(.system(size: 8, weight: .bold, design: .monospaced))
                    .foregroundColor(.white)
                Text("L\(max(1, entry.level)) · \(entry.xp)XP")
                    .font(.system(size: 9, weight: .medium, design: .monospaced))
                    .foregroundColor(.yellow)
                if entry.streak > 0 {
                    Text("\(entry.streak)d streak 🔥")
                        .font(.system(size: 8, design: .monospaced))
                        .foregroundColor(.orange)
                }
            }
        }
    }

    private var stateEmoji: String {
        switch entry.state {
        case "thinking":  return "🤔"
        case "waiting":   return "😤"
        case "success":   return "😈"
        case "error":     return "💀"
        case "taskcheck": return "✅"
        default:          return "😏"
        }
    }
}

struct HookbotComplicationInline: View {
    let entry: HookbotComplicationEntry

    var body: some View {
        Text("\(stateLabel) · L\(max(1, entry.level))")
    }

    private var stateLabel: String {
        switch entry.state {
        case "thinking":  return "Plotting"
        case "waiting":   return "Displeased"
        case "success":   return "Conquered"
        case "error":     return "DESTROY"
        case "taskcheck": return "Approved"
        default:          return "Scheming"
        }
    }
}
