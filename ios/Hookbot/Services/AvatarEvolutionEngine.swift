import Foundation
import Combine

// Avatar evolution — your phone avatar evolves based on coding patterns (Phase 13.5)
// Types: speed demon, night owl, architect, test wizard — each with unique visual form

// MARK: - Evolution Type

enum EvolutionType: String, Codable, CaseIterable {
    case speedDemon   = "speed_demon"
    case nightOwl     = "night_owl"
    case architect    = "architect"
    case testWizard   = "test_wizard"
    case bugHunter    = "bug_hunter"
    case deployer     = "deployer"
    case allRounder   = "all_rounder"

    var displayName: String {
        switch self {
        case .speedDemon:  return "Speed Demon"
        case .nightOwl:    return "Night Owl"
        case .architect:   return "Architect"
        case .testWizard:  return "Test Wizard"
        case .bugHunter:   return "Bug Hunter"
        case .deployer:    return "Deployer"
        case .allRounder:  return "All-Rounder"
        }
    }

    var emoji: String {
        switch self {
        case .speedDemon:  return "⚡"
        case .nightOwl:    return "🦉"
        case .architect:   return "🏛️"
        case .testWizard:  return "🧙"
        case .bugHunter:   return "🐛"
        case .deployer:    return "🚀"
        case .allRounder:  return "🌟"
        }
    }

    var description: String {
        switch self {
        case .speedDemon:  return "Blazing fast tool calls and short sessions"
        case .nightOwl:    return "Most active between midnight and 4am"
        case .architect:   return "Long, focused deep work sessions"
        case .testWizard:  return "High ratio of test-writing to total work"
        case .bugHunter:   return "Frequently encounters and resolves errors"
        case .deployer:    return "Commits and deploys with high frequency"
        case .allRounder:  return "Balanced across all coding activities"
        }
    }

    // Unique visual traits for AvatarParams
    var paramBias: AvatarParamBias {
        switch self {
        case .speedDemon:  return AvatarParamBias(eyeOpenBias: 1.1, browAngleBias: -0.3, mouthCurveBias: 0.2)
        case .nightOwl:    return AvatarParamBias(eyeOpenBias: 0.5, browAngleBias: 0.3, mouthCurveBias: -0.1)
        case .architect:   return AvatarParamBias(eyeOpenBias: 0.8, browAngleBias: -0.8, mouthCurveBias: 0.0)
        case .testWizard:  return AvatarParamBias(eyeOpenBias: 0.9, browAngleBias: 0.5, mouthCurveBias: 0.3)
        case .bugHunter:   return AvatarParamBias(eyeOpenBias: 1.2, browAngleBias: -1.0, mouthCurveBias: -0.2)
        case .deployer:    return AvatarParamBias(eyeOpenBias: 0.7, browAngleBias: 0.2, mouthCurveBias: 0.5)
        case .allRounder:  return AvatarParamBias(eyeOpenBias: 0.9, browAngleBias: 0.0, mouthCurveBias: 0.1)
        }
    }
}

struct AvatarParamBias {
    var eyeOpenBias: Float
    var browAngleBias: Float
    var mouthCurveBias: Float
}

// MARK: - Coding Pattern Tracker

struct CodingPattern: Codable {
    var totalSessions: Int = 0
    var nightSessions: Int = 0         // 12am–4am
    var avgSessionMinutes: Float = 0
    var totalToolCalls: Int = 0
    var totalErrors: Int = 0
    var totalSuccesses: Int = 0
    var totalDeploys: Int = 0
    var testToolCalls: Int = 0         // Bash with test commands
    var currentStreak: Int = 0
    var totalXP: Int = 0
}

// MARK: - Engine

@MainActor
final class AvatarEvolutionEngine: ObservableObject {

    static let shared = AvatarEvolutionEngine()

    @Published private(set) var currentEvolution: EvolutionType = .allRounder
    @Published private(set) var evolutionProgress: Float = 0   // 0..1 toward next evolution
    @Published private(set) var pattern: CodingPattern = CodingPattern()
    @Published private(set) var isLeveledUp = false

    private var sessionStartTime: Date?

    private init() {
        loadPattern()
        recalculateEvolution()
    }

    // MARK: - Session tracking

    func startSession() {
        sessionStartTime = Date()
        let hour = Calendar.current.component(.hour, from: Date())
        if hour >= 0 && hour < 4 {
            pattern.nightSessions += 1
        }
        pattern.totalSessions += 1
        savePattern()
    }

