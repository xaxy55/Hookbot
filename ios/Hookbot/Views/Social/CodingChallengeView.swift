import SwiftUI

// Coding challenges — "Who writes more tests this week?" Loser's avatar wears a silly hat (Phase 13.4)

struct CodingChallengeView: View {
    @EnvironmentObject var engine: AvatarEngine
    @StateObject private var social = SocialService.shared
    @State private var showNewChallenge = false

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Active challenges
                if !social.activeChallenges.isEmpty {
                    sectionHeader("ACTIVE CHALLENGES")
                    ForEach(social.activeChallenges) { challenge in
                        ChallengeCard(challenge: challenge, isActive: true)
                    }
                }

                // Pending challenges
                if !social.pendingChallenges.isEmpty {
                    sectionHeader("PENDING")
                    ForEach(social.pendingChallenges) { challenge in
                        ChallengeCard(challenge: challenge, isActive: false)
                    }
                }

                if social.activeChallenges.isEmpty && social.pendingChallenges.isEmpty {
                    emptyState
                }

                // New challenge button
                Button {
                    showNewChallenge = true
                } label: {
                    Label("New Challenge", systemImage: "trophy.fill")
                        .font(.system(size: 15, weight: .bold, design: .monospaced))
                        .foregroundStyle(.black)
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(Color.yellow)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .buttonStyle(.plain)
                .disabled(social.friends.isEmpty)
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Challenges")
        .navigationBarTitleDisplayMode(.inline)
        .preferredColorScheme(.dark)
        .onAppear {
            social.configure(engine: engine)
            social.refreshChallenges()
        }
        .sheet(isPresented: $showNewChallenge) {
            NewChallengeSheet(friends: social.friends) { friendId, type in
                social.sendChallenge(to: friendId, type: type)
                showNewChallenge = false
            }
        }
    }

    private func sectionHeader(_ title: String) -> some View {
        HStack {
            Text(title)
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)
            Spacer()
        }
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Text("🏆")
                .font(.system(size: 48))
            Text("No challenges yet")
                .font(.system(size: 16, weight: .bold, design: .monospaced))
                .foregroundStyle(.white)
            Text("Challenge a friend to a coding duel. Loser's avatar gets a silly hat.")
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
        }
        .padding(40)
    }
}

// MARK: - Challenge Card

struct ChallengeCard: View {
    let challenge: CodingChallenge
    let isActive: Bool

    var isWinning: Bool { challenge.myScore >= challenge.theirScore }
    var winner: String { challenge.myScore >= challenge.theirScore ? "You" : challenge.challengerName }

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    Text("vs. \(challenge.challengerName)")
                        .font(.system(size: 16, weight: .bold, design: .monospaced))
                        .foregroundStyle(.white)
                    Text(challenge.metric)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.gray)
                }
                Spacer()
                Image(systemName: isActive ? "flame.fill" : "clock.fill")
                    .foregroundStyle(isActive ? .orange : .gray)
            }

            // Score bar
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 6)
                        .fill(Color(white: 0.15))
                        .frame(height: 12)

                    let total = max(1, challenge.myScore + challenge.theirScore)
                    RoundedRectangle(cornerRadius: 6)
                        .fill(isWinning ? Color.green : Color.red)
                        .frame(width: geo.size.width * CGFloat(challenge.myScore) / CGFloat(total), height: 12)
                }
            }
            .frame(height: 12)

            HStack {
                VStack(alignment: .leading) {
                    Text("You")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.gray)
                    Text("\(challenge.myScore)")
                        .font(.system(size: 20, weight: .black, design: .monospaced))
                        .foregroundStyle(isWinning ? .green : .white)
                }
                Spacer()
                if isActive {
                    countdownBadge
                }
                Spacer()
                VStack(alignment: .trailing) {
                    Text(challenge.challengerName)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.gray)
                    Text("\(challenge.theirScore)")
                        .font(.system(size: 20, weight: .black, design: .monospaced))
                        .foregroundStyle(!isWinning ? .red : .white)
                }
            }

            if !isActive {
                // Result
                HStack {
                    Image(systemName: "trophy.fill")
                        .foregroundStyle(.yellow)
                    Text("\(winner) wins! Loser's avatar wears the silly hat 🎩")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.gray)
                }
                .padding(8)
                .frame(maxWidth: .infinity)
                .background(RoundedRectangle(cornerRadius: 8).fill(Color.yellow.opacity(0.1)))
            }
        }
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 14)
                .fill(Color(white: 0.08))
                .overlay(
                    RoundedRectangle(cornerRadius: 14)
                        .stroke(isActive ? Color.orange.opacity(0.4) : Color.clear, lineWidth: 1)
                )
        )
    }

    private var countdownBadge: some View {
        let remaining = max(0, challenge.endDate.timeIntervalSince(.now))
        let days = Int(remaining / 86400)
        let hours = Int((remaining.truncatingRemainder(dividingBy: 86400)) / 3600)
        let text = days > 0 ? "\(days)d left" : "\(hours)h left"

        return Label(text, systemImage: "timer")
            .font(.system(size: 10, weight: .bold, design: .monospaced))
            .foregroundStyle(.orange)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Capsule().fill(Color.orange.opacity(0.15)))
    }
}

// MARK: - New Challenge Sheet

struct NewChallengeSheet: View {
    let friends: [Friend]
    let onSend: (String, String) -> Void

    @State private var selectedFriendId = ""
    @State private var selectedType = "xp_earned"
    @Environment(\.dismiss) var dismiss

    private let challengeTypes = [
        ("xp_earned", "Most XP Earned", "star.fill"),
        ("commits", "Most Commits", "arrow.triangle.branch"),
        ("tests_written", "Most Tests Written", "checkmark.shield.fill"),
        ("deploys", "Most Deploys", "paperplane.fill"),
        ("hours_coded", "Most Hours Coded", "clock.fill")
    ]

    var body: some View {
        NavigationStack {
            Form {
                Section("Challenge") {
                    Picker("Friend", selection: $selectedFriendId) {
                        ForEach(friends) { friend in
                            Text(friend.username).tag(friend.id)
                        }
                    }
                    .onAppear { selectedFriendId = friends.first?.id ?? "" }
                }

                Section("Type (this week)") {
                    ForEach(challengeTypes, id: \.0) { type in
                        HStack {
                            Image(systemName: type.2)
                                .foregroundStyle(.cyan)
                                .frame(width: 24)
                            Text(type.1)
                            Spacer()
                            if selectedType == type.0 {
                                Image(systemName: "checkmark")
                                    .foregroundStyle(.cyan)
                            }
                        }
                        .contentShape(Rectangle())
                        .onTapGesture { selectedType = type.0 }
                    }
                }

                Section {
                    Text("The loser's avatar will wear a silly hat 🎩 until the next challenge.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .navigationTitle("New Challenge")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Send") {
                        onSend(selectedFriendId, selectedType)
                    }
                    .fontWeight(.bold)
                    .disabled(selectedFriendId.isEmpty)
                }
            }
        }
        .preferredColorScheme(.dark)
    }
}
