import SwiftUI

// Team campfire — all team avatars gather around a campfire; more people = bigger fire (Phase 13.4)

struct CampfireMember: Identifiable {
    let id: String
    let username: String
    let avatarState: String
    let isActive: Bool

    var moodEmoji: String {
        switch avatarState {
        case "thinking": return "🤔"
        case "success":  return "😎"
        case "error":    return "🤬"
        case "waiting":  return "😤"
        default:         return "😏"
        }
    }
}

struct TeamCampfireView: View {
    @EnvironmentObject var engine: AvatarEngine
    @StateObject private var social = SocialService.shared
    @State private var campfireIntensity: CGFloat = 0.3
    @State private var members: [CampfireMember] = []
    @State private var firePhase: CGFloat = 0
    let fireTimer = Timer.publish(every: 0.05, on: .main, in: .common).autoconnect()

    var activeMemberCount: Int { members.filter { $0.isActive }.count }

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                campfireScene
                membersGrid
                statsRow
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Team Campfire")
        .navigationBarTitleDisplayMode(.inline)
        .preferredColorScheme(.dark)
        .onAppear {
            social.configure(engine: engine)
            loadMembers()
        }
        .onReceive(fireTimer) { _ in
            firePhase += 0.05
            campfireIntensity = 0.3 + CGFloat(activeMemberCount) * 0.1 + CGFloat(sin(firePhase)) * 0.05
        }
    }

    // MARK: - Campfire scene

    private var campfireScene: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 20)
                .fill(Color(red: 0.08, green: 0.04, blue: 0.02))
                .frame(height: 240)

            // Fire
            Canvas { ctx, size in
                drawFire(ctx: &ctx, size: size, phase: firePhase, intensity: campfireIntensity)
            }
            .frame(height: 200)

            // Sitting avatars around campfire
            VStack {
                Spacer()
                HStack(spacing: 0) {
                    ForEach(members.prefix(6)) { member in
                        Text(member.moodEmoji)
                            .font(.system(size: 24))
                            .opacity(member.isActive ? 1 : 0.4)
                            .scaleEffect(member.isActive ? 1.0 : 0.8)
                    }
                    if members.count > 6 {
                        Text("+\(members.count - 6)")
                            .font(.system(size: 12, weight: .bold, design: .monospaced))
                            .foregroundStyle(.orange)
                    }
                }
                .padding(.bottom, 12)
            }

            // Fire intensity label
            VStack {
                HStack {
                    Spacer()
                    Label("\(activeMemberCount) coding", systemImage: "flame.fill")
                        .font(.system(size: 11, weight: .bold, design: .monospaced))
                        .foregroundStyle(.orange)
                        .padding(8)
                        .background(.ultraThinMaterial)
                        .clipShape(Capsule())
                        .padding(10)
                }
                Spacer()
            }
        }
    }

    private func drawFire(ctx: inout GraphicsContext, size: CGSize, phase: CGFloat, intensity: CGFloat) {
        let colors: [Color] = [.red, .orange, .yellow, .white]
        let cx = size.width / 2
        let baseY = size.height * 0.85

        for layer in 0..<8 {
            let fl = CGFloat(layer)
            let flameH = intensity * size.height * 0.7 * (1.0 - fl * 0.1)
            let flameW = intensity * size.width * 0.18 * (1.0 - fl * 0.08)
            let colorIdx = min(Int(fl / 2.0), colors.count - 1)
            let xOff = CGFloat(sin(Double(phase) + Double(layer) * 0.7)) * flameW * 0.4

            var path = Path()
            path.move(to: CGPoint(x: cx - flameW / 2 + xOff, y: baseY))
            path.addCurve(
                to: CGPoint(x: cx + xOff, y: baseY - flameH),
                control1: CGPoint(x: cx - flameW * 0.3 + xOff, y: baseY - flameH * 0.5),
                control2: CGPoint(x: cx - flameW * 0.1 + xOff, y: baseY - flameH * 0.8)
            )
            path.addCurve(
                to: CGPoint(x: cx + flameW / 2 + xOff, y: baseY),
                control1: CGPoint(x: cx + flameW * 0.1 + xOff, y: baseY - flameH * 0.8),
                control2: CGPoint(x: cx + flameW * 0.3 + xOff, y: baseY - flameH * 0.5)
            )
            path.closeSubpath()

            let alpha = 0.7 - fl * 0.08
            ctx.fill(path, with: .color(colors[colorIdx].opacity(alpha)))
        }

        // Logs
        ctx.fill(Path(CGRect(x: cx - 30, y: baseY - 4, width: 60, height: 8)),
                 with: .color(Color(red: 0.3, green: 0.15, blue: 0.05)))
    }

    // MARK: - Members grid

    private var membersGrid: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("TEAM MEMBERS")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)

            LazyVGrid(columns: [.init(.flexible()), .init(.flexible())], spacing: 10) {
                ForEach(members) { member in
                    HStack(spacing: 10) {
                        Text(member.moodEmoji)
                            .font(.system(size: 24))
                            .opacity(member.isActive ? 1 : 0.4)

                        VStack(alignment: .leading, spacing: 2) {
                            Text(member.username)
                                .font(.system(size: 12, weight: .bold, design: .monospaced))
                                .foregroundStyle(member.isActive ? .white : .gray)
                                .lineLimit(1)
                            Text(member.isActive ? "Coding" : "Away")
                                .font(.system(size: 10, design: .monospaced))
                                .foregroundStyle(member.isActive ? .green : .gray)
                        }
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.08)))
                }
            }
        }
    }

    // MARK: - Stats row

    private var statsRow: some View {
        HStack(spacing: 12) {
            campfireStat(value: "\(members.count)", label: "Total", icon: "person.3.fill", color: .blue)
            campfireStat(value: "\(activeMemberCount)", label: "Active", icon: "flame.fill", color: .orange)
            campfireStat(value: fireLevel, label: "Fire", icon: "sparkles", color: .yellow)
        }
    }

    private func campfireStat(value: String, label: String, icon: String, color: Color) -> some View {
        VStack(spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 20))
                .foregroundStyle(color)
            Text(value)
                .font(.system(size: 20, weight: .black, design: .monospaced))
                .foregroundStyle(.white)
            Text(label)
                .font(.system(size: 10, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .frame(maxWidth: .infinity)
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private var fireLevel: String {
        switch activeMemberCount {
        case 0:     return "Ember"
        case 1...2: return "Small"
        case 3...5: return "Roaring"
        default:    return "INFERNO"
        }
    }

    // MARK: - Load

    private func loadMembers() {
        // Build from friends + self
        var all: [CampfireMember] = [
            CampfireMember(
                id: engine.config.deviceId,
                username: "You",
                avatarState: engine.currentState.rawValue,
                isActive: engine.currentState != .idle
            )
        ]
        for friend in social.friends {
            all.append(CampfireMember(
                id: friend.id,
                username: friend.username,
                avatarState: friend.avatarState,
                isActive: friend.isOnline
            ))
        }
        members = all
    }
}
