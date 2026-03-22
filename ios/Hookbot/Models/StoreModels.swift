import Foundation

struct StoreItem: Codable, Identifiable {
    let id: String
    let name: String
    let description: String
    let category: String
    let price: Int
    let icon: String?
    let rarity: String
    let owned: Bool
    let canAfford: Bool

    enum CodingKeys: String, CodingKey {
        case id, name, description, category, price, icon, rarity, owned
        case canAfford = "can_afford"
    }
}

struct StoreResponse: Codable {
    let items: [StoreItem]
    let balance: Int
}

struct BuyResponse: Codable {
    let ok: Bool
    let itemId: String?
    let xpSpent: Int?
    let newBalance: Int?

    enum CodingKeys: String, CodingKey {
        case ok
        case itemId = "item_id"
        case xpSpent = "xp_spent"
        case newBalance = "new_balance"
    }
}
