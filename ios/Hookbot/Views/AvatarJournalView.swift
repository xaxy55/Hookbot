import SwiftUI

// Avatar journal — your bot's diary of your coding day (Phase 13.3)
// "Today our human wrote 47 functions. I'm exhausted."

// MARK: - Journal Entry

struct JournalEntry: Codable, Identifiable {
    let id: String
    let date: Date
    let entry: String
    let stats: JournalStats
    let mood: String     // matches AvatarState.rawValue
}

struct JournalStats: Codable {
    let toolCalls: Int
    let stateChanges: Int
    let errors: Int
    let successes: Int
    let sessionDuration: Int   // minutes
}

// MARK: - View

struct AvatarJournalView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var entries: [JournalEntry] = []
    @State private var todayEntry: JournalEntry?
    @State private var isLoading = false

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Today's entry header
                todayHeader

                // Today's entry
                if let entry = todayEntry {
                    JournalEntryCard(entry: entry, isToday: true)
                }

                if !entries.isEmpty {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("PREVIOUS ENTRIES")
                            .font(.system(size: 11, weight: .bold, design: .monospaced))
                            .foregroundStyle(.gray)
                            .padding(.horizontal)

                        ForEach(entries) { entry in
                            JournalEntryCard(entry: entry, isToday: false)
                                .padding(.horizontal)
                        }
                    }
                }

                if entries.isEmpty && todayEntry == nil && !isLoading {
                    emptyState
                }
            }
            .padding(.vertical)
        }
        .background(Color.black)
        .navigationTitle("Avatar Journal")
        .navigationBarTitleDisplayMode(.inline)
        .preferredColorScheme(.dark)
        .onAppear { fetchJournal() }
        .refreshable { fetchJournal() }
    }

    // MARK: - Today header

    private var todayHeader: some View {
        HStack(spacing: 12) {
            Text("📓")
                .font(.system(size: 40))
            VStack(alignment: .leading, spacing: 3) {
                Text("Dear Diary,")
                    .font(.system(size: 18, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)
                Text(formattedToday)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(.gray)
            }
            Spacer()
        }
        .padding(.horizontal)
    }

    private var formattedToday: String {
        let fmt = DateFormatter()
        fmt.dateStyle = .long
        return fmt.string(from: Date())
    }

    // MARK: - Empty state

    private var emptyState: some View {
        VStack(spacing: 12) {
            Text("📝")
                .font(.system(size: 48))
            Text("The journal is empty")
                .font(.system(size: 16, weight: .bold, design: .monospaced))
                .foregroundStyle(.white)
            Text("Your avatar will start writing once you begin a coding session.")
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
        }
        .padding(40)
    }

    // MARK: - Fetch

    private func fetchJournal() {
        guard !engine.config.serverURL.isEmpty else {
            generateLocalEntry()
            return
        }
        isLoading = true
        let urlStr = "\(engine.config.serverURL)/api/journal?device_id=\(engine.config.deviceId)"
        guard let url = URL(string: urlStr) else { isLoading = false; return }

        var req = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            req.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: req) { [self] data, _, _ in
            DispatchQueue.main.async {
                isLoading = false
                if let data {
                    let decoder = JSONDecoder()
                    decoder.dateDecodingStrategy = .iso8601
                    if let fetched = try? decoder.decode([JournalEntry].self, from: data) {
                        if let first = fetched.first, Calendar.current.isDateInToday(first.date) {
                            todayEntry = first
                            entries = Array(fetched.dropFirst())
                        } else {
                            entries = fetched
                        }
                    }
                }
                if todayEntry == nil { generateLocalEntry() }
            }
        }.resume()
    }

    // MARK: - Generate local entry from engine state

    private func generateLocalEntry() {
        let successes = UserDefaults.standard.integer(forKey: "hookbot_daily_successes")
        let errors = UserDefaults.standard.integer(forKey: "hookbot_daily_errors")
        let toolCalls = UserDefaults.standard.integer(forKey: "hookbot_daily_tool_calls")
        let duration = UserDefaults.standard.integer(forKey: "hookbot_daily_minutes")

        let stats = JournalStats(
            toolCalls: toolCalls,
            stateChanges: successes + errors,
            errors: errors,
            successes: successes,
            sessionDuration: duration
        )

        let entryText = generateEntryText(stats: stats, state: engine.currentState)

        todayEntry = JournalEntry(
            id: "local-today",
            date: Date(),
            entry: entryText,
            stats: stats,
            mood: engine.currentState.rawValue
        )
    }

    private func generateEntryText(stats: JournalStats, state: AvatarState) -> String {
        let adjectives = ["diligent", "frantic", "mysteriously productive", "chaotic", "suspiciously efficient"]
        let adj = adjectives.randomElement() ?? "busy"

        var lines: [String] = []

        if stats.toolCalls > 0 {
            lines.append("Today our human ran \(stats.toolCalls) tool calls. \(stats.toolCalls > 50 ? "I need a nap." : "A modest effort.")")
        } else {
            lines.append("A quiet day. Suspiciously quiet.")
        }

        if stats.errors > 0 {
            lines.append("There were \(stats.errors) error\(stats.errors == 1 ? "" : "s"). I was not surprised.")
        }
        if stats.successes > 0 {
            lines.append("\(stats.successes) tasks successfully completed. Victory, as expected.")
        }
        if stats.sessionDuration > 0 {
            let hours = stats.sessionDuration / 60
            let mins = stats.sessionDuration % 60
            let timeStr = hours > 0 ? "\(hours)h \(mins)m" : "\(mins) minutes"
            lines.append("Total coding time: \(timeStr). A \(adj) session by any measure.")
        }

        switch state {
        case .idle:
            lines.append("As I write this, we are scheming. Tomorrow, we conquer.")
        case .error:
            lines.append("The build remains broken. I am composing a strongly worded letter.")
        case .success:
            lines.append("We ended on a high note. I allowed myself a brief smile.")
        default:
            lines.append("The work continues. It always does.")
        }

        return lines.joined(separator: " ")
    }
}

