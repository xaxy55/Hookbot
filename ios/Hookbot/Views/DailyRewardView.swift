import SwiftUI

// Daily login reward — open app daily for XP boosts and rare loot (Phase 13.5)

// MARK: - Reward Model

struct DailyReward: Identifiable {
    let id = UUID()
    let day: Int
    let xp: Int
    let rarity: Rarity
    let item: String     // emoji or name
    let description: String
    var claimed: Bool

    enum Rarity: String {
        case common    = "Common"
        case uncommon  = "Uncommon"
        case rare      = "Rare"
        case epic      = "Epic"
        case legendary = "Legendary"

        var color: Color {
            switch self {
            case .common:    return .gray
            case .uncommon:  return .green
            case .rare:      return .blue
            case .epic:      return .purple
            case .legendary: return .yellow
            }
        }

        var glowColor: Color { color.opacity(0.4) }
    }
}

// MARK: - View

struct DailyRewardView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var rewards: [DailyReward] = DailyRewardView.generateWeek()
    @State private var canClaim: Bool = false
    @State private var claimedToday: DailyReward?
    @State private var showClaimAnimation = false
    @State private var claimedXP = 0

    var currentDay: Int {
        rewards.firstIndex(where: { !$0.claimed }) ?? 6
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Header
                header

                // Claim animation
                if showClaimAnimation, let reward = claimedToday {
                    claimAnimation(reward: reward)
                        .transition(.scale.combined(with: .opacity))
                }

                // Weekly reward strip
                weeklyStrip

                // Streak info
                streakCard

                // Claim button
                if canClaim {
                    claimButton
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Daily Reward")
        .navigationBarTitleDisplayMode(.inline)
        .preferredColorScheme(.dark)
        .onAppear { checkClaimStatus() }
    }

    // MARK: - Header

    private var header: some View {
        VStack(spacing: 8) {
            Text("🎁")
                .font(.system(size: 60))
            Text("Daily Reward")
                .font(.system(size: 24, weight: .black, design: .monospaced))
                .foregroundStyle(.white)
            Text("Come back every day to increase rarity")
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
        }
    }

    // MARK: - Claim animation

    private func claimAnimation(reward: DailyReward) -> some View {
        ZStack {
            RoundedRectangle(cornerRadius: 20)
                .fill(reward.rarity.glowColor)
                .blur(radius: 20)
            VStack(spacing: 12) {
                Text(reward.item)
                    .font(.system(size: 60))
                Text("+\(reward.xp) XP")
                    .font(.system(size: 28, weight: .black, design: .monospaced))
                    .foregroundStyle(.yellow)
                Text(reward.rarity.rawValue.uppercased())
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                    .foregroundStyle(reward.rarity.color)
                Text(reward.description)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(.white.opacity(0.8))
                    .multilineTextAlignment(.center)
            }
            .padding(24)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 20)
    }

    // MARK: - Weekly reward strip

    private var weeklyStrip: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("THIS WEEK")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 10) {
                    ForEach(rewards) { reward in
                        RewardDayCell(
                            reward: reward,
                            isCurrent: rewards.firstIndex(where: { $0.id == reward.id }) == currentDay
                        )
                    }
                }
                .padding(.horizontal, 2)
            }
        }
    }

    // MARK: - Streak card

    private var streakCard: some View {
        let streak = UserDefaults.standard.integer(forKey: "hookbot_streak")
        return HStack(spacing: 16) {
            Image(systemName: "flame.fill")
                .font(.system(size: 32))
                .foregroundStyle(.orange)
            VStack(alignment: .leading, spacing: 4) {
                Text("\(streak)-day streak")
                    .font(.system(size: 18, weight: .black, design: .monospaced))
                    .foregroundStyle(.white)
                Text(streak > 6 ? "Rare drops unlocked!" : "Keep going for better rewards")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.gray)
            }
            Spacer()
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 14).fill(Color(white: 0.1)))
    }

    // MARK: - Claim button

    private var claimButton: some View {
        let reward = rewards[safe: currentDay] ?? rewards[0]
        return VStack(spacing: 12) {
            HStack {
                Text("TODAY'S REWARD")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundStyle(.gray)
                Spacer()
                Text(reward.rarity.rawValue.uppercased())
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundStyle(reward.rarity.color)
            }

            Button {
                claimReward()
            } label: {
                HStack(spacing: 12) {
                    Text(reward.item)
                        .font(.system(size: 28))
                    VStack(alignment: .leading, spacing: 3) {
                        Text("Claim Reward")
                            .font(.system(size: 16, weight: .bold, design: .monospaced))
                            .foregroundStyle(.black)
                        Text("+\(reward.xp) XP · \(reward.description)")
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(.black.opacity(0.7))
                    }
                    Spacer()
                }
                .padding(16)
                .frame(maxWidth: .infinity)
                .background(reward.rarity.color)
                .clipShape(RoundedRectangle(cornerRadius: 14))
            }
            .buttonStyle(.plain)
        }
    }

    // MARK: - Logic

    private func checkClaimStatus() {
        let lastClaim = UserDefaults.standard.object(forKey: "hookbot_last_claim") as? Date
        if let lastClaim {
            canClaim = !Calendar.current.isDateInToday(lastClaim)
        } else {
            canClaim = true
        }

        // Mark past rewards as claimed
        let streak = UserDefaults.standard.integer(forKey: "hookbot_streak")
        rewards = rewards.enumerated().map { i, reward in
            var updated = reward
            updated.claimed = i < streak % 7
            return updated
        }
    }

    private func claimReward() {
        guard canClaim else { return }
        let reward = rewards[safe: currentDay] ?? rewards[0]

        // Update state
        var updatedRewards = rewards
        updatedRewards[currentDay].claimed = true
        rewards = updatedRewards

        claimedToday = reward
        claimedXP = reward.xp
        canClaim = false

        // Save
        UserDefaults.standard.set(Date(), forKey: "hookbot_last_claim")
        let currentXP = UserDefaults.standard.integer(forKey: "hookbot_xp")
        UserDefaults.standard.set(currentXP + reward.xp, forKey: "hookbot_xp")
        let currentStreak = UserDefaults.standard.integer(forKey: "hookbot_streak")
        UserDefaults.standard.set(currentStreak + 1, forKey: "hookbot_streak")

        // Animate
        HapticManager.playEvent(.dailyReward)
        VoiceLinesManager.shared.playEventQuip("daily_reward")

        withAnimation(.spring()) {
            showClaimAnimation = true
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
            withAnimation { showClaimAnimation = false }
        }
    }

    // MARK: - Generate week

    static func generateWeek() -> [DailyReward] {
        [
            DailyReward(day: 1, xp: 50,   rarity: .common,    item: "⚡", description: "XP Boost",          claimed: false),
            DailyReward(day: 2, xp: 100,  rarity: .common,    item: "🔧", description: "Tool Badge",         claimed: false),
            DailyReward(day: 3, xp: 150,  rarity: .uncommon,  item: "🎩", description: "Hat Accessory",      claimed: false),
            DailyReward(day: 4, xp: 200,  rarity: .uncommon,  item: "🌟", description: "Star Token",         claimed: false),
            DailyReward(day: 5, xp: 300,  rarity: .rare,      item: "💎", description: "Rare Gem",           claimed: false),
            DailyReward(day: 6, xp: 500,  rarity: .epic,      item: "🚀", description: "Rocket Boost",       claimed: false),
            DailyReward(day: 7, xp: 1000, rarity: .legendary, item: "👑", description: "Legendary Crown",    claimed: false),
        ]
    }
}

// MARK: - Reward Day Cell

struct RewardDayCell: View {
    let reward: DailyReward
    let isCurrent: Bool

    var body: some View {
        VStack(spacing: 6) {
            ZStack {
                Circle()
                    .fill(
                        isCurrent ? reward.rarity.color.opacity(0.3) :
                        reward.claimed ? Color(white: 0.2) : Color(white: 0.08)
                    )
                    .frame(width: 56, height: 56)
                    .overlay(
                        Circle()
                            .stroke(isCurrent ? reward.rarity.color : Color.clear, lineWidth: 2)
                    )

                Text(reward.claimed ? "✓" : reward.item)
                    .font(.system(size: reward.claimed ? 20 : 28))
                    .opacity(reward.claimed ? 0.5 : 1)
            }

            Text("Day \(reward.day)")
                .font(.system(size: 9, weight: .bold, design: .monospaced))
                .foregroundStyle(isCurrent ? reward.rarity.color : .gray)

            Text("+\(reward.xp)")
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(.gray)
        }
    }
}

// MARK: - Safe subscript

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
