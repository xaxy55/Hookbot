import SwiftUI

// Avatar wardrobe — unlockable outfits, hats, and props (Phase 13.3)
// Items are unlocked by coding stats: deploy cape, debug monocle, 100-streak crown

// MARK: - Wardrobe Item

struct WardrobeItem: Identifiable, Codable {
    let id: String
    let name: String
    let description: String
    let icon: String       // emoji or SF Symbol name
    let slot: WardrobeSlot
    let unlockCondition: String
    var isUnlocked: Bool
    var isEquipped: Bool

    enum WardrobeSlot: String, Codable, CaseIterable {
        case head    = "Head"
        case face    = "Face"
        case neck    = "Neck"
        case effect  = "Effect"
    }
}

// MARK: - View

struct WardrobeView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var selectedSlot: WardrobeItem.WardrobeSlot = .head
    @State private var items: [WardrobeItem] = WardrobeView.defaultItems()

    var filteredItems: [WardrobeItem] {
        items.filter { $0.slot == selectedSlot }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Avatar preview
            avatarPreview
                .frame(height: 160)
                .padding(.horizontal)
                .padding(.top, 8)

            // Slot picker
            slotPicker
                .padding(.vertical, 12)

            // Item grid
            ScrollView {
                LazyVGrid(columns: [.init(.flexible()), .init(.flexible())], spacing: 12) {
                    ForEach(filteredItems) { item in
                        WardrobeItemCard(item: item) {
                            equipItem(item)
                        }
                    }
                }
                .padding()
            }
        }
        .background(Color.black)
        .navigationTitle("Wardrobe")
        .navigationBarTitleDisplayMode(.inline)
        .preferredColorScheme(.dark)
        .onAppear { syncWithEngine() }
    }

    // MARK: - Avatar preview

    private var avatarPreview: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 12)
                .fill(Color(white: 0.06))
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(Color(white: 0.15), lineWidth: 1)
                )

            AvatarView()
                .environmentObject(engine)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .padding(8)

            VStack {
                Spacer()
                Text("Preview")
                    .font(.system(size: 10, weight: .bold, design: .monospaced))
                    .foregroundStyle(.gray)
                    .padding(.bottom, 8)
            }
        }
    }

    // MARK: - Slot picker

    private var slotPicker: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 10) {
                ForEach(WardrobeItem.WardrobeSlot.allCases, id: \.self) { slot in
                    Button {
                        selectedSlot = slot
                    } label: {
                        Text(slot.rawValue)
                            .font(.system(size: 12, weight: .bold, design: .monospaced))
                            .foregroundStyle(selectedSlot == slot ? .black : .gray)
                            .padding(.horizontal, 14)
                            .padding(.vertical, 8)
                            .background(
                                Capsule()
                                    .fill(selectedSlot == slot ? Color.cyan : Color(white: 0.12))
                            )
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal)
        }
    }

    // MARK: - Equip

    private func equipItem(_ item: WardrobeItem) {
        guard item.isUnlocked else { return }
        items = items.map { i in
            var updated = i
            if i.slot == item.slot {
                // Unequip others in same slot
                updated.isEquipped = false
            }
            if i.id == item.id {
                updated.isEquipped = !item.isEquipped
            }
            return updated
        }
        applyToEngine()
    }

    private func applyToEngine() {
        let equipped = items.filter { $0.isEquipped }
        var accessories = AccessoriesConfig()

        for item in equipped {
            switch item.id {
            case "tophat":   accessories.topHat = true
            case "crown":    accessories.crown = true
            case "horns":    accessories.horns = true
            case "halo":     accessories.halo = true
            case "glasses":  accessories.glasses = true
            case "monocle":  accessories.monocle = true
            case "bowtie":   accessories.bowtie = true
            case "cigar":    accessories.cigar = true
            default: break
            }
        }
        engine.config.accessories = accessories
        if let data = try? JSONEncoder().encode(engine.config) {
            UserDefaults.standard.set(data, forKey: "hookbot_config")
        }
    }

    private func syncWithEngine() {
        let acc = engine.config.accessories
        items = items.map { item in
            var updated = item
            switch item.id {
            case "tophat":   updated.isEquipped = acc.topHat
            case "crown":    updated.isEquipped = acc.crown
            case "horns":    updated.isEquipped = acc.horns
            case "halo":     updated.isEquipped = acc.halo
            case "glasses":  updated.isEquipped = acc.glasses
            case "monocle":  updated.isEquipped = acc.monocle
            case "bowtie":   updated.isEquipped = acc.bowtie
            case "cigar":    updated.isEquipped = acc.cigar
            default: break
            }
            return updated
        }
        // Unlock items based on XP / streak
        let xp = UserDefaults.standard.integer(forKey: "hookbot_xp")
        let streak = UserDefaults.standard.integer(forKey: "hookbot_streak")
        items = items.map { item in
            var updated = item
            switch item.id {
            case "crown":      updated.isUnlocked = streak >= 100
            case "monocle":    updated.isUnlocked = xp >= 5000
            case "deploy_cape": updated.isUnlocked = UserDefaults.standard.integer(forKey: "hookbot_deploys") >= 10
            case "halo":       updated.isUnlocked = xp >= 10000
            default: break
            }
            return updated
        }
    }

    // MARK: - Default items

    static func defaultItems() -> [WardrobeItem] {
        [
            WardrobeItem(id: "tophat",   name: "Top Hat",       description: "Classic CEO headwear",           icon: "🎩", slot: .head, unlockCondition: "Always unlocked", isUnlocked: true, isEquipped: false),
            WardrobeItem(id: "crown",    name: "100-Streak Crown", description: "Reach a 100-day streak",     icon: "👑", slot: .head, unlockCondition: "100-day streak", isUnlocked: false, isEquipped: false),
            WardrobeItem(id: "horns",    name: "Devil Horns",   description: "For chaotic builds",             icon: "😈", slot: .head, unlockCondition: "Always unlocked", isUnlocked: true, isEquipped: false),
            WardrobeItem(id: "halo",     name: "Halo",          description: "For the righteous coder",        icon: "😇", slot: .head, unlockCondition: "10,000 XP",     isUnlocked: false, isEquipped: false),
            WardrobeItem(id: "glasses",  name: "Glasses",       description: "See the code clearly",           icon: "🕶️", slot: .face, unlockCondition: "Always unlocked", isUnlocked: true, isEquipped: false),
            WardrobeItem(id: "monocle",  name: "Debug Monocle", description: "Unlock at 5,000 XP",            icon: "🧐", slot: .face, unlockCondition: "5,000 XP",      isUnlocked: false, isEquipped: false),
            WardrobeItem(id: "bowtie",   name: "Bow Tie",       description: "Power move in any standup",      icon: "🎀", slot: .neck, unlockCondition: "Always unlocked", isUnlocked: true, isEquipped: false),
            WardrobeItem(id: "cigar",    name: "Cigar",         description: "For celebrating production",     icon: "🚬", slot: .neck, unlockCondition: "Age verified",   isUnlocked: true, isEquipped: false),
            WardrobeItem(id: "deploy_cape", name: "Deploy Cape", description: "10 successful deploys",        icon: "🦸", slot: .effect, unlockCondition: "10 deploys",   isUnlocked: false, isEquipped: false),
        ]
    }
}

// MARK: - Item Card

struct WardrobeItemCard: View {
    let item: WardrobeItem
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            VStack(spacing: 8) {
                ZStack {
                    RoundedRectangle(cornerRadius: 12)
                        .fill(cardBackground)
                        .frame(height: 80)

                    Text(item.icon)
                        .font(.system(size: 40))
                        .opacity(item.isUnlocked ? 1 : 0.3)

                    if !item.isUnlocked {
                        Image(systemName: "lock.fill")
                            .font(.system(size: 16))
                            .foregroundStyle(.gray)
                            .offset(y: 10)
                    }

                    if item.isEquipped {
                        VStack {
                            HStack {
                                Spacer()
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundStyle(.cyan)
                                    .padding(6)
                            }
                            Spacer()
                        }
                    }
                }

                Text(item.name)
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundStyle(item.isUnlocked ? .white : .gray)
                    .lineLimit(1)

                if !item.isUnlocked {
                    Text(item.unlockCondition)
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(.gray)
                        .lineLimit(1)
                }
            }
        }
        .buttonStyle(.plain)
        .disabled(!item.isUnlocked)
    }

    private var cardBackground: Color {
        if item.isEquipped { return Color.cyan.opacity(0.2) }
        return Color(white: 0.1)
    }
}
