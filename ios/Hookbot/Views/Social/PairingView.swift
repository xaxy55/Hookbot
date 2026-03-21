import SwiftUI
import CoreNFC

// Buddy phone pairing — QR code or NFC tap (Phase 13.2)

struct PairingView: View {
    @EnvironmentObject var engine: AvatarEngine
    @StateObject private var social = SocialService.shared
    @State private var manualCode = ""
    @State private var showManualEntry = false
    @FocusState private var codeFieldFocused: Bool

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 24) {
                    header

                    switch social.pairingState {
                    case .idle:
                        idleContent
                    case .generating(let code, let qrImage):
                        generatingContent(code: code, qrImage: qrImage)
                    case .waiting:
                        waitingContent
                    case .paired(let friend):
                        pairedContent(friend: friend)
                    case .failed(let error):
                        failedContent(error: error)
                    }
                }
                .padding()
            }
            .background(Color.black)
            .navigationTitle("Pair with Buddy")
            .navigationBarTitleDisplayMode(.inline)
            .preferredColorScheme(.dark)
            .onAppear {
                social.configure(engine: engine)
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        VStack(spacing: 8) {
            Text("👥")
                .font(.system(size: 60))
            Text("Link two phones via QR code or NFC tap.")
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
        }
    }

    // MARK: - Idle state

    private var idleContent: some View {
        VStack(spacing: 16) {
            // Generate QR button
            actionCard(
                icon: "qrcode",
                title: "Show my QR code",
                subtitle: "Let your buddy scan to pair"
            ) {
                social.generatePairingCode()
            }

            // NFC pair button (if supported)
            if NFCTagReaderSession.readingAvailable {
                actionCard(
                    icon: "wave.3.right.circle",
                    title: "NFC tap to pair",
                    subtitle: "Hold phones back-to-back"
                ) {
                    startNFCPairing()
                }
            }

            // Manual code entry
            actionCard(
                icon: "keyboard",
                title: "Enter pairing code",
                subtitle: "Type the code your buddy sees"
            ) {
                showManualEntry = true
            }

            if showManualEntry {
                manualCodeEntry
            }
        }
    }

    private var manualCodeEntry: some View {
        VStack(spacing: 12) {
            TextField("Enter 6-digit code", text: $manualCode)
                .keyboardType(.asciiCapable)
                .textInputAutocapitalization(.characters)
                .autocorrectionDisabled()
                .font(.system(size: 24, weight: .bold, design: .monospaced))
                .multilineTextAlignment(.center)
                .padding()
                .background(Color(white: 0.1))
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .focused($codeFieldFocused)
                .onAppear { codeFieldFocused = true }

            Button {
                social.acceptPairingCode(manualCode.uppercased())
                showManualEntry = false
            } label: {
                Text("Pair")
                    .font(.system(size: 16, weight: .bold, design: .monospaced))
                    .foregroundStyle(.black)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.cyan)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .disabled(manualCode.count < 4)
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 16).fill(Color(white: 0.08)))
    }

    // MARK: - Generating state

    private func generatingContent(code: String, qrImage: UIImage?) -> some View {
        VStack(spacing: 20) {
            Text("Your pairing code")
                .font(.system(size: 13, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)

            // QR Code
            if let qrImage {
                Image(uiImage: qrImage)
                    .interpolation(.none)
                    .resizable()
                    .scaledToFit()
                    .frame(width: 200, height: 200)
                    .padding(8)
                    .background(Color.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }

            // Code display
            Text(code)
                .font(.system(size: 36, weight: .black, design: .monospaced))
                .tracking(8)
                .foregroundStyle(.cyan)
                .padding(.vertical, 16)
                .frame(maxWidth: .infinity)
                .background(Color(white: 0.08))
                .clipShape(RoundedRectangle(cornerRadius: 12))

            Text("Waiting for your buddy to scan or enter this code...")
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)

            ProgressView()
                .tint(.cyan)

            Button("Cancel") {
                social.pairingState = .idle
            }
            .font(.system(size: 13, design: .monospaced))
            .foregroundStyle(.red)
        }
    }

    // MARK: - Waiting

    private var waitingContent: some View {
        VStack(spacing: 16) {
            ProgressView()
                .controlSize(.large)
                .tint(.cyan)
            Text("Connecting...")
                .font(.system(size: 14, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .padding(40)
    }

    // MARK: - Paired

    private func pairedContent(friend: Friend) -> some View {
        VStack(spacing: 16) {
            Text("🎉")
                .font(.system(size: 60))
            Text("Paired with \(friend.username)!")
                .font(.system(size: 20, weight: .bold, design: .monospaced))
                .foregroundStyle(.green)

            infoRow(label: "Username", value: friend.username)
            infoRow(label: "Streak", value: "\(friend.streak) days")
            infoRow(label: "XP", value: "\(friend.xp)")

            Button {
                social.pairingState = .idle
            } label: {
                Text("Done")
                    .font(.system(size: 16, weight: .bold, design: .monospaced))
                    .foregroundStyle(.black)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.green)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
        }
        .padding(20)
        .background(RoundedRectangle(cornerRadius: 16).fill(Color(white: 0.08)))
    }

    // MARK: - Failed

    private func failedContent(error: String) -> some View {
        VStack(spacing: 12) {
            Image(systemName: "xmark.circle.fill")
                .font(.system(size: 40))
                .foregroundStyle(.red)
            Text("Pairing failed")
                .font(.system(size: 18, weight: .bold, design: .monospaced))
            Text(error)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)

            Button("Try Again") {
                social.pairingState = .idle
            }
            .buttonStyle(.borderedProminent)
            .tint(.cyan)
        }
    }

    // MARK: - Components

    private func actionCard(icon: String, title: String, subtitle: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 16) {
                Image(systemName: icon)
                    .font(.system(size: 24))
                    .foregroundStyle(.cyan)
                    .frame(width: 40)

                VStack(alignment: .leading, spacing: 3) {
                    Text(title)
                        .font(.system(size: 15, weight: .semibold, design: .monospaced))
                        .foregroundStyle(.white)
                    Text(subtitle)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.gray)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .foregroundStyle(.gray)
            }
            .padding(16)
            .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
        }
        .buttonStyle(.plain)
    }

    private func infoRow(label: String, value: String) -> some View {
        HStack {
            Text(label)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
            Spacer()
            Text(value)
                .font(.system(size: 12, weight: .bold, design: .monospaced))
                .foregroundStyle(.white)
        }
        .padding(.horizontal)
    }

    // MARK: - NFC

    private func startNFCPairing() {
        // NFC tag reader session for proximity detection
        // On a real device, read NFC tag containing pairing code
        social.generatePairingCode()
    }
}

