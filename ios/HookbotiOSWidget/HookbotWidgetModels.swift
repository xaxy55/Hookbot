import Foundation
import WidgetKit

// MARK: - Widget Entry

struct HookbotWidgetEntry: TimelineEntry {
    let date: Date
    let state: String
    let displayName: String
    let xp: Int
    let streak: Int
    let toolName: String
    let deviceName: String
    let isConnected: Bool

    static var placeholder: HookbotWidgetEntry {
        HookbotWidgetEntry(
            date: .now,
            state: "idle",
            displayName: "Scheming",
            xp: 1337,
            streak: 7,
            toolName: "",
            deviceName: "hookbot",
            isConnected: true
        )
    }
}

// MARK: - Provider

struct HookbotWidgetProvider: TimelineProvider {

    func placeholder(in context: Context) -> HookbotWidgetEntry {
        .placeholder
    }

    func getSnapshot(in context: Context, completion: @escaping (HookbotWidgetEntry) -> Void) {
        if context.isPreview {
            completion(.placeholder)
            return
        }
        loadEntry(completion: completion)
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<HookbotWidgetEntry>) -> Void) {
        loadEntry { entry in
            // Refresh every 3 minutes
            let refresh = Calendar.current.date(byAdding: .minute, value: 3, to: .now) ?? .now
            completion(Timeline(entries: [entry], policy: .after(refresh)))
        }
    }

    // MARK: - Private

    private func loadEntry(completion: @escaping (HookbotWidgetEntry) -> Void) {
        guard
            let data = UserDefaults(suiteName: "group.com.mr-ai.hookbot")?.data(forKey: "widget_state"),
            let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            completion(.placeholder)
            return
        }

        let entry = HookbotWidgetEntry(
            date: .now,
            state: json["state"] as? String ?? "idle",
            displayName: json["display_name"] as? String ?? "Scheming",
            xp: json["xp"] as? Int ?? 0,
            streak: json["streak"] as? Int ?? 0,
            toolName: json["tool"] as? String ?? "",
            deviceName: json["device_name"] as? String ?? "hookbot",
            isConnected: json["connected"] as? Bool ?? false
        )
        completion(entry)
    }
}
