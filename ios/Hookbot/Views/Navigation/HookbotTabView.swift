import SwiftUI

struct HookbotTabView: View {
    @EnvironmentObject var engine: AvatarEngine
    @EnvironmentObject var network: NetworkService
    @EnvironmentObject var sound: SoundManager
    var auth: AuthService?

    @State private var selectedTab = 0

    var body: some View {
        TabView(selection: $selectedTab) {
            HomeView(auth: auth)
                .tabItem {
                    Label("Home", systemImage: "house.fill")
                }
                .tag(0)

            NavigationStack {
                DeviceListView()
                    .environmentObject(engine)
            }
            .tabItem {
                Label("Devices", systemImage: "cpu.fill")
            }
            .tag(1)

            NavigationStack {
                StoreView()
                    .environmentObject(engine)
            }
            .tabItem {
                Label("Store", systemImage: "cart.fill")
            }
            .tag(2)

            NavigationStack {
                HubView()
                    .environmentObject(engine)
            }
            .tabItem {
                Label("Hub", systemImage: "square.grid.2x2.fill")
            }
            .tag(3)

            NavigationStack {
                SettingsView(auth: auth)
                    .environmentObject(engine)
            }
            .tabItem {
                Label("Settings", systemImage: "gearshape.fill")
            }
            .tag(4)
        }
        .tint(.cyan)
        .preferredColorScheme(.dark)
    }
}
