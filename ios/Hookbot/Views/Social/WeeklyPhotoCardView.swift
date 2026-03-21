import SwiftUI
import UIKit

// Auto-generated weekly stats card — shareable to Twitter/Discord (Phase 13.4)

struct WeeklyPhotoCardView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var cardImage: UIImage?
    @State private var showShareSheet = false
    @State private var weeklyStats: WeeklyStats = .empty

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Card preview
                weeklyCard
                    .clipShape(RoundedRectangle(cornerRadius: 20))
                    .shadow(color: .blue.opacity(0.3), radius: 20)

                // Share button
                Button {
                    generateAndShare()
                } label: {
                    Label("Share to Twitter / Discord", systemImage: "square.and.arrow.up")
                        .font(.system(size: 15, weight: .bold, design: .monospaced))
                        .foregroundStyle(.black)
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(Color.cyan)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .buttonStyle(.plain)

                // Stats breakdown
                statsBreakdown
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Weekly Card")
        .navigationBarTitleDisplayMode(.inline)
        .preferredColorScheme(.dark)
        .onAppear { loadWeeklyStats() }
        .sheet(isPresented: $showShareSheet) {
            if let image = cardImage {
                ShareSheet(items: [image, weeklyStats.shareCaption])
            }
        }
    }

    // MARK: - Card design

    private var weeklyCard: some View {
        ZStack {
            // Background gradient
            LinearGradient(
                colors: [Color(red: 0.05, green: 0.02, blue: 0.15), Color(red: 0.02, green: 0.1, blue: 0.2)],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            // Grid pattern
            Canvas { ctx, size in
                let spacing: CGFloat = 30
                var path = Path()
                var x: CGFloat = 0
                while x < size.width {
                    path.move(to: CGPoint(x: x, y: 0))
                    path.addLine(to: CGPoint(x: x, y: size.height))
                    x += spacing
                }
                var y: CGFloat = 0
                while y < size.height {
                    path.move(to: CGPoint(x: 0, y: y))
                    path.addLine(to: CGPoint(x: size.width, y: y))
                    y += spacing
                }
                ctx.stroke(path, with: .color(.cyan.opacity(0.07)), lineWidth: 0.5)
            }

            VStack(spacing: 20) {
                // Header
                HStack {
                    VStack(alignment: .leading, spacing: 3) {
                        Text("HOOKBOT")
                            .font(.system(size: 12, weight: .black, design: .monospaced))
                            .foregroundStyle(.cyan)
                        Text("Weekly Report")
                            .font(.system(size: 22, weight: .black, design: .monospaced))
                            .foregroundStyle(.white)
                        Text(weekRangeString)
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(.gray)
                    }
                    Spacer()
                    // Avatar preview
                    AvatarView()
                        .environmentObject(engine)
                        .frame(width: 70, height: 70)
                }
                .padding(.horizontal, 20)
                .padding(.top, 20)

                // Main stats
                HStack(spacing: 12) {
                    cardStat(value: "\(weeklyStats.totalXP)", label: "XP EARNED", color: .yellow, icon: "star.fill")
                    cardStat(value: "\(weeklyStats.currentStreak)d", label: "STREAK", color: .orange, icon: "flame.fill")
                    cardStat(value: "\(weeklyStats.totalDeploys)", label: "DEPLOYS", color: .green, icon: "paperplane.fill")
                }
                .padding(.horizontal, 16)

                // Achievement badges
                if !weeklyStats.achievements.isEmpty {
                    HStack(spacing: 8) {
                        ForEach(weeklyStats.achievements.prefix(4), id: \.self) { badge in
                            Text(badge)
                                .font(.system(size: 22))
                                .padding(8)
                                .background(Circle().fill(Color.white.opacity(0.1)))
                        }
                        if weeklyStats.achievements.count > 4 {
                            Text("+\(weeklyStats.achievements.count - 4)")
                                .font(.system(size: 11, weight: .bold, design: .monospaced))
                                .foregroundStyle(.gray)
                        }
                    }
                }

                // Activity bar chart
                activityBars

                // Footer
                HStack {
                    Text("hookbot.mr-ai.no")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.gray)
                    Spacer()
                    Text("@\(engine.config.deviceName)")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.cyan)
                }
                .padding(.horizontal, 20)
                .padding(.bottom, 16)
            }
        }
        .frame(width: UIScreen.main.bounds.width - 32, height: 380)
    }

    private var activityBars: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("ACTIVITY")
                .font(.system(size: 9, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)
                .padding(.horizontal, 20)

            HStack(alignment: .bottom, spacing: 6) {
                ForEach(Array(weeklyStats.dailyActivity.enumerated()), id: \.offset) { i, value in
                    VStack(spacing: 4) {
                        RoundedRectangle(cornerRadius: 3)
                            .fill(value > 0 ? Color.cyan : Color(white: 0.15))
                            .frame(width: 24, height: CGFloat(max(4, value)) * 2)
                        Text(dayLabel(i))
                            .font(.system(size: 8, design: .monospaced))
                            .foregroundStyle(.gray)
                    }
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.horizontal, 20)
        }
    }

    private func cardStat(value: String, label: String, color: Color, icon: String) -> some View {
        VStack(spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 14))
                .foregroundStyle(color)
            Text(value)
                .font(.system(size: 20, weight: .black, design: .monospaced))
                .foregroundStyle(.white)
                .minimumScaleFactor(0.6)
                .lineLimit(1)
            Text(label)
                .font(.system(size: 8, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .frame(maxWidth: .infinity)
        .padding(10)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color.white.opacity(0.07)))
    }

    // MARK: - Stats breakdown

    private var statsBreakdown: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("THIS WEEK")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)

            statRow(label: "Total XP", value: "\(weeklyStats.totalXP)", icon: "star.fill", color: .yellow)
            statRow(label: "Current Streak", value: "\(weeklyStats.currentStreak) days", icon: "flame.fill", color: .orange)
            statRow(label: "Deploys", value: "\(weeklyStats.totalDeploys)", icon: "paperplane.fill", color: .green)
            statRow(label: "Errors Survived", value: "\(weeklyStats.errors)", icon: "exclamationmark.triangle.fill", color: .red)
            statRow(label: "Tool Calls", value: "\(weeklyStats.toolCalls)", icon: "wrench.and.screwdriver.fill", color: .cyan)
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 14).fill(Color(white: 0.08)))
    }

    private func statRow(label: String, value: String, icon: String, color: Color) -> some View {
        HStack {
            Label(label, systemImage: icon)
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(color)
            Spacer()
            Text(value)
                .font(.system(size: 13, weight: .bold, design: .monospaced))
                .foregroundStyle(.white)
        }
    }

    // MARK: - Helpers

    private var weekRangeString: String {
        let calendar = Calendar.current
        let now = Date()
        let weekday = calendar.component(.weekday, from: now)
        let daysToSubtract = weekday - 2
        let startOfWeek = calendar.date(byAdding: .day, value: -max(0, daysToSubtract), to: now) ?? now
        let fmt = DateFormatter()
        fmt.dateFormat = "MMM d"
        return "\(fmt.string(from: startOfWeek)) – \(fmt.string(from: now))"
    }

    private func dayLabel(_ index: Int) -> String {
        ["M", "T", "W", "T", "F", "S", "S"][index % 7]
    }

    // MARK: - Load / Generate

    private func loadWeeklyStats() {
        weeklyStats = WeeklyStats(
            totalXP: UserDefaults.standard.integer(forKey: "hookbot_xp"),
            currentStreak: UserDefaults.standard.integer(forKey: "hookbot_streak"),
            totalDeploys: UserDefaults.standard.integer(forKey: "hookbot_deploys"),
            errors: UserDefaults.standard.integer(forKey: "hookbot_weekly_errors"),
            toolCalls: UserDefaults.standard.integer(forKey: "hookbot_weekly_tool_calls"),
            dailyActivity: loadDailyActivity(),
            achievements: loadAchievementBadges()
        )
    }

    private func loadDailyActivity() -> [Int] {
        (UserDefaults.standard.array(forKey: "hookbot_daily_activity") as? [Int]) ?? Array(repeating: 0, count: 7)
    }

    private func loadAchievementBadges() -> [String] {
        (UserDefaults.standard.array(forKey: "hookbot_achievement_badges") as? [String]) ?? []
    }

    private func generateAndShare() {
        let renderer = ImageRenderer(content: weeklyCard.environmentObject(engine))
        renderer.scale = 3.0
        if let uiImage = renderer.uiImage {
            cardImage = uiImage
            showShareSheet = true
        }
    }
}

// MARK: - Weekly Stats Model

struct WeeklyStats {
    let totalXP: Int
    let currentStreak: Int
    let totalDeploys: Int
    let errors: Int
    let toolCalls: Int
    let dailyActivity: [Int]
    let achievements: [String]

    var shareCaption: String {
        "🤖 Weekly Hookbot Report: \(totalXP) XP earned, \(currentStreak)-day streak, \(totalDeploys) deploys shipped! #hookbot #buildinpublic"
    }

    static var empty: WeeklyStats {
        WeeklyStats(
            totalXP: 0, currentStreak: 0, totalDeploys: 0,
            errors: 0, toolCalls: 0,
            dailyActivity: Array(repeating: 0, count: 7),
            achievements: []
        )
    }
}
