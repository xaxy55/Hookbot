import Foundation

/// Signs the app in from a QR code shown in the web dashboard, so nobody has to
/// type a server address and a password on a phone keyboard.
///
/// The QR carries `hookbot://pair?server=<url>&token=<one-time token>`. The token
/// is short-lived and single-use, so each way it can fail gets its own message —
/// "expired" and "already used" mean different things to the person holding the phone.
@MainActor
final class PairingService: ObservableObject {

    // MARK: - Types

    struct Payload: Equatable {
        let serverURL: String
        let token: String
    }

    struct Credential: Equatable {
        let serverURL: String
        let apiKey: String
        let email: String?
    }

    enum State: Equatable {
        case idle
        case redeeming
        case paired(Credential)
        case failed(String)
    }

    // MARK: - State

    @Published var state: State = .idle

    private let session: URLSession

    init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 15
        self.session = URLSession(configuration: config)
    }

    // MARK: - Payload parsing

    /// Parse a scanned string. Returns nil for anything that is not a pairing code,
    /// which keeps the scanner running rather than failing on a stray QR code.
    static func parse(_ raw: String) -> Payload? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let components = URLComponents(string: trimmed),
              components.scheme?.lowercased() == "hookbot",
              components.host?.lowercased() == "pair",
              let items = components.queryItems else { return nil }

        guard let server = items.first(where: { $0.name == "server" })?.value,
              let token = items.first(where: { $0.name == "token" })?.value,
              !server.isEmpty, !token.isEmpty else { return nil }

        return Payload(serverURL: normalize(server), token: token)
    }

    /// Same normalisation the login screen applies to a hand-typed server URL.
    static func normalize(_ url: String) -> String {
        var trimmed = url.trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        if !trimmed.hasPrefix("https://") && !trimmed.hasPrefix("http://") {
            trimmed = "https://\(trimmed)"
        }
        return trimmed
    }

    // MARK: - Redemption

    func redeem(_ payload: Payload) async {
        state = .redeeming

        guard let url = URL(string: "\(payload.serverURL)/api/auth/pair/redeem") else {
            state = .failed("That code carries an address this app can't use.")
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try? JSONSerialization.data(withJSONObject: ["token": payload.token])

        do {
            let (data, response) = try await session.data(for: request)
            guard let http = response as? HTTPURLResponse else {
                state = .failed("No response from \(host(of: payload.serverURL)).")
                return
            }
            let json = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? [:]

            switch http.statusCode {
            case 200:
                guard let apiKey = json["api_key"] as? String, !apiKey.isEmpty else {
                    state = .failed("The server paired but sent no credential.")
                    return
                }
                state = .paired(Credential(
                    serverURL: payload.serverURL,
                    apiKey: apiKey,
                    email: json["email"] as? String
                ))
            case 404:
                state = .failed("This code isn't recognised. Show a new one in the dashboard and scan again.")
            case 409:
                state = .failed("This code has already been used. Show a new one in the dashboard and scan again.")
            case 410:
                state = .failed("This code has expired. Codes last two minutes — show a new one and scan again.")
            case 429:
                state = .failed("Too many pairing attempts. Wait a minute, then try again.")
            default:
                let message = json["error"] as? String ?? "Pairing failed (HTTP \(http.statusCode))."
                state = .failed(message)
            }
        } catch {
            state = .failed("Couldn't reach \(host(of: payload.serverURL)). Check that the server is up and this phone can reach it.")
        }
    }

    func reset() {
        state = .idle
    }

    private func host(of urlString: String) -> String {
        URL(string: urlString)?.host ?? urlString
    }
}
