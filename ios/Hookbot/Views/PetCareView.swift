import SwiftUI

struct PetCareView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var pet: PetState?
    @State private var feedMessage: String?
    @State private var isLoading = false
    @State private var isFeeding = false
    @State private var isPetting = false
    @State private var selectedTab: PetTab = .care

    enum PetTab: String, CaseIterable {
        case care = "Pet Care"
        case tokens = "Tokens"
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                tabPicker

                if isLoading && pet == nil {
                    ProgressView()
                        .padding(.top, 40)
                } else if let pet {
                    switch selectedTab {
                    case .care:
                        careTab(pet)
                    case .tokens:
                        tokensTab
                    }
                } else {
                    Text("Connect to a server to see pet status")
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                        .padding(.top, 40)
                }
            }
            .padding()
        }
        .background(Color.black)
        .onAppear { fetchPetState() }
        .refreshable { fetchPetState() }
    }

    // MARK: - Tab Picker

    private var tabPicker: some View {
        HStack(spacing: 8) {
            ForEach(PetTab.allCases, id: \.self) { tab in
                Button {
                    withAnimation(.easeInOut(duration: 0.2)) { selectedTab = tab }
                } label: {
                    Text(tab.rawValue)
                        .font(.system(size: 13, weight: .semibold, design: .monospaced))
                        .foregroundColor(selectedTab == tab ? .white : .gray)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)
                        .background(
                            RoundedRectangle(cornerRadius: 8)
                                .fill(selectedTab == tab ? Color(white: 0.2) : .clear)
                        )
                }
                .buttonStyle(.plain)
            }
            Spacer()
        }
    }

    // MARK: - Care Tab

    @ViewBuilder
    private func careTab(_ pet: PetState) -> some View {
        // Mood header
        VStack(spacing: 4) {
            Text(moodEmoji(pet.mood))
                .font(.system(size: 64))
            Text(pet.mood.capitalized)
                .font(.system(size: 18, weight: .bold, design: .monospaced))
                .foregroundColor(.white)
            if !pet.active_pet.isEmpty {
                Text(pet.active_pet)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 8)

        // Stats bars
        HStack(spacing: 12) {
            statBar(label: "HUNGER", value: pet.hunger, color: .orange)
            statBar(label: "HAPPINESS", value: pet.happiness, color: .pink)
        }

        // Stats row
        HStack(spacing: 16) {
            statCard(value: "\(pet.total_feeds)", label: "FEEDS", color: .orange)
            statCard(value: "\(pet.total_pets)", label: "PETS", color: .pink)
        }

        // Feed message
        if let feedMessage {
            Text(feedMessage)
                .font(.system(size: 13, design: .monospaced))
                .foregroundColor(.green)
                .padding(12)
                .frame(maxWidth: .infinity)
                .background(
                    RoundedRectangle(cornerRadius: 10)
                        .fill(Color.green.opacity(0.1))
                        .overlay(RoundedRectangle(cornerRadius: 10).stroke(Color.green.opacity(0.3)))
                )
        }

        // Feed buttons
        VStack(alignment: .leading, spacing: 10) {
            Text("FEED YOUR BOT")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            HStack(spacing: 10) {
                feedButton(type: "snack", emoji: "\u{1F36A}", label: "Snack", desc: "+15", color: Color(red: 0, green: 0.4, blue: 0))
                feedButton(type: "meal", emoji: "\u{1F35C}", label: "Meal", desc: "+35", color: Color(red: 0, green: 0.2, blue: 0.5))
                feedButton(type: "feast", emoji: "\u{1F969}", label: "Feast", desc: "+60", color: Color(red: 0.35, green: 0, blue: 0.5))
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))

        // Pet button
        VStack(alignment: .leading, spacing: 10) {
            Text("SHOW AFFECTION")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            Button {
                petThePet()
            } label: {
                HStack(spacing: 12) {
                    Text("\u{1F49C}")
                        .font(.system(size: 32))
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Pet your bot")
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundColor(.white)
                        Text("+20 happiness")
                            .font(.system(size: 11))
                            .foregroundColor(.white.opacity(0.6))
                    }
                    Spacer()
                }
                .padding(16)
                .background(
                    RoundedRectangle(cornerRadius: 12)
                        .fill(Color(red: 0.45, green: 0, blue: 0.35))
                )
            }
            .buttonStyle(.plain)
            .disabled(isPetting)
            .opacity(isPetting ? 0.5 : 1)
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))

        // History
        VStack(alignment: .leading, spacing: 8) {
            Text("HISTORY")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            historyRow(label: "Last fed", value: formatTimestamp(pet.last_fed_at))
            historyRow(label: "Last pet", value: formatTimestamp(pet.last_pet_at))
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - Tokens Tab

    @ViewBuilder
    private var tokensTab: some View {
        Text("Token usage tracking")
            .font(.system(size: 13, design: .monospaced))
            .foregroundColor(.secondary)
        Text("View detailed token stats in the web dashboard")
            .font(.system(size: 12, design: .monospaced))
            .foregroundColor(.gray)
            .multilineTextAlignment(.center)
            .padding(.top, 40)
    }

    // MARK: - Components

    private func statBar(label: String, value: Int, color: Color) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(label)
                    .font(.system(size: 10, weight: .bold, design: .monospaced))
                    .foregroundColor(color.opacity(0.7))
                Spacer()
                Text("\(value)")
                    .font(.system(size: 20, weight: .bold, design: .monospaced))
                    .foregroundColor(color)
            }
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 4)
                        .fill(Color(white: 0.15))
                    RoundedRectangle(cornerRadius: 4)
                        .fill(color)
                        .frame(width: geo.size.width * CGFloat(value) / 100.0)
                }
            }
            .frame(height: 6)
        }
        .padding(14)
        .background(
            RoundedRectangle(cornerRadius: 10)
                .fill(Color(white: 0.08))
                .overlay(RoundedRectangle(cornerRadius: 10).stroke(color.opacity(0.2)))
        )
    }

    private func statCard(value: String, label: String, color: Color) -> some View {
        VStack(spacing: 4) {
            Text(value)
                .font(.system(size: 22, weight: .bold, design: .monospaced))
                .foregroundColor(.white)
            Text(label)
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)
        }
        .frame(maxWidth: .infinity)
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.08)))
    }

    private func feedButton(type: String, emoji: String, label: String, desc: String, color: Color) -> some View {
        Button {
            feedPet(type)
        } label: {
            VStack(spacing: 6) {
                Text(emoji)
                    .font(.system(size: 28))
                Text(label)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundColor(.white)
                Text(desc)
                    .font(.system(size: 10))
                    .foregroundColor(.white.opacity(0.6))
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 14)
            .background(RoundedRectangle(cornerRadius: 10).fill(color))
        }
        .buttonStyle(.plain)
        .disabled(isFeeding)
        .opacity(isFeeding ? 0.5 : 1)
    }

    private func historyRow(label: String, value: String) -> some View {
        HStack {
            Text(label)
                .font(.system(size: 12, design: .monospaced))
                .foregroundColor(.gray)
            Spacer()
            Text(value)
                .font(.system(size: 12, design: .monospaced))
                .foregroundColor(.white)
        }
    }

    // MARK: - Helpers

    private func moodEmoji(_ mood: String) -> String {
        switch mood {
        case "ecstatic":  return "\u{1F929}"
        case "happy":     return "\u{1F60A}"
        case "content":   return "\u{1F642}"
        case "grumpy":    return "\u{1F612}"
        case "sad":       return "\u{1F622}"
        case "miserable": return "\u{1F62D}"
        default:          return "\u{1F610}"
        }
    }

    private func formatTimestamp(_ ts: String?) -> String {
        guard let ts, !ts.isEmpty else { return "Never" }
        // Show just the relative time or the raw timestamp
        return String(ts.prefix(19))
    }

    // MARK: - API Calls

    private func fetchPetState() {
        guard !engine.config.serverURL.isEmpty else { return }
        isLoading = true

        let deviceQuery = engine.config.deviceId.isEmpty ? "" : "?device_id=\(engine.config.deviceId)"
        guard let url = URL(string: "\(engine.config.serverURL)/api/pet\(deviceQuery)") else {
            isLoading = false
            return
        }

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, response, _ in
            DispatchQueue.main.async {
                isLoading = false
                guard let data,
                      let http = response as? HTTPURLResponse, http.statusCode == 200 else { return }
                if let decoded = try? JSONDecoder().decode(PetState.self, from: data) {
                    self.pet = decoded
                }
            }
        }.resume()
    }

    private func feedPet(_ foodType: String) {
        guard !engine.config.serverURL.isEmpty else { return }
        isFeeding = true

        guard let url = URL(string: "\(engine.config.serverURL)/api/pet/feed") else {
            isFeeding = false
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        let body: [String: Any] = [
            "food_type": foodType,
            "device_id": engine.config.deviceId,
        ]
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)

        URLSession.shared.dataTask(with: request) { data, response, _ in
            DispatchQueue.main.async {
                isFeeding = false
                guard let data,
                      let http = response as? HTTPURLResponse, http.statusCode == 200,
                      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }

                if let message = json["message"] as? String {
                    feedMessage = message
                    DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                        feedMessage = nil
                    }
                }

                // Refresh state
                fetchPetState()
            }
        }.resume()
    }

    private func petThePet() {
        guard !engine.config.serverURL.isEmpty else { return }
        isPetting = true

        guard let url = URL(string: "\(engine.config.serverURL)/api/pet/pet") else {
            isPetting = false
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        let body: [String: Any] = ["device_id": engine.config.deviceId]
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)

        URLSession.shared.dataTask(with: request) { data, response, _ in
            DispatchQueue.main.async {
                isPetting = false
                guard let data,
                      let http = response as? HTTPURLResponse, http.statusCode == 200,
                      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }

                if let message = json["message"] as? String {
                    feedMessage = message
                    DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                        feedMessage = nil
                    }
                }

                fetchPetState()
            }
        }.resume()
    }
}

// MARK: - Pet API Models

struct PetStoreItem: Codable {
    let id: Int
    let name: String
    let unlocked: Bool
    let cost: Int
    let progress: Double
}

struct PetState: Codable {
    let device_id: String
    let hunger: Int
    let happiness: Int
    let last_fed_at: String?
    let last_pet_at: String?
    let total_feeds: Int
    let total_pets: Int
    let mood: String
    let active_pet: String
    let active_pet_id: Int
    let store: [PetStoreItem]
}
