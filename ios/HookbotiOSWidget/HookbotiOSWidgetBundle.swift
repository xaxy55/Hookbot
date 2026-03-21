import WidgetKit
import SwiftUI

// iOS WidgetKit extension — Home screen & Lock screen widgets (Phase 13.1)

@main
struct HookbotiOSWidgetBundle: WidgetBundle {
    var body: some Widget {
        HookbotStatusWidget()
        HookbotXPWidget()
        HookbotLockScreenWidget()
        HookbotLiveActivityWidget()
    }
}
