import SwiftUI

struct MoodJournalView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var entries: [MoodEntry] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    // New entry form
    @State private var selectedMood: String = ""
    @State private var noteText = ""
    @State private var energyLevel: Double = 5
    @State private var isSubmitting = false
    @State private var statusMessage: String?

    private let moods = [
        ("Awesome", "star.fill"),
        ("Happy", "face.smiling"),
        ("Okay", "equal.circle"),
        ("Tired", "moon.zzz"),
        ("Stressed", "bolt.heart"),
        ("Sad", "cloud.rain"),
    ]

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                // New entry
                newEntryCard

                // History
                if isLoading && entries.isEmpty {
                    LoadingStateView(message: "Loading journal...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchEntries() }
                } else if entries.isEmpty {
                    EmptyStateView(icon: "heart.text.square", message: "No mood entries yet\nHow are you feeling?")
                } else {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("HISTORY")
                            .font(.system(size: 11, weight: .bold, design: .monospaced))
                            .foregroundColor(.gray)

                        ForEach(entries) { entry in
                            entryRow(entry)
                        }
                    }
                }

                if let statusMessage {
                    Text(statusMessage)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundColor(.green)
                        .padding(10)
                        .frame(maxWidth: .infinity)
                        .background(RoundedRectangle(cornerRadius: 8).fill(Color.green.opacity(0.1)))
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Mood Journal")
        .onAppear { fetchEntries() }
        .refreshable { fetchEntries() }
    }

    // MARK: - New Entry Card

    private var newEntryCard: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("HOW ARE YOU FEELING?")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            // Mood picker
            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())], spacing: 10) {
                ForEach(moods, id: \.0) { mood, icon in
                    Button {
                        selectedMood = mood
                    } label: {
                        VStack(spacing: 4) {
                            Image(systemName: icon)
                                .font(.system(size: 20))
                            Text(mood)
                                .font(.system(size: 10, weight: .medium, design: .monospaced))
                        }
                        .foregroundColor(selectedMood == mood ? .black : .white)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 10)
                        .background(
                            RoundedRectangle(cornerRadius: 8)
                                .fill(selectedMood == mood ? Color.cyan : Color(white: 0.12))
                        )
                    }
                    .buttonStyle(.plain)
                }
            }

            // Energy slider
            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text("ENERGY")
                        .font(.system(size: 10, weight: .bold, design: .monospaced))
                        .foregroundColor(.gray)
                    Spacer()
                    Text("\(Int(energyLevel))/10")
                        .font(.system(size: 12, weight: .bold, design: .monospaced))
                        .foregroundColor(.orange)
                }
                Slider(value: $energyLevel, in: 1...10, step: 1)
                    .tint(.orange)
            }

            // Note
            TextField("Add a note...", text: $noteText, axis: .vertical)
                .font(.system(size: 13, design: .monospaced))
                .foregroundColor(.white)
                .lineLimit(3...5)
                .padding(10)
                .background(RoundedRectangle(cornerRadius: 8).fill(Color(white: 0.12)))

            // Submit
            Button {
                submitEntry()
            } label: {
                Text("Log Mood")
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                    .foregroundColor(.black)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 12)
                    .background(RoundedRectangle(cornerRadius: 10).fill(
                        selectedMood.isEmpty ? Color.gray : Color.cyan
                    ))
            }
            .buttonStyle(.plain)
            .disabled(selectedMood.isEmpty || isSubmitting)
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - Entry Row

    private func entryRow(_ entry: MoodEntry) -> some View {
        HStack(spacing: 12) {
            Image(systemName: moodIcon(entry.mood))
                .font(.system(size: 18))
                .foregroundColor(.cyan)
                .frame(width: 32)

            VStack(alignment: .leading, spacing: 3) {
                Text(entry.mood)
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                if let note = entry.note, !note.isEmpty {
                    Text(note)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundColor(.gray)
                        .lineLimit(2)
                }
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 3) {
                Text("E:\(entry.energy)")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundColor(.orange)

                if let ts = entry.createdAt {
                    Text(String(ts.prefix(10)))
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundColor(.gray)
                }
            }
        }
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.08)))
    }

    private func moodIcon(_ mood: String) -> String {
        switch mood.lowercased() {
        case "awesome": return "star.fill"
        case "happy": return "face.smiling"
        case "okay": return "equal.circle"
        case "tired": return "moon.zzz"
        case "stressed": return "bolt.heart"
        case "sad": return "cloud.rain"
        default: return "circle"
        }
    }

    // MARK: - API

    private func fetchEntries() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/mood") else { return }
        isLoading = true
        errorMessage = nil

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                isLoading = false
                if let error { errorMessage = error.localizedDescription; return }
                guard let data,
                      let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    errorMessage = "Failed to load mood entries"
                    return
                }
                entries = (try? JSONDecoder().decode([MoodEntry].self, from: data)) ?? []
            }
        }.resume()
    }

    private func submitEntry() {
        guard !engine.config.serverURL.isEmpty, !selectedMood.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/mood") else { return }
        isSubmitting = true

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        let body: [String: Any] = [
            "mood": selectedMood,
            "note": noteText,
            "energy": Int(energyLevel)
        ]
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)

        URLSession.shared.dataTask(with: request) { _, response, _ in
            DispatchQueue.main.async {
                isSubmitting = false
                guard let http = response as? HTTPURLResponse, http.statusCode == 200 || http.statusCode == 201 else {
                    statusMessage = "Failed to log mood"
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                    return
                }
                statusMessage = "Mood logged"
                selectedMood = ""
                noteText = ""
                energyLevel = 5
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) { statusMessage = nil }
                fetchEntries()
            }
        }.resume()
    }
}

// MARK: - Model

struct MoodEntry: Codable, Identifiable {
    var id: String { "\(mood)-\(createdAt ?? UUID().uuidString)" }
    let mood: String
    let note: String?
    let energy: Int
    let createdAt: String?

    enum CodingKeys: String, CodingKey {
        case mood, note, energy
        case createdAt = "created_at"
    }
}