    func endSession() {
        guard let start = sessionStartTime else { return }
        let durationMinutes = Float(Date().timeIntervalSince(start) / 60)
        pattern.avgSessionMinutes = (pattern.avgSessionMinutes * Float(pattern.totalSessions - 1) + durationMinutes) / Float(pattern.totalSessions)
        sessionStartTime = nil
        recalculateEvolution()
        savePattern()
    }

    func recordToolCall(_ toolName: String) {
        pattern.totalToolCalls += 1
        let testKeywords = ["test", "spec", "pytest", "jest", "xcode", "cargo test"]
        if testKeywords.contains(where: { toolName.lowercased().contains($0) }) {
            pattern.testToolCalls += 1
        }
        savePattern()
    }

    func recordSuccess() {
        pattern.totalSuccesses += 1
        let newXP = pattern.totalXP + 10
        let oldLevel = level(for: pattern.totalXP)
        let newLevel = level(for: newXP)
        pattern.totalXP = newXP
        UserDefaults.standard.set(newXP, forKey: "hookbot_xp")

        if newLevel > oldLevel {
            isLeveledUp = true
            HapticManager.playEvent(.levelUp)
            VoiceLinesManager.shared.playEventQuip("level_up")
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in
                self?.isLeveledUp = false
            }
        }
        savePattern()
    }

    func recordError() {
        pattern.totalErrors += 1
        savePattern()
    }

    func recordDeploy() {
        pattern.totalDeploys += 1
        let xp = UserDefaults.standard.integer(forKey: "hookbot_xp") + 50
        UserDefaults.standard.set(xp, forKey: "hookbot_xp")
        let deploys = UserDefaults.standard.integer(forKey: "hookbot_deploys") + 1
        UserDefaults.standard.set(deploys, forKey: "hookbot_deploys")
        savePattern()
    }

    func updateStreak(_ streak: Int) {
        pattern.currentStreak = streak
        UserDefaults.standard.set(streak, forKey: "hookbot_streak")
        savePattern()
    }

    // MARK: - Evolution calculation

    private func recalculateEvolution() {
        let t = pattern
        guard t.totalSessions > 0 else { return }

        let nightRatio = t.totalSessions > 0 ? Float(t.nightSessions) / Float(t.totalSessions) : 0
        let errorRatio = t.totalToolCalls > 0 ? Float(t.totalErrors) / Float(t.totalToolCalls) : 0
        let testRatio = t.totalToolCalls > 0 ? Float(t.testToolCalls) / Float(t.totalToolCalls) : 0
        let deployFrequency = t.totalSessions > 0 ? Float(t.totalDeploys) / Float(t.totalSessions) : 0

        var scores: [EvolutionType: Float] = [:]
        scores[.nightOwl]   = nightRatio
        scores[.bugHunter]  = errorRatio
        scores[.testWizard] = testRatio
        scores[.deployer]   = deployFrequency
        scores[.speedDemon] = t.avgSessionMinutes < 30 ? 0.8 : 0.2
        scores[.architect]  = t.avgSessionMinutes > 90 ? 0.8 : 0.2
        scores[.allRounder] = 0.3 // base fallback

        let best = scores.max(by: { $0.value < $1.value }) ?? (.allRounder, 0.3)
        let newEvolution = best.key

        if newEvolution != currentEvolution {
            currentEvolution = newEvolution
        }
        evolutionProgress = min(1.0, best.value)
    }

    // MARK: - XP Level

    func level(for xp: Int) -> Int { max(1, Int(sqrt(Double(xp) / 100.0)) + 1) }

    var currentLevel: Int { level(for: pattern.totalXP) }

    var xpToNextLevel: Int {
        let nextLevel = currentLevel + 1
        return nextLevel * nextLevel * 100 - pattern.totalXP
    }

    // MARK: - Persistence

    private func savePattern() {
        if let data = try? JSONEncoder().encode(pattern) {
            UserDefaults.standard.set(data, forKey: "hookbot_coding_pattern")
        }
    }

    private func loadPattern() {
        if let data = UserDefaults.standard.data(forKey: "hookbot_coding_pattern"),
           let decoded = try? JSONDecoder().decode(CodingPattern.self, from: data) {
            pattern = decoded
        }
        pattern.totalXP = UserDefaults.standard.integer(forKey: "hookbot_xp")
        pattern.currentStreak = UserDefaults.standard.integer(forKey: "hookbot_streak")
    }
}
