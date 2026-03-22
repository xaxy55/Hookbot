import SwiftUI

struct StoreView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var items: [StoreItem] = []
    @State private var balance: Int = 0
    @State private var isLoading = true
    @State private var error: String?
    @State private var selectedCategory = "All"
    @State private var buyingId: String?

    private let categories = ["All", "Accessories", "Titles", "Animations", "Screensavers"]
    private let columns = [GridItem(.flexible()), GridItem(.flexible())]

    var filteredItems: [StoreItem] {
        if selectedCategory == "All" { return items }
        return items.filter { $0.category.lowercased() == selectedCategory.lowercased() }
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                // Balance bar
                HStack(spacing: 12) {
                    StatCardView(value: "\(balance)", label: "XP Balance", color: .orange, icon: "star.fill")
                    StatCardView(value: "\(items.filter { $0.owned }.count)", label: "Owned", color: .green, icon: "checkmark.circle")
                    StatCardView(value: "\(items.count)", label: "Total", color: .cyan, icon: "bag")
                }
                .padding(.horizontal)

                CategoryFilterBar(categories: categories, selected: $selectedCategory)

                if isLoading {
                    LoadingStateView(message: "Loading store...")
                } else if let error {
                    ErrorStateView(message: error) { fetchStore() }
                } else if filteredItems.isEmpty {
                    EmptyStateView(icon: "bag", message: "No items in this category")
                } else {
                    LazyVGrid(columns: columns, spacing: 12) {
                        ForEach(filteredItems) { item in
                            storeItemCard(item)
                        }
                    }
                    .padding(.horizontal)
                }
            }
            .padding(.vertical)
        }
        .background(Color.black)
        .navigationTitle("Store")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { fetchStore() }
        .refreshable { fetchStore() }
    }

    private func storeItemCard(_ item: StoreItem) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(item.icon ?? storeIcon(for: item.category))
                    .font(.system(size: 24))
                Spacer()
                Text(item.rarity.uppercased())
                    .font(.system(size: 8, weight: .bold, design: .monospaced))
                    .foregroundStyle(rarityColor(item.rarity))
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Capsule().fill(rarityColor(item.rarity).opacity(0.2)))
            }

            Text(item.name)
                .font(.system(size: 13, weight: .bold, design: .monospaced))
                .foregroundStyle(.white)
                .lineLimit(1)

            Text(item.description)
                .font(.system(size: 10, design: .monospaced))
                .foregroundStyle(.gray)
                .lineLimit(2)

            Spacer()

            if item.owned {
                Text("OWNED")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundStyle(.green)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
                    .background(Capsule().fill(Color.green.opacity(0.15)))
            } else {
                Button {
                    buyItem(item)
                } label: {
                    HStack(spacing: 4) {
                        if buyingId == item.id {
                            ProgressView().controlSize(.mini).tint(.black)
                        } else {
                            Image(systemName: "star.fill")
                                .font(.system(size: 9))
                            Text("\(item.price)")
                                .font(.system(size: 12, weight: .bold, design: .monospaced))
                        }
                    }
                    .foregroundStyle(item.canAfford ? .black : .gray)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
                    .background(
                        Capsule().fill(item.canAfford ? Color.orange : Color(white: 0.2))
                    )
                }
                .disabled(!item.canAfford || buyingId != nil)
            }
        }
        .padding(12)
        .frame(minHeight: 160)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(rarityColor(item.rarity).opacity(0.3), lineWidth: 1))
    }

    private func storeIcon(for category: String) -> String {
        switch category.lowercased() {
        case "accessories": return "🎩"
        case "titles": return "🏷️"
        case "animations": return "🎬"
        case "screensavers": return "🌌"
        default: return "📦"
        }
    }

    private func rarityColor(_ rarity: String) -> Color {
        switch rarity.lowercased() {
        case "common": return .green
        case "uncommon": return .blue
        case "rare": return .purple
        case "epic": return .orange
        case "legendary": return .red
        default: return .gray
        }
    }

    // MARK: - API

    private func fetchStore() {
        isLoading = true
        error = nil
        let deviceId = APIService.deviceId
        let query = deviceId.isEmpty ? "" : "?device_id=\(deviceId)"
        guard let url = URL(string: "\(engine.config.serverURL)/api/store\(query)") else { return }

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, response, err in
            DispatchQueue.main.async {
                isLoading = false
                if let err {
                    error = err.localizedDescription
                    return
                }
                guard let data, let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    error = "Failed to load store"
                    return
                }
                let decoder = JSONDecoder()
                decoder.keyDecodingStrategy = .convertFromSnakeCase
                if let resp = try? decoder.decode(StoreResponse.self, from: data) {
                    items = resp.items
                    balance = resp.balance
                }
            }
        }.resume()
    }

    private func buyItem(_ item: StoreItem) {
        buyingId = item.id
        let deviceId = APIService.deviceId
        guard let url = URL(string: "\(engine.config.serverURL)/api/store/buy") else { return }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }
        var body: [String: Any] = ["item_id": item.id]
        if !deviceId.isEmpty { body["device_id"] = deviceId }
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)

        URLSession.shared.dataTask(with: request) { data, _, _ in
            DispatchQueue.main.async {
                buyingId = nil
                if let data,
                   let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let newBal = json["new_balance"] as? Int {
                    balance = newBal
                }
                fetchStore()
            }
        }.resume()
    }
}
