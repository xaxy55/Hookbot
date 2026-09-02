import SwiftUI

/// Sign in by scanning the pairing code from the web dashboard
/// (Account → Pair Phone) instead of typing a server URL and password.
struct PairPhoneView: View {
    @EnvironmentObject var engine: AvatarEngine
    @EnvironmentObject var network: NetworkService
    @ObservedObject var auth: AuthService
    @Environment(\.dismiss) var dismiss

    @StateObject private var pairing = PairingService()

    var body: some View {
        NavigationStack {
            ZStack {
                Color.black.ignoresSafeArea()

                switch pairing.state {
                case .idle:
                    scanner
                case .redeeming:
                    redeemingContent
                case .paired(let credential):
                    pairedContent(credential)
                case .failed(let message):
                    failedContent(message)
                }
            }
            .navigationTitle("Pair Phone")
            .navigationBarTitleDisplayMode(.inline)
            .preferredColorScheme(.dark)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Cancel") { dismiss() }
                        .font(.system(size: 14, design: .monospaced))
                }
            }
        }
    }

    // MARK: - Scanning

    private var scanner: some View {
        ZStack(alignment: .top) {
            QRScannerView(
                prompt: "Scan the code from Account → Pair Phone",
                validate: { PairingService.parse($0) != nil ? $0 : nil },
                onCodeScanned: { scanned in
                    guard let payload = PairingService.parse(scanned) else { return }
                    Task { await pairing.redeem(payload) }
                }
            )
            .ignoresSafeArea()

            Text("The code expires two minutes after the dashboard shows it.")
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 32)
                .padding(.top, 12)
        }
    }

    // MARK: - Outcomes

    private func pairedContent(_ credential: PairingService.Credential) -> some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 50))
                .foregroundStyle(.green)
            Text("Paired")
                .font(.system(size: 20, weight: .bold, design: .monospaced))
                .foregroundStyle(.green)
            if let email = credential.email, !email.isEmpty {
                Text(email)
                    .font(.system(size: 13, design: .monospaced))
                    .foregroundStyle(.white)
            }
            Text(credential.serverURL)
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
        }
        .padding(32)
        .onAppear { signIn(with: credential) }
    }

    private func failedContent(_ message: String) -> some View {
        VStack(spacing: 16) {
            Image(systemName: "xmark.circle.fill")
                .font(.system(size: 44))
                .foregroundStyle(.red)
            Text("Pairing Failed")
                .font(.system(size: 18, weight: .bold, design: .monospaced))
                .foregroundStyle(.white)
            Text(message)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)

            Button("Scan Again") { pairing.reset() }
                .font(.system(size: 15, weight: .bold, design: .monospaced))
                .buttonStyle(.borderedProminent)
                .tint(.green)
        }
        .padding(32)
    }

    private var redeemingContent: some View {
        VStack(spacing: 16) {
            ProgressView()
                .controlSize(.large)
                .tint(.green)
            Text("Pairing...")
                .font(.system(size: 15, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .padding(40)
    }

    // MARK: - Credential storage

    /// Store the credential exactly the way the login screen does, so every other
    /// screen and the API service pick it up unchanged.
    private func signIn(with credential: PairingService.Credential) {
        engine.config.serverURL = credential.serverURL
        engine.config.apiKey = credential.apiKey
        if let data = try? JSONEncoder().encode(engine.config) {
            UserDefaults.standard.set(data, forKey: "hookbot_config")
        }
        APIService.configure(serverURL: credential.serverURL, apiKey: credential.apiKey)

        auth.errorMessage = nil
        auth.isAuthenticated = true
        network.start(engine: engine)

        // Let the confirmation land before handing the app back.
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) {
            dismiss()
        }
    }
}
