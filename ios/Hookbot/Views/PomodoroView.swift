import SwiftUI

// Pomodoro buddy — built-in timer, avatar cheers during work, relaxes during breaks (Phase 13.6)

// MARK: - Pomodoro State

enum PomodoroPhase: String {
    case idle     = "idle"
    case work     = "work"
    case shortBreak = "short_break"
    case longBreak  = "long_break"

    var displayName: String {
        switch self {
        case .idle:       return "Ready"
        case .work:       return "FOCUS"
        case .shortBreak: return "Short Break"
        case .longBreak:  return "Long Break"
        }
    }

    var emoji: String {
        switch self {
        case .idle:       return "🍅"
        case .work:       return "⚡"
        case .shortBreak: return "☕"
        case .longBreak:  return "🌿"
        }
    }

    var color: Color {
        switch self {
        case .idle:       return .gray
        case .work:       return .red
        case .shortBreak: return .green
        case .longBreak:  return .blue
        }
    }
}

// MARK: - View

struct PomodoroView: View {
    @EnvironmentObject var engine: AvatarEngine

    @State private var phase: PomodoroPhase = .idle
    @State private var secondsRemaining: Int = 25 * 60
    @State private var completedPomodoros = 0
    @State private var isRunning = false
    @State private var showSettings = false

    // Settings
    @State private var workMinutes = 25
    @State private var shortBreakMinutes = 5
    @State private var longBreakMinutes = 15

