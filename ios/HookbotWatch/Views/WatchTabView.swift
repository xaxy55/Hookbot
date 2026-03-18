import SwiftUI

struct WatchTabView: View {
    @EnvironmentObject var engine: AvatarEngine

    var body: some View {
        TabView {
            WatchMainView()
            WatchStatsView()
            WatchActivityView()
            WatchSettingsView()
        }
        .tabViewStyle(.verticalPage)
    }
}
