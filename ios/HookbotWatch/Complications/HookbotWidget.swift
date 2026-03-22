import WidgetKit
import SwiftUI

struct HookbotWidget: Widget {
    let kind = "HookbotComplication"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: HookbotComplicationProvider()) { entry in
            HookbotComplicationEntryView(entry: entry)
        }
        .configurationDisplayName("Hookbot")
        .description("Shows your Hookbot's current state and level.")
        .supportedFamilies({
            var families: [WidgetFamily] = [
                .accessoryCircular,
                .accessoryRectangular,
                .accessoryInline,
            ]
            #if os(watchOS)
            families.append(.accessoryCorner)
            #endif
            return families
        }())
    }
}

struct HookbotComplicationEntryView: View {
    @Environment(\.widgetFamily) var family
    let entry: HookbotComplicationEntry

    var body: some View {
        switch family {
        case .accessoryCircular:
            HookbotComplicationCircular(entry: entry)
        case .accessoryRectangular:
            HookbotComplicationRectangular(entry: entry)
        case .accessoryInline:
            HookbotComplicationInline(entry: entry)
        default:
            HookbotComplicationCircular(entry: entry)
        }
    }
}