    private let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    var progress: Double {
        let total: Int
        switch phase {
        case .idle:       return 0
        case .work:       total = workMinutes * 60
        case .shortBreak: total = shortBreakMinutes * 60
        case .longBreak:  total = longBreakMinutes * 60
        }
        return 1.0 - Double(secondsRemaining) / Double(total)
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 28) {
                // Header
                Text("Pomodoro Buddy")
                    .font(.system(size: 22, weight: .black, design: .monospaced))
                    .foregroundStyle(.white)

                // Mini avatar reacting to phase
                miniAvatarSection

                // Timer ring
                timerRing

                // Controls
                controls

                // Session count
                sessionDots

                // Stats
                statsSection
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Pomodoro")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    showSettings.toggle()
                } label: {
                    Image(systemName: "gearshape")
                }
            }
        }
        .preferredColorScheme(.dark)
        .onReceive(timer) { _ in
            guard isRunning else { return }
            if secondsRemaining > 0 {
                secondsRemaining -= 1
            } else {
                phaseComplete()
            }
        }
        .sheet(isPresented: $showSettings) {
            PomodoroSettingsSheet(
                workMinutes: $workMinutes,
                shortBreakMinutes: $shortBreakMinutes,
                longBreakMinutes: $longBreakMinutes
            )
        }
    }

    // MARK: - Mini avatar

    private var miniAvatarSection: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 16)
                .fill(Color(white: 0.06))
                .frame(height: 120)

            HStack(spacing: 20) {
                AvatarView()
                    .environmentObject(engine)
                    .frame(width: 100, height: 80)

                VStack(alignment: .leading, spacing: 6) {
                    Text(avatarQuip)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(.white.opacity(0.8))
                        .lineLimit(4)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer()
            }
            .padding(.horizontal, 16)
        }
    }

    private var avatarQuip: String {
        switch phase {
        case .idle:
            return "Ready when you are. Let's crush it."
        case .work:
            isRunning
                ? "In the zone. Don't you dare open Twitter."
                : "Paused. Weakness detected."
        case .shortBreak:
            return "Good sprint. Rest for \(shortBreakMinutes) minutes. You've earned it."
        case .longBreak:
            return "That was serious work. Take \(longBreakMinutes) minutes. Walk around. I'll be here."
        }
    }

    // MARK: - Timer ring

    private var timerRing: some View {
        ZStack {
            Circle()
                .stroke(Color(white: 0.15), lineWidth: 12)
                .frame(width: 200, height: 200)

            Circle()
                .trim(from: 0, to: progress)
                .stroke(phase.color, style: StrokeStyle(lineWidth: 12, lineCap: .round))
                .frame(width: 200, height: 200)
                .rotationEffect(.degrees(-90))
                .animation(.linear(duration: 1), value: progress)

            VStack(spacing: 4) {
                Text(phase.emoji)
                    .font(.system(size: 36))
                Text(timeString)
                    .font(.system(size: 44, weight: .black, design: .monospaced))
                    .foregroundStyle(.white)
                Text(phase.displayName)
                    .font(.system(size: 13, weight: .bold, design: .monospaced))
                    .foregroundStyle(phase.color)
            }
        }
    }

    private var timeString: String {
        let m = secondsRemaining / 60
        let s = secondsRemaining % 60
        return String(format: "%02d:%02d", m, s)
    }

    // MARK: - Controls

    private var controls: some View {
        HStack(spacing: 24) {
            // Reset
            Button {
                resetTimer()
            } label: {
                Image(systemName: "arrow.counterclockwise")
                    .font(.system(size: 20))
                    .foregroundStyle(.gray)
                    .frame(width: 50, height: 50)
                    .background(Circle().fill(Color(white: 0.12)))
            }
            .buttonStyle(.plain)

            // Play / Pause
            Button {
                toggleTimer()
            } label: {
                Image(systemName: isRunning ? "pause.fill" : "play.fill")
                    .font(.system(size: 28))
                    .foregroundStyle(.white)
                    .frame(width: 72, height: 72)
                    .background(Circle().fill(phase == .idle ? Color.red : phase.color))
            }
            .buttonStyle(.plain)

            // Skip
            Button {
                skipPhase()
            } label: {
                Image(systemName: "forward.fill")
                    .font(.system(size: 20))
                    .foregroundStyle(.gray)
                    .frame(width: 50, height: 50)
                    .background(Circle().fill(Color(white: 0.12)))
            }
            .buttonStyle(.plain)
        }
    }

    // MARK: - Session dots

    private var sessionDots: some View {
        HStack(spacing: 8) {
            ForEach(0..<4, id: \.self) { i in
                Circle()
                    .fill(i < completedPomodoros % 4 ? Color.red : Color(white: 0.15))
                    .frame(width: 10, height: 10)
            }
        }
    }

    // MARK: - Stats

    private var statsSection: some View {
        HStack(spacing: 16) {
            statCard(
                value: "\(completedPomodoros)",
                label: "Sessions",
                color: .red
            )
            statCard(
                value: "\(completedPomodoros * workMinutes)m",
                label: "Focus Time",
                color: .orange
            )
            statCard(
                value: "\(UserDefaults.standard.integer(forKey: "hookbot_total_pomodoros"))",
                label: "All Time",
                color: .cyan
            )
        }
    }

    private func statCard(value: String, label: String, color: Color) -> some View {
        VStack(spacing: 4) {
            Text(value)
                .font(.system(size: 18, weight: .black, design: .monospaced))
                .foregroundStyle(color)
            Text(label)
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .frame(maxWidth: .infinity)
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.08)))
    }

    // MARK: - Logic

    private func toggleTimer() {
        if phase == .idle {
            phase = .work
            secondsRemaining = workMinutes * 60
            HapticManager.playEvent(.pomodoroStart)
        }
        isRunning.toggle()
        if isRunning {
            AvatarEvolutionEngine.shared.startSession()
        }
    }

    private func resetTimer() {
        isRunning = false
        phase = .idle
        secondsRemaining = workMinutes * 60
    }

    private func skipPhase() {
        phaseComplete()
    }

    private func phaseComplete() {
        isRunning = false
        HapticManager.playEvent(.pomodoroEnd)

        switch phase {
        case .idle:
            break
        case .work:
            completedPomodoros += 1
            let total = UserDefaults.standard.integer(forKey: "hookbot_total_pomodoros") + 1
            UserDefaults.standard.set(total, forKey: "hookbot_total_pomodoros")
            AvatarEvolutionEngine.shared.endSession()

            // Long break after 4 pomodoros
            if completedPomodoros % 4 == 0 {
                phase = .longBreak
                secondsRemaining = longBreakMinutes * 60
            } else {
                phase = .shortBreak
                secondsRemaining = shortBreakMinutes * 60
            }
            NotificationManager.shared.notifyPomodoroComplete(isWork: true)

        case .shortBreak, .longBreak:
            phase = .work
            secondsRemaining = workMinutes * 60
            NotificationManager.shared.notifyPomodoroComplete(isWork: false)
        }
    }
}

// MARK: - Settings Sheet

struct PomodoroSettingsSheet: View {
    @Binding var workMinutes: Int
    @Binding var shortBreakMinutes: Int
    @Binding var longBreakMinutes: Int
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section("Work Session") {
                    Stepper("\(workMinutes) minutes", value: $workMinutes, in: 5...60, step: 5)
                }
                Section("Short Break") {
                    Stepper("\(shortBreakMinutes) minutes", value: $shortBreakMinutes, in: 3...15, step: 1)
                }
                Section("Long Break") {
                    Stepper("\(longBreakMinutes) minutes", value: $longBreakMinutes, in: 10...30, step: 5)
                }
            }
            .navigationTitle("Pomodoro Settings")
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
