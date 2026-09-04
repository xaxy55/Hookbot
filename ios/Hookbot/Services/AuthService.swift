import AuthenticationServices
import Foundation

/// Handles WorkOS OAuth2 login via ASWebAuthenticationSession.
final class AuthService: NSObject, ObservableObject, ASWebAuthenticationPresentationContextProviding {
    @Published var isAuthenticated = false
    @Published var isLoading = false
    @Published var errorMessage: String?

    private let callbackScheme = "hookbot"

    /// Check if we have stored credentials
    func checkExistingAuth(config: RuntimeConfig) -> Bool {
        !config.apiKey.isEmpty && !config.serverURL.isEmpty
    }

    /// Start the WorkOS OAuth flow via ASWebAuthenticationSession
    func login(serverURL: String, completion: @escaping (String?, String?) -> Void) {
        var trimmed = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))

        // Ensure the server URL has an https scheme
        if !trimmed.hasPrefix("https://") && !trimmed.hasPrefix("http://") {
            trimmed = "https://\(trimmed)"
        }

        guard var components = URLComponents(string: "\(trimmed)/auth/login") else {
            errorMessage = "Invalid server URL"
            completion(nil, nil)
            return
        }
        components.queryItems = [
            URLQueryItem(name: "mobile_redirect", value: "\(callbackScheme)://auth/callback")
        ]
        guard let loginURL = components.url else {
            errorMessage = "Invalid server URL"
            completion(nil, nil)
            return
        }

        isLoading = true
        errorMessage = nil

        let session = ASWebAuthenticationSession(
            url: loginURL,
            callbackURLScheme: callbackScheme
        ) { [weak self] callbackURL, error in
            DispatchQueue.main.async {
                self?.isLoading = false

                if let error = error as? ASWebAuthenticationSessionError,
                   error.code == .canceledLogin {
                    completion(nil, nil)
                    return
                }

                if let error {
                    self?.errorMessage = error.localizedDescription
                    completion(nil, nil)
                    return
                }

                guard let callbackURL,
                      let components = URLComponents(url: callbackURL, resolvingAgainstBaseURL: false),
                      let items = components.queryItems else {
                    self?.errorMessage = "Invalid callback"
                    completion(nil, nil)
                    return
                }

                let apiKey = items.first(where: { $0.name == "api_key" })?.value
                let email = items.first(where: { $0.name == "email" })?.value

                if let apiKey, !apiKey.isEmpty {
                    self?.isAuthenticated = true
                    completion(apiKey, email)
                } else {
                    self?.errorMessage = "No API key received"
                    completion(nil, nil)
                }
            }
        }

        session.presentationContextProvider = self
        session.prefersEphemeralWebBrowserSession = false
        session.start()
    }

    /// Sign in with the server's admin password. Single-admin deployments have
    /// no WorkOS, so this is the only flow that does not require typing a long
    /// key on a phone keyboard or scanning a code from another screen.
    ///
    /// Same two steps the CLI uses: trade the password for a session cookie,
    /// then trade the cookie for a durable token. The server mints a revocable
    /// hb_ token for this, so the password is never stored on the device.
    func loginWithPassword(
        serverURL: String,
        password: String,
        completion: @escaping (String?) -> Void
    ) {
        let base = Self.normalizeServerURL(serverURL)
        guard let loginURL = URL(string: "\(base)/api/auth/login"),
              let meURL = URL(string: "\(base)/api/auth/me") else {
            errorMessage = "That server address doesn't look right."
            completion(nil)
            return
        }

        isLoading = true
        errorMessage = nil

        var request = URLRequest(url: loginURL)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try? JSONSerialization.data(withJSONObject: ["password": password])

        // The default session keeps the session cookie, which the second
        // request needs.
        let session = URLSession.shared

        session.dataTask(with: request) { [weak self] data, response, error in
            let finish: (String?, String?) -> Void = { key, message in
                DispatchQueue.main.async {
                    self?.isLoading = false
                    if let message { self?.errorMessage = message }
                    if let key { self?.isAuthenticated = true }
                    completion(key)
                }
            }

            if let error {
                finish(nil, "Couldn't reach \(URL(string: base)?.host ?? base): \(error.localizedDescription)")
                return
            }
            guard let http = response as? HTTPURLResponse else {
                finish(nil, "No response from the server.")
                return
            }
            let json = (data.flatMap { try? JSONSerialization.jsonObject(with: $0) }) as? [String: Any] ?? [:]

            switch http.statusCode {
            case 200:
                break
            case 401:
                finish(nil, "Wrong password.")
                return
            case 429:
                // Saying "try later" without a number is not actionable, and a
                // wrong password burns an attempt.
                let secs = json["retry_after_secs"] as? Int
                finish(nil, secs.map { "Too many attempts. Wait \($0)s and try again." }
                    ?? "Too many attempts. Try again shortly.")
                return
            default:
                finish(nil, (json["error"] as? String) ?? "Sign-in failed (HTTP \(http.statusCode)).")
                return
            }

            // Signed in; now collect the durable token.
            session.dataTask(with: meURL) { data, _, error in
                if let error {
                    finish(nil, "Signed in, but couldn't fetch a key: \(error.localizedDescription)")
                    return
                }
                guard let data,
                      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                      let key = json["api_key"] as? String, !key.isEmpty else {
                    finish(nil, "Signed in, but the server sent no key back.")
                    return
                }
                finish(key, nil)
            }.resume()
        }.resume()
    }

    /// Ask the server whether a stored credential is still good. Only an
    /// explicit "no" clears it: a server that is merely unreachable must not
    /// sign someone out, but a key the server rejects should not leave the app
    /// sitting on screens that all say "retry".
    func validateStoredCredential(
        serverURL: String,
        apiKey: String,
        completion: @escaping (Bool) -> Void
    ) {
        let base = Self.normalizeServerURL(serverURL)
        guard let url = URL(string: "\(base)/api/auth/status") else {
            completion(true)   // can't tell; leave it alone
            return
        }
        var request = URLRequest(url: url)
        request.setValue(apiKey, forHTTPHeaderField: "X-API-Key")

        URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                if error != nil || response == nil {
                    completion(true)   // offline, not rejected
                    return
                }
                guard let data,
                      let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                      let authenticated = json["authenticated"] as? Bool else {
                    completion(true)
                    return
                }
                completion(authenticated)
            }
        }.resume()
    }

    static func normalizeServerURL(_ url: String) -> String {
        var trimmed = url.trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        if !trimmed.hasPrefix("https://") && !trimmed.hasPrefix("http://") {
            trimmed = "https://\(trimmed)"
        }
        return trimmed
    }

    func logout() {
        errorMessage = nil
        isAuthenticated = false
    }

    // MARK: - ASWebAuthenticationPresentationContextProviding

    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        #if targetEnvironment(macCatalyst)
        return ASPresentationAnchor()
        #else
        return UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .flatMap { $0.windows }
            .first(where: { $0.isKeyWindow }) ?? ASPresentationAnchor()
        #endif
    }
}
