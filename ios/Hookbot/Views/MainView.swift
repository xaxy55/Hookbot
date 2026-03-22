import SwiftUI

struct MainView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.verticalSizeClass) var verticalSizeClass
    @State private var showSettings = false
    @State private var showPetCare = false
    @State private var showSocialHub = false
    @State private var showPhotoMode = false
    var auth: AuthService?

    @StateObject private var breakReminder = BreakReminderManager.shared
    @StateObject private var sleepGuardian = SleepGuardianManager.shared
    @StateObject private var seasonalEvents = SeasonalEventsManager.shared
    @StateObject private var evolution = AvatarEvolutionEngine.shared

    var isLandscape: Bool { verticalSizeClass == .compact }

    var body: some View {
        NavigationStack {
            ZStack {
                glowBackground

                VStack(spacing: 0) {
                    if !isLandscape {
                        statusBar
                    }

                    if seasonalEvents.isEventActive && !isLandscape {
                        SeasonalBannerView(season: seasonalEvents.activeSeason)
                            .padding(.horizontal)
                            .padding(.bottom, 4)
                    }

                    if evolution.isLeveledUp && !isLandscape {
                        levelUpToast
                            .transition(.move(edge: .top).combined(with: .opacity))
                    }

                    // Avatar - fills all available space
                    AvatarView()
                        .environmentObject(engine)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .padding(isLandscape ? 4 : 16)

                    if !isLandscape {
                        // Control panel (portrait only)
                        ControlPanelView()
                            .environmentObject(engine)
                            .padding(.bottom, 8)
                    }
                }

                if breakReminder.shouldShowBreakPrompt {
                    breakPromptOverlay
                        .transition(.opacity)
                        .zIndex(8)
                }

                if breakReminder.shouldShowEyeStrainAlert {
                    eyeStrainOverlay
                        .transition(.opacity)
                        .zIndex(9)
                }

                if sleepGuardian.showSleepPrompt {
                    SleepGuardianPrompt(
                        sleepiness: sleepGuardian.sleepiness
                    ) {
                        withAnimation { sleepGuardian.dismissPrompt() }
                    } onStopCoding: {
                        withAnimation {
                            sleepGuardian.dismissPrompt()
                            engine.setState(.idle)
                        }
                    }
                    .transition(.opacity)
                    .zIndex(10)
                }
            }
            .background(Color.black)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar(isLandscape ? .hidden : .visible, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    HStack(spacing: 12) {
                        Button {
                            showPetCare = true
                        } label: {
                            Image(systemName: "pawprint.fill")
                                .foregroundColor(.gray)
                        }
                        .accessibilityIdentifier("petCareButton")

                        Button {
                            showSocialHub = true
                        } label: {
                            ZStack {
                                Image(systemName: "person.2.fill")
                                    .foregroundColor(.gray)
                                if SocialService.shared.unreadVisits > 0 {
                                    Circle()
                                        .fill(Color.red)
                                        .frame(width: 8, height: 8)
                                        .offset(x: 8, y: -8)
                                }
                            }
                        }
                        .accessibilityIdentifier("socialButton")
                    }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    HStack(spacing: 12) {
                        Button {
                            showPhotoMode = true
                        } label: {
                            Image(systemName: "camera.fill")
                                .foregroundColor(.gray)
                        }
                        Button {
                            showSettings = true
                        } label: {
                            Image(systemName: "gearshape")
                                .foregroundColor(.gray)
                        }
                        .accessibilityIdentifier("settingsButton")
                    }
                }
            }
            .sheet(isPresented: $showPetCare) {
                NavigationStack {
                    PetCareView()
                        .environmentObject(engine)
                        .navigationTitle("Pet Care")
                        .navigationBarTitleDisplayMode(.inline)
                        .toolbar {
                            ToolbarItem(placement: .topBarTrailing) {
                                Button("Done") { showPetCare = false }
                            }
                        }
                }
                .preferredColorScheme(.dark)
            }
            .sheet(isPresented: $showSettings) {
                SettingsView(auth: auth)
                    .environmentObject(engine)
            }
            .sheet(isPresented: $showSocialHub) {
                SocialHubView()
                    .environmentObject(engine)
            }
            .fullScreenCover(isPresented: $showPhotoMode) {
                PhotoModeView()
                    .environmentObject(engine)
            }
        }
        .preferredColorScheme(.dark)
        .onAppear {
            breakReminder.startTracking()
        }
    }

    // MARK: - Status bar

    private var statusBar: some View {
        HStack {
            Circle()
                .fill(Color.green)
                .frame(width: 8, height: 8)
            Text(engine.currentState.displayName.uppercased())
                .font(.system(size: 12, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)
            Spacer()
            if evolution.currentEvolution != .allRounder {
                Label(evolution.currentEvolution.displayName, systemImage: "sparkles")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.cyan)
            } else {
                Text("DESTROYER OF WORLDS")
                    .font(.system(size: 10, weight: .medium, design: .monospaced))
                    .foregroundColor(Color(white: 0.3))
            }
        }
        .padding(.horizontal)
        .padding(.top, 8)
    }

    // MARK: - Level up toast

    private var levelUpToast: some View {
        HStack(spacing: 12) {
            Text("⬆️")
            VStack(alignment: .leading, spacing: 2) {
                Text("LEVEL UP!")
                    .font(.system(size: 13, weight: .black, design: .monospaced))
                    .foregroundStyle(.yellow)
                Text("Level \(evolution.currentLevel) — \(evolution.currentEvolution.displayName)")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.white)
            }
            Spacer()
            Text(evolution.currentEvolution.emoji)
                .font(.system(size: 24))
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(Color.yellow.opacity(0.2))
                .overlay(RoundedRectangle(cornerRadius: 12).stroke(Color.yellow.opacity(0.6), lineWidth: 1))
        )
        .padding(.horizontal)
    }

    // MARK: - Break prompt overlay

    private var breakPromptOverlay: some View {
        ZStack {
            Color.black.opacity(0.7).ignoresSafeArea()
            VStack(spacing: 20) {
                Text("🧘")
                    .font(.system(size: 60))
                Text("Time for a stretch!")
                    .font(.system(size: 20, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)
                Text("You've been coding for \(breakReminder.sessionMinutes) minutes.")
                    .font(.system(size: 13, design: .monospaced))
                    .foregroundStyle(.gray)
                    .multilineTextAlignment(.center)

                HStack(spacing: 16) {
                    Button {
                        withAnimation { breakReminder.snoozeBreak(minutes: 5) }
                    } label: {
                        Text("5 more min")
                            .font(.system(size: 13, design: .monospaced))
                            .foregroundStyle(.gray)
                            .padding(.horizontal, 16)
                            .padding(.vertical, 10)
                            .background(Capsule().fill(Color(white: 0.15)))
                    }
                    .buttonStyle(.plain)

                    Button {
                        withAnimation { breakReminder.dismissBreakPrompt() }
                    } label: {
                        Text("Taking a break")
                            .font(.system(size: 13, weight: .bold, design: .monospaced))
                            .foregroundStyle(.black)
                            .padding(.horizontal, 20)
                            .padding(.vertical, 10)
                            .background(Capsule().fill(Color.cyan))
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(28)
            .background(RoundedRectangle(cornerRadius: 20).fill(Color(white: 0.1)))
            .padding(24)
        }
    }

    // MARK: - Eye strain overlay

    private var eyeStrainOverlay: some View {
        VStack {
            HStack(spacing: 10) {
                Text("👁️")
                Text("20-20-20: Look 20ft away for 20 seconds")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.white)
                Spacer()
                Button {
                    breakReminder.dismissEyeStrainAlert()
                } label: {
                    Image(systemName: "xmark")
                        .foregroundStyle(.gray)
                }
            }
            .padding(12)
            .background(.ultraThinMaterial)
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .padding(.horizontal)
            Spacer()
        }
        .padding(.top, 60)
    }

    // MARK: - Glow background

    private var glowBackground: some View {
        let led = engine.ledColor
        let glowColor = Color(
            red: Double(led.r),
            green: Double(led.g),
            blue: Double(led.b)
        )
        let seasonColor = seasonalEvents.activeSeason.themeColor
        return ZStack {
            RadialGradient(
                gradient: Gradient(colors: [glowColor.opacity(0.4), .clear]),
                center: .center,
                startRadius: 50,
                endRadius: 300
            )
            if seasonalEvents.isEventActive {
                RadialGradient(
                    gradient: Gradient(colors: [seasonColor.opacity(0.15), .clear]),
                    center: .topTrailing,
                    startRadius: 0,
                    endRadius: 200
                )
            }
        }
        .ignoresSafeArea()
    }
}

// MARK: - Social Hub Sheet

struct SocialHubView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section("Friends") {
                    NavigationLink {
                        FriendListView()
                            .environmentObject(engine)
                    } label: {
                        Label("Friend List", systemImage: "person.2.fill")
                    }
                    NavigationLink {
                        PairingView()
                            .environmentObject(engine)
                    } label: {
                        Label("Pair with Buddy", systemImage: "qrcode")
                    }
                    NavigationLink {
                        VisitHistoryView()
                            .environmentObject(engine)
                    } label: {
                        Label("Visit History", systemImage: "clock.arrow.circlepath")
                    }
                    NavigationLink {
                        TeamCampfireView()
                            .environmentObject(engine)
                    } label: {
                        Label("Team Campfire", systemImage: "flame.fill")
                    }
                    NavigationLink {
                        CodingChallengeView()
                            .environmentObject(engine)
                    } label: {
                        Label("Challenges", systemImage: "trophy.fill")
                    }
                    NavigationLink {
                        WeeklyPhotoCardView()
                            .environmentObject(engine)
                    } label: {
                        Label("Weekly Card", systemImage: "photo.fill")
                    }
                }

                Section("AR & Games") {
                    NavigationLink {
                        SharedARView()
                            .environmentObject(engine)
                    } label: {
                        Label("Shared AR Space", systemImage: "arkit")
                    }
                    NavigationLink {
                        PongGameView()
                            .environmentObject(engine)
                    } label: {
                        Label("AR Pong", systemImage: "gamecontroller.fill")
                    }
                }

                Section("Your Avatar") {
                    NavigationLink {
                        WardrobeView()
                            .environmentObject(engine)
                    } label: {
                        Label("Wardrobe", systemImage: "tshirt.fill")
                    }
                    NavigationLink {
                        AvatarJournalView()
                            .environmentObject(engine)
                    } label: {
                        Label("Avatar Journal", systemImage: "book.fill")
                    }
                    NavigationLink {
                        AchievementShowcaseView()
                            .environmentObject(engine)
                    } label: {
                        Label("Achievements", systemImage: "rosette")
                    }
                    NavigationLink {
                        DailyRewardView()
                            .environmentObject(engine)
                    } label: {
                        Label("Daily Reward", systemImage: "gift.fill")
                    }
                }

                Section("Focus") {
                    NavigationLink {
                        PomodoroView()
                            .environmentObject(engine)
                    } label: {
                        Label("Pomodoro", systemImage: "timer")
                    }
                }
            }
            .scrollContentBackground(.hidden)
            .background(Color.black)
            .navigationTitle("Social Hub")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .preferredColorScheme(.dark)
    }
}