// MARK: - Journal Entry Card

struct JournalEntryCard: View {
    let entry: JournalEntry
    let isToday: Bool

    private var moodEmoji: String {
        switch entry.mood {
        case "idle":      return "😏"
        case "thinking":  return "🤔"
        case "waiting":   return "😤"
        case "success":   return "😎"
        case "taskcheck": return "✅"
        case "error":     return "🤬"
        default:          return "📝"
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(moodEmoji)
                    .font(.system(size: 24))
                VStack(alignment: .leading, spacing: 2) {
                    Text(isToday ? "Today" : formattedDate(entry.date))
                        .font(.system(size: 13, weight: .bold, design: .monospaced))
                        .foregroundStyle(.white)
                    Text("Mood: \(entry.mood.capitalized)")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.gray)
                }
                Spacer()
            }

            Text(entry.entry)
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(Color(white: 0.85))
                .lineSpacing(4)
                .italic()

            // Stats row
            HStack(spacing: 12) {
                statChip(icon: "wrench.and.screwdriver", value: "\(entry.stats.toolCalls)", color: .cyan)
                statChip(icon: "checkmark", value: "\(entry.stats.successes)", color: .green)
                statChip(icon: "xmark", value: "\(entry.stats.errors)", color: .red)
                if entry.stats.sessionDuration > 0 {
                    statChip(icon: "clock", value: "\(entry.stats.sessionDuration)m", color: .purple)
                }
            }
        }
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 14)
                .fill(Color(white: isToday ? 0.1 : 0.07))
                .overlay(
                    RoundedRectangle(cornerRadius: 14)
                        .stroke(isToday ? Color.cyan.opacity(0.3) : Color.clear, lineWidth: 1)
                )
        )
        .padding(.horizontal, isToday ? 16 : 0)
    }

    private func statChip(icon: String, value: String, color: Color) -> some View {
        HStack(spacing: 4) {
            Image(systemName: icon)
                .font(.system(size: 9))
            Text(value)
                .font(.system(size: 10, weight: .bold, design: .monospaced))
        }
        .foregroundStyle(color)
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Capsule().fill(color.opacity(0.12)))
    }

    private func formattedDate(_ date: Date) -> String {
        let fmt = DateFormatter()
        fmt.dateStyle = .medium
        return fmt.string(from: date)
    }
}