// MARK: - Avatar Walk-Over Animation View

struct AvatarWalkoverView: View {
    let fromName: String
    let message: String
    @Binding var isVisible: Bool

    @State private var avatarX: CGFloat = -100
    @State private var opacity: Double = 0

    var body: some View {
        ZStack {
            Color.black.opacity(0.85).ignoresSafeArea()

            VStack(spacing: 20) {
                Text("👋 \(fromName)'s avatar is visiting!")
                    .font(.system(size: 16, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)

                // Animated avatar walking across
                ZStack(alignment: .leading) {
                    Color.clear.frame(height: 100)
                    AvatarWalkSprite()
                        .frame(width: 60, height: 60)
                        .offset(x: avatarX)
                }
                .frame(maxWidth: .infinity)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .background(Color(white: 0.08))

                if !message.isEmpty {
                    Text(message)
                        .font(.system(size: 14, design: .monospaced))
                        .foregroundStyle(.cyan)
                        .padding()
                        .background(Color(white: 0.1))
                        .clipShape(RoundedRectangle(cornerRadius: 10))
                }

                Button("Say Hi Back") {
                    withAnimation { isVisible = false }
                }
                .buttonStyle(.borderedProminent)
                .tint(.cyan)
            }
            .padding(24)
        }
        .opacity(opacity)
        .onAppear {
            withAnimation(.easeIn(duration: 0.3)) { opacity = 1 }
            withAnimation(.easeInOut(duration: 2.0)) {
                avatarX = UIScreen.main.bounds.width - 80
            }
        }
    }
}

struct AvatarWalkSprite: View {
    @State private var step = false
    let timer = Timer.publish(every: 0.3, on: .main, in: .common).autoconnect()

    var body: some View {
        ZStack {
            // Simple walking avatar
            Circle()
                .fill(Color.white.opacity(0.9))
                .frame(width: 30, height: 30)
                .offset(y: -10)

            // Legs
            Rectangle()
                .fill(Color.white)
                .frame(width: 4, height: 15)
                .offset(x: step ? -6 : -4, y: 15)
                .rotationEffect(.degrees(step ? -15 : 15))

            Rectangle()
                .fill(Color.white)
                .frame(width: 4, height: 15)
                .offset(x: step ? 6 : 4, y: 15)
                .rotationEffect(.degrees(step ? 15 : -15))
        }
        .onReceive(timer) { _ in
            withAnimation(.easeInOut(duration: 0.3)) {
                step.toggle()
            }
        }
    }
}
