import SwiftUI

// Achievement showcase — badges, rare items, longest streak, total XP — shareable (Phase 13.5)

// MARK: - Achievement

struct Achievement: Identifiable, Codable {
    let id: String
    let name: String
    let description: String
    let icon: String
    let category: Category
    let rarity: Rarity
    var isUnlocked: Bool
    var unlockedAt: Date?
    var progress: Double   // 0..1
    var progressDescription: String

    enum Category: String, Codable, CaseIterable {
        case streak     = "Streak"
        case xp         = "XP"
        case social     = "Social"
        case coding     = "Coding"
        case health     = "Health"
    }

    enum Rarity: String, Codable {
        case bronze   = "Bronze"
        case silver   = "Silver"
        case gold     = "Gold"
        case platinum = "Platinum"
        case diamond  = "Diamond"

        var color: Color {
            switch self {
            case .bronze:   return Color(red: 0.8, green: 0.5, blue: 0.2)
            case .silver:   return Color(white: 0.7)
            case .gold:     return Color(red: 1.0, green: 0.85, blue: 0.0)
            case .platinum: return Color(red: 0.6, green: 0.9, blue: 1.0)
            case .diamond:  return Color(red: 0.7, green: 0.9, blue: 1.0)
            }
        }
    }
}

// MARK: - View

struct AchievementShowcaseView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var achievements: [Achievement] = Achievement.allAchievements()
    @State private var selectedCategory: Achievement.Category? = nil
    @State private var showShareSheet = false
    @State private var shareImage: UIImage?

    var filtered: [Achievement] {
        if let cat = selectedCategory {
            return achievements.filter { $0.category == cat }
        }
        return achievements
    }

    var unlocked: [Achievement] { filtered.filter { $0.isUnlocked } }
    var locked: [Achievement] { filtered.filter { !$0.isUnlocked } }

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Stats header
                statsHeader

                // Category filter
                categoryFilter

                // Unlocked achievements
                if !unlocked.isEmpty {
                    achievementsSection(title: "UNLOCKED (\(unlocked.count))", items: unlocked)
                }

                // In-progress locked
                if !locked.isEmpty {
                    achievementsSection(title: "IN PROGRESS", items: locked)
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Achievements")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    generateShareImage()
                } label: {
                    Image(systemName: "square.and.arrow.up")
                }
            }
        }
        .preferredColorScheme(.dark)
        .onAppear { loadProgress() }
        .sheet(isPresented: $showShareSheet) {
            if let image = shareImage {
                ShareSheet(items: [image, "Check out my Hookbot achievements! 🤖 #hookbot"])
            }
        }
    }

    // MARK: - Stats header

    private var statsHeader: some View {
        HStack(spacing: 12) {
            statBlock(
                value: "\(unlocked.count)/\(achievements.count)",
                label: "Badges",
                icon: "rosette",
                color: .yellow
            )
            statBlock(
                value: "\(UserDefaults.standard.integer(forKey: "hookbot_xp"))",
                label: "Total XP",
                icon: "star.fill",
                color: .orange
            )
            statBlock(
                value: "\(UserDefaults.standard.integer(forKey: "hookbot_streak"))d",
                label: "Best Streak",
                icon: "flame.fill",
                color: .red
            )
        }
    }

    private func statBlock(value: String, label: String, icon: String, color: Color) -> some View {
        VStack(spacing: 6) {
            Image(systemName: icon).foregroundStyle(color).font(.system(size: 18))
            Text(value)
                .font(.system(size: 18, weight: .black, design: .monospaced))
                .foregroundStyle(.white)
                .minimumScaleFactor(0.6)
                .lineLimit(1)
            Text(label)
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .frame(maxWidth: .infinity)
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - Category filter

    private var categoryFilter: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                filterChip(label: "All", isSelected: selectedCategory == nil) {
                    selectedCategory = nil
                }
                ForEach(Achievement.Category.allCases, id: \.self) { cat in
                    filterChip(label: cat.rawValue, isSelected: selectedCategory == cat) {
                        selectedCategory = selectedCategory == cat ? nil : cat
                    }
                }
            }
            .padding(.horizontal, 2)
        }
    }

    private func filterChip(label: String, isSelected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(label)
                .font(.system(size: 12, weight: .bold, design: .monospaced))
                .foregroundStyle(isSelected ? .black : .gray)
                .padding(.horizontal, 14)
                .padding(.vertical, 7)
                .background(Capsule().fill(isSelected ? Color.cyan : Color(white: 0.12)))
        }
        .buttonStyle(.plain)
    }

    // MARK: - Achievement section

    private func achievementsSection(title: String, items: [Achievement]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)

            LazyVStack(spacing: 8) {
                ForEach(items) { achievement in
                    AchievementRow(achievement: achievement)
                }
            }
        }
    }

    // MARK: - Share image generation

    private func generateShareImage() {
        let topUnlocked = Array(unlocked.prefix(6))
        let content = AchievementShareCard(achievements: topUnlocked, engine: engine)
        let renderer = ImageRenderer(content: content)
        renderer.scale = 3.0
        shareImage = renderer.uiImage
        showShareSheet = true
    }

    // MARK: - Load progress from UserDefaults / server

    private func loadProgress() {
        let xp = UserDefaults.standard.integer(forKey: "hookbot_xp")
        let streak = UserDefaults.standard.integer(forKey: "hookbot_streak")
        let friends = UserDefaults.standard.integer(forKey: "hookbot_friend_count")
        let deploys = UserDefaults.standard.integer(forKey: "hookbot_deploys")
        let sessions = UserDefaults.standard.integer(forKey: "hookbot_sessions")

        achievements = achievements.map { achievement in
            var updated = achievement
            switch achievement.id {
            case "first_session":
                updated.progress = sessions > 0 ? 1 : 0
                updated.isUnlocked = sessions > 0
            case "streak_7":
                updated.progress = min(1, Double(streak) / 7)
                updated.isUnlocked = streak >= 7
            case "streak_30":
                updated.progress = min(1, Double(streak) / 30)
                updated.isUnlocked = streak >= 30
            case "streak_100":
                updated.progress = min(1, Double(streak) / 100)
                updated.isUnlocked = streak >= 100
            case "xp_1000":
                updated.progress = min(1, Double(xp) / 1000)
                updated.isUnlocked = xp >= 1000
            case "xp_10000":
                updated.progress = min(1, Double(xp) / 10000)
                updated.isUnlocked = xp >= 10000
            case "first_friend":
                updated.progress = friends > 0 ? 1 : 0
                updated.isUnlocked = friends > 0
            case "first_deploy":
                updated.progress = deploys > 0 ? 1 : 0
                updated.isUnlocked = deploys > 0
            case "ten_deploys":
                updated.progress = min(1, Double(deploys) / 10)
                updated.isUnlocked = deploys >= 10
            default: break
            }
            return updated
        }
    }
}

