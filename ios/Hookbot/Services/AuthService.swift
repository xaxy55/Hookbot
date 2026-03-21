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

    func logout() {
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
