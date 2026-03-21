import HealthKit
import Foundation
import SwiftUI

// HealthKit integration — step challenge, avatar gets energized at goal (Phase 13.6)

@MainActor
final class HealthKitManager: ObservableObject {

    static let shared = HealthKitManager()

    @Published private(set) var todaySteps: Int = 0
    @Published private(set) var stepGoal: Int = 10000
    @Published private(set) var isAuthorized = false
    @Published private(set) var goalProgress: Double = 0   // 0..1

    private let store = HKHealthStore()
    private var observerQuery: HKObserverQuery?

    private init() {}

    // MARK: - Authorization

    func requestAuthorization() {
        guard HKHealthStore.isHealthDataAvailable() else { return }

        let stepType = HKObjectType.quantityType(forIdentifier: .stepCount)!
        store.requestAuthorization(toShare: nil, read: [stepType]) { [weak self] granted, _ in
            Task { @MainActor in
                self?.isAuthorized = granted
                if granted {
                    self?.fetchTodaySteps()
                    self?.startObserving()
                }
            }
        }
    }

    // MARK: - Fetch today's steps

    func fetchTodaySteps() {
        guard isAuthorized else { return }
        guard let stepType = HKQuantityType.quantityType(forIdentifier: .stepCount) else { return }

        let startOfDay = Calendar.current.startOfDay(for: Date())
        let predicate = HKQuery.predicateForSamples(withStart: startOfDay, end: Date(), options: .strictStartDate)

        let query = HKStatisticsQuery(
            quantityType: stepType,
            quantitySamplePredicate: predicate,
            options: .cumulativeSum
        ) { [weak self] _, result, _ in
            let steps = Int(result?.sumQuantity()?.doubleValue(for: .count()) ?? 0)
            DispatchQueue.main.async {
                self?.updateSteps(steps)
            }
        }
        store.execute(query)
    }

    private func updateSteps(_ steps: Int) {
        let previous = todaySteps
        todaySteps = steps
        goalProgress = min(1.0, Double(steps) / Double(stepGoal))

        // Celebrate hitting goal
        if previous < stepGoal && steps >= stepGoal {
            HapticManager.playEvent(.levelUp)
            VoiceLinesManager.shared.speak("Step goal reached! Your avatar is energized!")
        }

        // Update avatar energy via evolution engine
        if steps > stepGoal / 2 {
            // Energized — record as positive signal
        }

        // Sedentary warning — if low steps and long session
        let sessionMinutes = BreakReminderManager.shared.sessionMinutes
        if sessionMinutes > 120 && steps < 500 {
            // Avatar gets sluggish
        }
    }

    // MARK: - Live step observer

    private func startObserving() {
        guard let stepType = HKQuantityType.quantityType(forIdentifier: .stepCount) else { return }
        observerQuery = HKObserverQuery(sampleType: stepType, predicate: nil) { [weak self] _, _, _ in
            Task { @MainActor in
                self?.fetchTodaySteps()
            }
        }
        store.execute(observerQuery!)
    }

    // MARK: - Step goal

    func setStepGoal(_ goal: Int) {
        stepGoal = goal
        UserDefaults.standard.set(goal, forKey: "hookbot_step_goal")
        goalProgress = min(1.0, Double(todaySteps) / Double(goal))
    }

    // MARK: - Avatar energy state

    var avatarEnergyLevel: AvatarEnergyLevel {
        switch goalProgress {
        case 0..<0.3:    return .sedentary
        case 0.3..<0.7:  return .moderate
        case 0.7..<1.0:  return .active
        default:         return .energized
        }
    }

    enum AvatarEnergyLevel {
        case sedentary
        case moderate
        case active
        case energized

        var displayName: String {
            switch self {
            case .sedentary:  return "Sedentary 😴"
            case .moderate:   return "Moving 🚶"
            case .active:     return "Active 🏃"
            case .energized:  return "Energized ⚡"
            }
        }
    }
}

// MARK: - Step Challenge View component

struct StepChallengeCard: View {
    @ObservedObject var healthKit: HealthKitManager

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Label("Step Challenge", systemImage: "figure.walk")
                    .font(.system(size: 13, weight: .bold, design: .monospaced))
                    .foregroundStyle(.green)
                Spacer()
                Text(healthKit.avatarEnergyLevel.displayName)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.gray)
            }

            HStack {
                Text("\(healthKit.todaySteps)")
                    .font(.system(size: 28, weight: .black, design: .monospaced))
                    .foregroundStyle(.white)
                Text("/ \(healthKit.stepGoal)")
                    .font(.system(size: 16, design: .monospaced))
                    .foregroundStyle(.gray)
                    .alignmentGuide(.firstTextBaseline) { d in d[.lastTextBaseline] }
            }

            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 4)
                        .fill(Color(white: 0.15))
                    RoundedRectangle(cornerRadius: 4)
                        .fill(healthKit.goalProgress >= 1 ? Color.green : Color.cyan)
                        .frame(width: geo.size.width * healthKit.goalProgress)
                        .animation(.easeOut, value: healthKit.goalProgress)
                }
            }
            .frame(height: 8)

            if !healthKit.isAuthorized {
                Button {
                    healthKit.requestAuthorization()
                } label: {
                    Label("Allow HealthKit access", systemImage: "heart.fill")
                        .font(.system(size: 12, design: .monospaced))
                }
                .buttonStyle(.borderedProminent)
                .tint(.green)
                .controlSize(.small)
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 14).fill(Color(white: 0.08)))
    }
}