// MARK: - Achievement Row

struct AchievementRow: View {
    let achievement: Achievement

    var body: some View {
        HStack(spacing: 14) {
            ZStack {
                Circle()
                    .fill(achievement.isUnlocked ? achievement.rarity.color.opacity(0.2) : Color(white: 0.1))
                    .frame(width: 52, height: 52)
                Text(achievement.icon)
                    .font(.system(size: 26))
                    .opacity(achievement.isUnlocked ? 1 : 0.3)
            }

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(achievement.name)
                        .font(.system(size: 14, weight: .bold, design: .monospaced))
                        .foregroundStyle(achievement.isUnlocked ? .white : .gray)
                    Spacer()
                    Text(achievement.rarity.rawValue)
                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                        .foregroundStyle(achievement.rarity.color)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Capsule().fill(achievement.rarity.color.opacity(0.15)))
                }

                Text(achievement.description)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.gray)
                    .lineLimit(2)

                if !achievement.isUnlocked && achievement.progress > 0 {
                    HStack(spacing: 6) {
                        GeometryReader { geo in
                            ZStack(alignment: .leading) {
                                RoundedRectangle(cornerRadius: 3).fill(Color(white: 0.15)).frame(height: 4)
                                RoundedRectangle(cornerRadius: 3).fill(Color.cyan)
                                    .frame(width: geo.size.width * achievement.progress, height: 4)
                            }
                        }
                        .frame(height: 4)
                        Text(achievement.progressDescription)
                            .font(.system(size: 9, design: .monospaced))
                            .foregroundStyle(.gray)
                    }
                } else if achievement.isUnlocked, let date = achievement.unlockedAt {
                    Text("Unlocked \(relativeDate(date))")
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(achievement.rarity.color.opacity(0.7))
                }
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(Color(white: achievement.isUnlocked ? 0.09 : 0.06))
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(achievement.isUnlocked ? achievement.rarity.color.opacity(0.3) : Color.clear, lineWidth: 1)
                )
        )
    }

    private func relativeDate(_ date: Date) -> String {
        let secs = Date().timeIntervalSince(date)
        if secs < 86400 { return "today" }
        if secs < 604800 { return "\(Int(secs / 86400))d ago" }
        let fmt = DateFormatter()
        fmt.dateStyle = .short
        return fmt.string(from: date)
    }
}

