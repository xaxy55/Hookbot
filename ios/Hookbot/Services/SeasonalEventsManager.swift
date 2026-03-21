import Foundation
import SwiftUI

// Seasonal avatar events — time-limited transformations (Phase 13.5)
// Spooky October, cozy December, glitchy April Fools, etc.

// MARK: - Season

enum Season: String, Codable {
    case halloween   = "halloween"
    case christmas   = "christmas"
    case aprilFools  = "april_fools"
    case summerHeat  = "summer_heat"
    case newYear     = "new_year"
    case none        = "none"

    static var current: Season {
        let cal = Calendar.current
        let now = Date()
        let month = cal.component(.month, from: now)
        let day = cal.component(.day, from: now)

        switch (month, day) {
        case (10, _):                  return .halloween
        case (12, 1...26):             return .christmas
        case (12, 27...31), (1, 1...3): return .newYear
        case (4, 1):                   return .aprilFools
        case (7, _), (8, _):           return .summerHeat
        default:                       return .none
        }
    }

    var displayName: String {
        switch self {
        case .halloween:  return "Spooky Season 🎃"
        case .christmas:  return "Cozy December ❄️"
        case .newYear:    return "New Year 🎆"
        case .aprilFools: return "Glitch Day 👾"
        case .summerHeat: return "Summer Heat ☀️"
        case .none:       return "No Active Event"
        }
    }

    var themeColor: Color {
        switch self {
        case .halloween:  return Color(red: 1.0, green: 0.4, blue: 0)
        case .christmas:  return Color(red: 0.8, green: 0.1, blue: 0.1)
        case .newYear:    return Color(red: 1.0, green: 0.85, blue: 0)
        case .aprilFools: return Color(red: 0, green: 0.9, blue: 0.4)
        case .summerHeat: return Color(red: 1.0, green: 0.7, blue: 0.1)
        case .none:       return Color.blue
        }
    }

    var description: String {
        switch self {
        case .halloween:
            return "Your avatar wears a spooky costume and has orange eyes. Exclusive accessories unlocked!"
        case .christmas:
            return "Cozy hat and snowflake particles. Warm glow on the avatar. Limited holiday items available."
        case .newYear:
            return "Confetti effects and firework accessories. Countdown timer on the avatar."
        case .aprilFools:
            return "Avatar randomly glitches, shows wrong states, and speaks backwards. Very unstable."
        case .summerHeat:
            return "Sun hat accessory, warm yellow glow, and occasional sweat drops."
        case .none:
            return "No seasonal event is currently active."
        }
    }

    var exclusiveItems: [String] {
        switch self {
        case .halloween:  return ["🎃 Pumpkin Hat", "🕷️ Spider Web", "🦇 Bat Wings"]
        case .christmas:  return ["🎄 Santa Hat", "⛄ Snowflake Aura", "🎁 Gift Bow"]
        case .newYear:    return ["🎆 Confetti Crown", "🥂 Champagne Glass", "✨ Star Trail"]
        case .aprilFools: return ["👾 Glitch Skin", "🃏 Chaos Emote", "🌀 Warp Effect"]
        case .summerHeat: return ["🌞 Sun Hat", "🍦 Ice Cream Prop", "🌊 Wave Effect"]
        case .none:       return []
        }
    }

    // Modified avatar params for this season
    var paramOverride: AvatarSeasonalParams {
        switch self {
        case .halloween:
            return AvatarSeasonalParams(eyeColorHue: 0.08, glowColor: Color(red: 1, green: 0.4, blue: 0).opacity(0.5))
        case .christmas:
            return AvatarSeasonalParams(eyeColorHue: 0.6, glowColor: Color(red: 0.8, green: 0.1, blue: 0.1).opacity(0.4))
        case .newYear:
            return AvatarSeasonalParams(eyeColorHue: 0.15, glowColor: Color.yellow.opacity(0.5))
        case .aprilFools:
            return AvatarSeasonalParams(eyeColorHue: 0.35, glowColor: Color.green.opacity(0.4), isGlitching: true)
        case .summerHeat:
            return AvatarSeasonalParams(eyeColorHue: 0.1, glowColor: Color.yellow.opacity(0.3))
        case .none:
            return AvatarSeasonalParams(eyeColorHue: -1, glowColor: .clear)
        }
    }
}

struct AvatarSeasonalParams {
    var eyeColorHue: Float    // -1 = no override
    var glowColor: Color
    var isGlitching: Bool = false
}

// MARK: - Manager

@MainActor
final class SeasonalEventsManager: ObservableObject {

    static let shared = SeasonalEventsManager()

    @Published private(set) var activeSeason: Season = .none
    @Published private(set) var unlockedSeasonalItems: [String] = []

    private init() {
        activeSeason = Season.current
        loadUnlockedItems()
        checkForNewUnlocks()
    }

    func refresh() {
        activeSeason = Season.current
        checkForNewUnlocks()
    }

    var isEventActive: Bool { activeSeason != .none }

    // MARK: - Unlock items

    private func checkForNewUnlocks() {
        guard activeSeason != .none else { return }
        for item in activeSeason.exclusiveItems {
            if !unlockedSeasonalItems.contains(item) {
                unlockedSeasonalItems.append(item)
            }
        }
        saveUnlockedItems()
    }

    private func saveUnlockedItems() {
        UserDefaults.standard.set(unlockedSeasonalItems, forKey: "hookbot_seasonal_items")
    }

    private func loadUnlockedItems() {
        unlockedSeasonalItems = UserDefaults.standard.stringArray(forKey: "hookbot_seasonal_items") ?? []
    }

    // MARK: - Glitch effect (April Fools)

    func shouldGlitch(for state: AvatarState) -> AvatarState {
        guard activeSeason == .aprilFools else { return state }
        // 10% chance to show wrong state
        if Int.random(in: 0..<10) == 0 {
            return AvatarState.allCases.randomElement() ?? state
        }
        return state
    }
}

// MARK: - Season Banner View

struct SeasonalBannerView: View {
    let season: Season

    var body: some View {
        HStack(spacing: 12) {
            Text(season.displayName.prefix(2).description)
                .font(.system(size: 28))
            VStack(alignment: .leading, spacing: 2) {
                Text(season.displayName)
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)
                Text("Limited event active!")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.white.opacity(0.7))
            }
            Spacer()
            Image(systemName: "sparkles")
                .foregroundStyle(.white)
        }
        .padding(14)
        .background(
            RoundedRectangle(cornerRadius: 14)
                .fill(season.themeColor.opacity(0.25))
                .overlay(RoundedRectangle(cornerRadius: 14).stroke(season.themeColor.opacity(0.6), lineWidth: 1))
        )
    }
}
