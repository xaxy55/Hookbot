import SwiftUI

/// Hub tab — expanded menu with all feature sections
struct HubView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.dismiss) var dismiss

    var body: some View {
        List {
            Section("Social") {
                NavigationLink {
                    FriendListView().environmentObject(engine)
                } label: { Label("Friend List", systemImage: "person.2.fill") }

                NavigationLink {
                    PairingView().environmentObject(engine)
                } label: { Label("Pair with Buddy", systemImage: "qrcode") }

                NavigationLink {
                    VisitHistoryView().environmentObject(engine)
                } label: { Label("Visit History", systemImage: "clock.arrow.circlepath") }

                NavigationLink {
                    TeamCampfireView().environmentObject(engine)
                } label: { Label("Team Campfire", systemImage: "flame.fill") }

                NavigationLink {
                    CodingChallengeView().environmentObject(engine)
                } label: { Label("Challenges", systemImage: "trophy.fill") }

                NavigationLink {
                    WeeklyPhotoCardView().environmentObject(engine)
                } label: { Label("Weekly Card", systemImage: "photo.fill") }

                NavigationLink {
                    TeamDashboardView()
                } label: { Label("Team Dashboard", systemImage: "person.3.fill") }

                NavigationLink {
                    GlobalWallView()
                } label: { Label("Global Wall", systemImage: "globe") }
            }

            Section("Avatar") {
                NavigationLink {
                    WardrobeView().environmentObject(engine)
                } label: { Label("Wardrobe", systemImage: "tshirt.fill") }

                NavigationLink {
                    AvatarJournalView().environmentObject(engine)
                } label: { Label("Avatar Journal", systemImage: "book.fill") }

                NavigationLink {
                    AchievementShowcaseView().environmentObject(engine)
                } label: { Label("Achievements", systemImage: "rosette") }

                NavigationLink {
                    DailyRewardView().environmentObject(engine)
                } label: { Label("Daily Reward", systemImage: "gift.fill") }
            }

            Section("Activity") {
                NavigationLink {
                    ActivityFeedView()
                } label: { Label("Activity Feed", systemImage: "list.bullet") }

                NavigationLink {
                    AnalyticsView()
                } label: { Label("Analytics", systemImage: "chart.bar.fill") }

                NavigationLink {
                    DeveloperInsightsView()
                } label: { Label("Developer Insights", systemImage: "brain.head.profile") }
            }

            Section("Focus & Wellness") {
                NavigationLink {
                    PomodoroView().environmentObject(engine)
                } label: { Label("Pomodoro", systemImage: "timer") }

                NavigationLink {
                    MoodJournalView()
                } label: { Label("Mood Journal", systemImage: "face.smiling") }
            }

            Section("Desk") {
                NavigationLink {
                    StandingDeskView()
                } label: { Label("Standing Desk", systemImage: "arrow.up.arrow.down") }

                NavigationLink {
                    DeskLightsView()
                } label: { Label("Desk Lights", systemImage: "lightbulb.fill") }
            }

            Section("Sound & Light") {
                NavigationLink {
                    SoundBoardView()
                } label: { Label("Sound Board", systemImage: "speaker.wave.3.fill") }

                NavigationLink {
                    LightShowView()
                } label: { Label("Light Show", systemImage: "sparkles") }
            }

            Section("Integrations") {
                NavigationLink {
                    IntegrationsView()
                } label: { Label("Webhooks & Services", systemImage: "link") }

                NavigationLink {
                    HomeAssistantView()
                } label: { Label("Home Assistant", systemImage: "house.fill") }

                NavigationLink {
                    StreamDeckView()
                } label: { Label("Stream Deck", systemImage: "keyboard") }
            }

            Section("Community") {
                NavigationLink {
                    CommunityStoreView()
                } label: { Label("Community Store", systemImage: "storefront") }
            }

            Section("AR & Games") {
                NavigationLink {
                    SharedARView().environmentObject(engine)
                } label: { Label("Shared AR Space", systemImage: "arkit") }

                NavigationLink {
                    PongGameView().environmentObject(engine)
                } label: { Label("AR Pong", systemImage: "gamecontroller.fill") }
            }

            Section("System") {
                NavigationLink {
                    DiagnosticsView()
                } label: { Label("Diagnostics", systemImage: "stethoscope") }

                NavigationLink {
                    LogsView()
                } label: { Label("Logs", systemImage: "doc.text") }

                NavigationLink {
                    OtaView()
                } label: { Label("OTA Updates", systemImage: "arrow.down.circle") }

                NavigationLink {
                    DiscoveryView()
                } label: { Label("Discovery", systemImage: "network") }
            }
        }
        .scrollContentBackground(.hidden)
        .background(Color.black)
        .navigationTitle("Hub")
        .navigationBarTitleDisplayMode(.inline)
    }
}
