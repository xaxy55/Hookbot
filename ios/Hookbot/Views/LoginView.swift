import SwiftUI

struct LoginView: View {
    @EnvironmentObject var engine: AvatarEngine
    @EnvironmentObject var network: NetworkService
    @ObservedObject var auth: AuthService

    @State private var serverURL: String = ""
    @State private var showManualEntry = false
    @State private var manualAPIKey: String = ""

    private var defaultServerURL: String {
        Bundle.main.object(forInfoDictionaryKey: "HookbotServerURL") as? String
            ?? "https://hookbot.mr-ai.no"
    }

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            VStack(spacing: 32) {
                Spacer()

                // Logo area
                VStack(spacing: 12) {
                    Image(systemName: "desktopcomputer")
                        .font(.system(size: 64))
                        .foregroundColor(.white)
                    Text("HOOKBOT")
                        .font(.system(size: 28, weight: .black, design: .monospaced))
                        .foregroundColor(.white)
                    Text("DESTROYER OF WORLDS")
                        .font(.system(size: 11, weight: .medium, design: .monospaced))
                        .foregroundColor(Color(white: 0.4))
                }

                Spacer()

                // Server URL field
                VStack(alignment: .leading, spacing: 8) {
                    Text("SERVER")
                        .font(.system(size: 10, weight: .bold, design: .monospaced))
                        .foregroundColor(Color(white: 0.5))
                    TextField("Server URL", text: $serverURL)
                        .font(.system(.body, design: .monospaced))
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .padding(12)
                        .background(Color(white: 0.1))
                        .cornerRadius(8)
                        .foregroundColor(.white)
                }
                .padding(.horizontal, 32)

                // Sign in button
                Button {
                    auth.login(serverURL: serverURL) { apiKey, email in
                        guard let apiKey else { return }
                        engine.config.apiKey = apiKey
                        engine.config.serverURL = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
                            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
                        if let data = try? JSONEncoder().encode(engine.config) {
                            UserDefaults.standard.set(data, forKey: "hookbot_config")
                        }
                        network.start(engine: engine)
                    }
                } label: {
                    HStack(spacing: 10) {
                        if auth.isLoading {
                            ProgressView()
                                .tint(.black)
                        } else {
                            Image(systemName: "person.badge.key")
                        }
                        Text("Sign in with WorkOS")
                            .font(.system(size: 17, weight: .semibold, design: .monospaced))
                    }
                    .frame(maxWidth: .infinity)
                    .padding(14)
                    .background(Color.white)
                    .foregroundColor(.black)
                    .cornerRadius(10)
                }
                .disabled(serverURL.isEmpty || auth.isLoading)
                .padding(.horizontal, 32)

                // Manual API key toggle
                Button {
                    showManualEntry.toggle()
                } label: {
                    Text(showManualEntry ? "Hide manual entry" : "Use API key instead")
                        .font(.system(size: 13, design: .monospaced))
                        .foregroundColor(Color(white: 0.5))
                }

                if showManualEntry {
                    VStack(spacing: 12) {
                        TextField("API Key", text: $manualAPIKey)
                            .font(.system(.body, design: .monospaced))
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .padding(12)
                            .background(Color(white: 0.1))
                            .cornerRadius(8)
                            .foregroundColor(.white)

                        Button {
                            engine.config.apiKey = manualAPIKey
                            engine.config.serverURL = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
                                .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
                            if let data = try? JSONEncoder().encode(engine.config) {
                                UserDefaults.standard.set(data, forKey: "hookbot_config")
                            }
                            auth.isAuthenticated = true
                            network.start(engine: engine)
                        } label: {
                            Text("Connect")
                                .font(.system(size: 17, weight: .semibold, design: .monospaced))
                                .frame(maxWidth: .infinity)
                                .padding(14)
                                .background(Color(white: 0.2))
                                .foregroundColor(.white)
                                .cornerRadius(10)
                        }
                        .disabled(manualAPIKey.isEmpty || serverURL.isEmpty)
                    }
                    .padding(.horizontal, 32)
                }

                if let error = auth.errorMessage {
                    Text(error)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundColor(.red)
                        .padding(.horizontal, 32)
                }

                Spacer()
            }
        }
        .onAppear {
            if serverURL.isEmpty {
                serverURL = defaultServerURL
            }
        }
    }
}
