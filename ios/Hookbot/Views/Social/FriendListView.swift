import SwiftUI

// Friend list & presence — who's online, their mood, streak, last activity (Phase 13.4)

struct FriendListView: View {
    @EnvironmentObject var engine: AvatarEngine
    @StateObject private var social = SocialService.shared
    @State private var showPairing = false
    @State private var selectedFriend: Friend?
    @State private var showEmotes = false
    @State private var showChallenge = false

    var onlineFriends: [Friend] { social.friends.filter { $0.isOnline } }
    var offlineFriends: [Friend] { social.friends.filter { !$0.isOnline } }

    var body: some View {
        NavigationStack {
            List {
                if social.friends.isEmpty {
                    emptySection
                } else {
                    if !onlineFriends.isEmpty {
                        Section("Online (\(onlineFriends.count))") {
                            ForEach(onlineFriends) { friend in
                                FriendRow(friend: friend, onAction: { action in
                                    handleAction(action, friend: friend)
                                })
                            }
                        }
                    }

                    if !offlineFriends.isEmpty {
                        Section("Offline") {
                            ForEach(offlineFriends) { friend in
                                FriendRow(friend: friend, onAction: { action in
                                    handleAction(action, friend: friend)
                                })
                            }
                        }
                    }
                }
            }
            .scrollContentBackground(.hidden)
            .background(Color.black)
            .navigationTitle("Friends")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        showPairing = true
                    } label: {
                        Image(systemName: "person.badge.plus")
                            .foregroundStyle(.cyan)
                    }
                }
            }
            .preferredColorScheme(.dark)
            .onAppear {
                social.configure(engine: engine)
                social.refreshFriends()
            }
            .refreshable { social.refreshFriends() }
            .sheet(isPresented: $showPairing) {
                PairingView()
                    .environmentObject(engine)
            }
            .sheet(item: $selectedFriend) { friend in
                FriendDetailSheet(friend: friend, social: social)
                    .environmentObject(engine)
            }
            .sheet(isPresented: $showEmotes) {
                if let friend = selectedFriend {
                    AvatarEmotesView(targetFriend: friend)
                        .environmentObject(engine)
                }
            }
        }
    }

    private var emptySection: some View {
        Section {
            VStack(spacing: 16) {
                Text("👥")
                    .font(.system(size: 48))
                Text("No friends yet")
                    .font(.system(size: 16, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)
                Text("Pair with a buddy to see them here")
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(.gray)
                Button {
                    showPairing = true
                } label: {
                    Label("Pair with Buddy", systemImage: "qrcode")
                        .font(.system(size: 14, weight: .semibold, design: .monospaced))
                }
                .buttonStyle(.borderedProminent)
                .tint(.cyan)
            }
            .frame(maxWidth: .infinity)
            .padding(30)
            .listRowBackground(Color.clear)
        }
    }

    private func handleAction(_ action: FriendRow.Action, friend: Friend) {
        selectedFriend = friend
        switch action {
        case .viewDetail:   selectedFriend = friend
        case .sendEmote:    showEmotes = true
        case .sendRaid:     social.sendRaid(to: friend.id)
        case .challenge:    showChallenge = true
        }
    }
}

// MARK: - Friend Row

struct FriendRow: View {
    enum Action { case viewDetail, sendEmote, sendRaid, challenge }

    let friend: Friend
    let onAction: (Action) -> Void

    var moodEmoji: String {
        switch friend.avatarState {
        case "idle":      return "😏"
        case "thinking":  return "🤔"
        case "waiting":   return "😤"
        case "success":   return "😎"
        case "taskcheck": return "✅"
        case "error":     return "🤬"
        default:          return "🤖"
        }
    }

    var body: some View {
        HStack(spacing: 12) {
            // Online indicator + mood
            ZStack {
                Circle()
                    .fill(friend.isOnline ? Color.green : Color.gray)
                    .frame(width: 8, height: 8)
                    .offset(x: 14, y: -14)
                Text(moodEmoji)
                    .font(.system(size: 32))
            }
            .frame(width: 40, height: 40)

            VStack(alignment: .leading, spacing: 3) {
                Text(friend.username)
                    .font(.system(size: 15, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)

                HStack(spacing: 8) {
                    Label("\(friend.streak)d", systemImage: "flame.fill")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.orange)
                    Label("\(friend.xp) XP", systemImage: "star.fill")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.yellow)
                }

                if !friend.lastActivity.isEmpty {
                    Text(friend.lastActivity)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.gray)
                        .lineLimit(1)
                }
            }

            Spacer()

            // Quick action menu
            Menu {
                Button {
                    onAction(.sendEmote)
                } label: {
                    Label("Send Emote", systemImage: "hand.wave.fill")
                }
                Button {
                    onAction(.sendRaid)
                } label: {
                    Label("Send Raid", systemImage: "bolt.fill")
                }
                Button {
                    onAction(.challenge)
                } label: {
                    Label("Challenge", systemImage: "trophy.fill")
                }
                Button {
                    onAction(.viewDetail)
                } label: {
                    Label("View Profile", systemImage: "person.fill")
                }
            } label: {
                Image(systemName: "ellipsis.circle")
                    .foregroundStyle(.gray)
                    .font(.system(size: 18))
            }
        }
        .padding(.vertical, 4)
        .listRowBackground(Color(white: 0.08))
    }
}

// MARK: - Friend Detail Sheet

struct FriendDetailSheet: View {
    let friend: Friend
    let social: SocialService
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 20) {
                    // Header
                    VStack(spacing: 8) {
                        Text(friend.isOnline ? "🟢" : "⚫")
                            .font(.system(size: 14))
                        Text(friend.username)
                            .font(.system(size: 28, weight: .black, design: .monospaced))
                            .foregroundStyle(.white)
                        Text(friend.avatarState.capitalized)
                            .font(.system(size: 14, design: .monospaced))
                            .foregroundStyle(.gray)
                    }

                    // Stats
                    HStack(spacing: 16) {
                        statCard(value: "\(friend.streak)", label: "Streak", icon: "flame.fill", color: .orange)
                        statCard(value: "\(friend.xp)", label: "XP", icon: "star.fill", color: .yellow)
                    }

                    // Actions
                    VStack(spacing: 12) {
                        actionButton(label: "Send Raid", icon: "bolt.fill", color: .purple) {
                            social.sendRaid(to: friend.id)
                            dismiss()
                        }
                        actionButton(label: "Visit Avatar", icon: "paperplane.fill", color: .cyan) {
                            social.sendAvatarVisit(to: friend.id, message: "Dropping by to say hi!")
                            dismiss()
                        }
                        actionButton(label: "Challenge", icon: "trophy.fill", color: .yellow) {
                            social.sendChallenge(to: friend.id, type: "xp_earned")
                            dismiss()
                        }
                    }
                }
                .padding()
            }
            .background(Color.black)
            .navigationTitle(friend.username)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .preferredColorScheme(.dark)
    }

    private func statCard(value: String, label: String, icon: String, color: Color) -> some View {
        VStack(spacing: 6) {
            Label(value, systemImage: icon)
                .font(.system(size: 22, weight: .black, design: .monospaced))
                .foregroundStyle(color)
            Text(label)
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .frame(maxWidth: .infinity)
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func actionButton(label: String, icon: String, color: Color, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack {
                Image(systemName: icon)
                    .foregroundStyle(color)
                Text(label)
                    .font(.system(size: 15, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.white)
                Spacer()
                Image(systemName: "chevron.right")
                    .foregroundStyle(.gray)
            }
            .padding(16)
            .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.1)))
        }
        .buttonStyle(.plain)
    }
}
