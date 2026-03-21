import SwiftUI

// Avatar visit history — a social timeline of who visited and when (Phase 13.2)

struct VisitHistoryView: View {
    @EnvironmentObject var engine: AvatarEngine
    @StateObject private var social = SocialService.shared
    @State private var showSendVisit = false
    @State private var selectedFriend: Friend?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                // Send visit button
                Button {
                    showSendVisit = true
                } label: {
                    HStack(spacing: 12) {
                        Image(systemName: "paperplane.fill")
                            .foregroundStyle(.cyan)
                        Text("Send your avatar to visit a friend")
                            .font(.system(size: 14, weight: .semibold, design: .monospaced))
                            .foregroundStyle(.white)
                        Spacer()
                        Image(systemName: "chevron.right")
                            .foregroundStyle(.gray)
                    }
                    .padding(16)
                    .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.1)))
                }
                .buttonStyle(.plain)

                // History
                if social.visitHistory.isEmpty {
                    emptyState
                } else {
                    LazyVStack(spacing: 10) {
                        ForEach(social.visitHistory) { visit in
                            VisitRow(visit: visit) {
                                social.markVisitRead(visit.id)
                            }
                        }
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Visit History")
        .navigationBarTitleDisplayMode(.inline)
        .preferredColorScheme(.dark)
        .onAppear {
            social.configure(engine: engine)
            social.fetchVisitHistory()
        }
        .refreshable { social.fetchVisitHistory() }
        .sheet(isPresented: $showSendVisit) {
            SendVisitSheet(friends: social.friends) { friendId, message, gift in
                social.sendAvatarVisit(to: friendId, message: message, gift: gift)
                showSendVisit = false
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Text("🚶")
                .font(.system(size: 48))
            Text("No visits yet")
                .font(.system(size: 16, weight: .bold, design: .monospaced))
                .foregroundStyle(.white)
            Text("Pair with a buddy and send your avatar to visit!")
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
        }
        .padding(40)
    }
}

// MARK: - Visit Row

struct VisitRow: View {
    let visit: AvatarVisit
    let onRead: () -> Void

    var body: some View {
        HStack(spacing: 12) {
            // Unread indicator
            Circle()
                .fill(visit.isRead ? Color.clear : Color.cyan)
                .frame(width: 8, height: 8)

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(visit.fromFriendName)
                        .font(.system(size: 14, weight: .bold, design: .monospaced))
                        .foregroundStyle(.white)
                    Spacer()
                    Text(relativeTime(visit.timestamp))
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.gray)
                }

                if !visit.message.isEmpty {
                    Text(visit.message)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(.gray)
                        .lineLimit(2)
                }

                if let gift = visit.gift {
                    Label("Gift: \(gift)", systemImage: "gift.fill")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.yellow)
                }
            }
        }
        .padding(14)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(Color(white: visit.isRead ? 0.06 : 0.1))
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(visit.isRead ? Color.clear : Color.cyan.opacity(0.3), lineWidth: 1)
                )
        )
        .onTapGesture {
            if !visit.isRead { onRead() }
        }
    }

    private func relativeTime(_ date: Date) -> String {
        let secs = Date().timeIntervalSince(date)
        if secs < 60 { return "Just now" }
        if secs < 3600 { return "\(Int(secs / 60))m ago" }
        if secs < 86400 { return "\(Int(secs / 3600))h ago" }
        return "\(Int(secs / 86400))d ago"
    }
}

// MARK: - Send Visit Sheet

struct SendVisitSheet: View {
    let friends: [Friend]
    let onSend: (String, String, String?) -> Void

    @State private var selectedFriendId: String = ""
    @State private var message = ""
    @State private var selectedGift: String? = nil
    @Environment(\.dismiss) var dismiss

    private let gifts = ["🎩", "🎸", "🌟", "🔥", "💎", "🚀", "🎆", "🦄"]

    var body: some View {
        NavigationStack {
            Form {
                Section("Send to") {
                    if friends.isEmpty {
                        Text("No friends yet — pair with someone first!")
                            .foregroundStyle(.secondary)
                            .font(.system(.body, design: .monospaced))
                    } else {
                        Picker("Friend", selection: $selectedFriendId) {
                            ForEach(friends) { friend in
                                Text(friend.username).tag(friend.id)
                            }
                        }
                        .onAppear {
                            if selectedFriendId.isEmpty { selectedFriendId = friends.first?.id ?? "" }
                        }
                    }
                }

                Section("Message") {
                    TextField("Say something to your buddy...", text: $message, axis: .vertical)
                        .lineLimit(3, reservesSpace: true)
                }

                Section("Send a gift") {
                    LazyVGrid(columns: Array(repeating: .init(.flexible()), count: 4), spacing: 12) {
                        ForEach(gifts, id: \.self) { gift in
                            Button {
                                selectedGift = selectedGift == gift ? nil : gift
                            } label: {
                                Text(gift)
                                    .font(.system(size: 30))
                                    .padding(8)
                                    .background(
                                        Circle()
                                            .fill(selectedGift == gift ? Color.cyan.opacity(0.3) : Color.clear)
                                    )
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
            .navigationTitle("Send Avatar Visit")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Send") {
                        onSend(selectedFriendId, message, selectedGift)
                    }
                    .disabled(selectedFriendId.isEmpty)
                    .fontWeight(.bold)
                }
            }
        }
        .preferredColorScheme(.dark)
    }
}