// MARK: - Share Card

struct AchievementShareCard: View {
    let achievements: [Achievement]
    let engine: AvatarEngine

    var body: some View {
        VStack(spacing: 16) {
            Text("🏆 My Hookbot Achievements")
                .font(.system(size: 18, weight: .black, design: .monospaced))
                .foregroundStyle(.white)

            LazyVGrid(columns: [.init(.flexible()), .init(.flexible()), .init(.flexible())], spacing: 10) {
                ForEach(achievements) { achievement in
                    VStack(spacing: 4) {
                        Text(achievement.icon).font(.system(size: 28))
                        Text(achievement.name)
                            .font(.system(size: 8, weight: .bold, design: .monospaced))
                            .foregroundStyle(achievement.rarity.color)
                            .lineLimit(1)
                    }
                }
            }
        }
        .padding(20)
        .background(Color(red: 0.05, green: 0.02, blue: 0.15))
    }
}

// MARK: - Static achievement definitions

extension Achievement {
    static func allAchievements() -> [Achievement] {
        [
            Achievement(id: "first_session",  name: "First Boot",      description: "Open the Hookbot app for the first time",         icon: "🤖", category: .coding,  rarity: .bronze,   isUnlocked: false, progress: 0, progressDescription: "0/1"),
            Achievement(id: "streak_7",       name: "Week Warrior",    description: "Maintain a 7-day coding streak",                  icon: "🔥", category: .streak,  rarity: .silver,   isUnlocked: false, progress: 0, progressDescription: "0/7 days"),
            Achievement(id: "streak_30",      name: "Month Master",    description: "Maintain a 30-day coding streak",                 icon: "📅", category: .streak,  rarity: .gold,     isUnlocked: false, progress: 0, progressDescription: "0/30 days"),
            Achievement(id: "streak_100",     name: "Century Coder",   description: "100-day coding streak — legendary status",        icon: "👑", category: .streak,  rarity: .diamond,  isUnlocked: false, progress: 0, progressDescription: "0/100 days"),
            Achievement(id: "xp_1000",        name: "XP Initiate",     description: "Earn 1,000 total XP",                            icon: "⭐", category: .xp,      rarity: .bronze,   isUnlocked: false, progress: 0, progressDescription: "0/1,000 XP"),
            Achievement(id: "xp_10000",       name: "XP Veteran",      description: "Earn 10,000 total XP",                           icon: "🌟", category: .xp,      rarity: .gold,     isUnlocked: false, progress: 0, progressDescription: "0/10,000 XP"),
            Achievement(id: "first_friend",   name: "Buddy System",    description: "Pair with your first friend",                    icon: "👥", category: .social,  rarity: .silver,   isUnlocked: false, progress: 0, progressDescription: "0/1 friends"),
            Achievement(id: "first_deploy",   name: "Ship It",         description: "Record your first successful deploy",            icon: "🚀", category: .coding,  rarity: .bronze,   isUnlocked: false, progress: 0, progressDescription: "0/1 deploys"),
            Achievement(id: "ten_deploys",    name: "Deploy God",      description: "Complete 10 successful deploys",                 icon: "💫", category: .coding,  rarity: .gold,     isUnlocked: false, progress: 0, progressDescription: "0/10 deploys"),
            Achievement(id: "pomodoro_10",    name: "Tomato Timer",    description: "Complete 10 Pomodoro sessions",                  icon: "🍅", category: .health,  rarity: .silver,   isUnlocked: false, progress: 0, progressDescription: "0/10 sessions"),
            Achievement(id: "night_owl",      name: "Night Owl",       description: "Code between midnight and 4am five times",       icon: "🦉", category: .coding,  rarity: .silver,   isUnlocked: false, progress: 0, progressDescription: "0/5 nights"),
            Achievement(id: "social_raid",    name: "Raider",          description: "Send an avatar raid to a friend",               icon: "⚡", category: .social,  rarity: .bronze,   isUnlocked: false, progress: 0, progressDescription: "0/1 raids"),
        ]
    }
}
