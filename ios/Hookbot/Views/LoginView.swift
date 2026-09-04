import SwiftUI

struct LoginView: View {
    @EnvironmentObject var engine: AvatarEngine
    @EnvironmentObject var network: NetworkService
    @ObservedObject var auth: AuthService

    @State private var serverURL: String = ""
    @State private var showManualEntry = false
    @State private var manualAPIKey: String = ""
    @State private var password: String = ""
    @State private var showPairing = false
    /// nil until the server has been asked. Servers without WorkOS answer 404
    /// on /auth/login, so offering that button there is a dead end.
    @State private var workosEnabled: Bool? = nil

    /// Comes from Info.plist, which Xcode Cloud fills from the
    /// HOOKBOT_SERVER_URL environment variable. No host is committed: the repo
    /// is public. Empty means the user types their own server on the login
    /// screen, which is the right default for a self-hosted install anyway.
    private var defaultServerURL: String {
        let configured = Bundle.main.object(forInfoDictionaryKey: "HookbotServerURL") as? String
        guard let configured, !configured.isEmpty, !configured.hasPrefix("$(") else { return "" }
        return configured
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

                // Pair by QR — no server URL or password to type
                Button {
                    showPairing = true
                } label: {
                    HStack(spacing: 10) {
                        Image(systemName: "qrcode.viewfinder")
                        Text("Scan pairing code")
                            .font(.system(size: 17, weight: .semibold, design: .monospaced))
                    }
                    .frame(maxWidth: .infinity)
                    .padding(14)
                    .background(Color.green)
                    .foregroundColor(.black)
                    .cornerRadius(10)
                }
                .padding(.horizontal, 32)

                Text("Open the dashboard → Account → Pair Phone")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundColor(Color(white: 0.4))

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

                // Password sign-in. Single-admin servers have no WorkOS, so
                // without this the only ways in are scanning a code from
                // another screen or typing a long API key.
                if workosEnabled == false {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("PASSWORD")
                            .font(.system(size: 10, weight: .bold, design: .monospaced))
                            .foregroundColor(Color(white: 0.5))
                        SecureField("Admin password", text: $password)
                            .font(.system(.body, design: .monospaced))
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .padding(12)
                            .background(Color(white: 0.1))
                            .cornerRadius(8)
                            .foregroundColor(.white)
                            .submitLabel(.go)
                            .onSubmit { signInWithPassword() }

                        Button {
                            signInWithPassword()
                        } label: {
                            HStack(spacing: 10) {
                                if auth.isLoading {
                                    ProgressView().tint(.black)
                                } else {
                                    Image(systemName: "key.fill")
                                }
                                Text("Sign in")
                                    .font(.system(size: 17, weight: .semibold, design: .monospaced))
                            }
                            .frame(maxWidth: .infinity)
                            .padding(14)
                            .background(Color.white)
                            .foregroundColor(.black)
                            .cornerRadius(10)
                        }
                        .disabled(serverURL.isEmpty || password.isEmpty || auth.isLoading)
                    }
                    .padding(.horizontal, 32)
                }

                // Sign in button — only where the server actually supports it.
                if workosEnabled != false {
                Button {
                    auth.login(serverURL: serverURL) { apiKey, email in
                        guard let apiKey else { return }
                        engine.config.apiKey = apiKey
                        var normalizedURL = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
                            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
                        if !normalizedURL.hasPrefix("https://") && !normalizedURL.hasPrefix("http://") {
                            normalizedURL = "https://\(normalizedURL)"
                        }
                        engine.config.serverURL = normalizedURL
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
                } else {
                    Text("Or use a pairing code or API key.")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundColor(Color(white: 0.4))
                        .padding(.horizontal, 32)
                }

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
                            var normalizedURL = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
                                .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
                            if !normalizedURL.hasPrefix("https://") && !normalizedURL.hasPrefix("http://") {
                                normalizedURL = "https://\(normalizedURL)"
                            }
                            engine.config.serverURL = normalizedURL
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
        .sheet(isPresented: $showPairing) {
            PairPhoneView(auth: auth)
                .environmentObject(engine)
                .environmentObject(network)
        }
        .onAppear {
            if serverURL.isEmpty {
                serverURL = defaultServerURL
            }
            probeServerMode()
        }
        .onChange(of: serverURL) { _, _ in
            workosEnabled = nil
            probeServerMode()
        }
    }

    private func signInWithPassword() {
        guard !serverURL.isEmpty, !password.isEmpty else { return }
        auth.loginWithPassword(serverURL: serverURL, password: password) { apiKey in
            guard let apiKey else { return }
            engine.config.apiKey = apiKey
            engine.config.serverURL = AuthService.normalizeServerURL(serverURL)
            if let data = try? JSONEncoder().encode(engine.config) {
                UserDefaults.standard.set(data, forKey: "hookbot_config")
            }
            // The password itself is never stored — the server handed back a
            // revocable token, and that is all the device keeps.
            password = ""
            network.start(engine: engine)
        }
    }

    /// Ask the server which sign-in methods it actually has. Anything other
    /// than a clear "WorkOS is on" leaves the button hidden: a self-hosted
    /// server 404s that flow, and a dead button is worse than one fewer option.
    private func probeServerMode() {
        let base = PairingService.normalize(serverURL)
        guard !serverURL.isEmpty, let url = URL(string: "\(base)/api/auth/status") else { return }

        URLSession.shared.dataTask(with: url) { data, _, _ in
            let enabled: Bool = {
                guard let data,
                      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
                else { return false }
                return json["workos_enabled"] as? Bool ?? false
            }()
            DispatchQueue.main.async { workosEnabled = enabled }
        }.resume()
    }
}
