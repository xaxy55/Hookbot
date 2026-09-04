import Foundation

/// Turns a URLSession result into a message that names the cause.
///
/// Distinct from APIError in APIService.swift, which is a thrown Error type;
/// this only builds human-facing text from a completed response.
///
/// Every screen used to collapse each failure into "Failed to load X", which
/// hides the three things worth knowing apart: the credential was rejected, the
/// server returned an error, or the address answered but is not this API. They
/// need different actions from the person holding the phone, so they should not
/// read identically.
enum APIFailure {

    /// Returns nil when the response is a usable 200, otherwise a description.
    static func describe(
        _ what: String,
        host: String? = nil,
        data: Data?,
        response: URLResponse?,
        error: Error?
    ) -> String? {
        if let error {
            return "\(what): \(error.localizedDescription)"
        }
        guard let http = response as? HTTPURLResponse else {
            return "\(what): no response from the server."
        }
        guard http.statusCode != 200 else { return nil }

        let serverMessage = (data
            .flatMap { try? JSONSerialization.jsonObject(with: $0) as? [String: Any] })?["error"] as? String

        switch http.statusCode {
        case 401, 403:
            // The single most common cause, and the one where "failed to load"
            // sends people looking at the network instead of at sign-in.
            return "\(what): the server rejected this sign-in (HTTP \(http.statusCode)). "
                + "Open Settings, sign out, and sign in again."
        case 404:
            let where_ = host.map { " at \($0)" } ?? ""
            return "\(what): not found\(where_) (HTTP 404). Check the server address."
        default:
            if let serverMessage {
                return "\(what): \(serverMessage) (HTTP \(http.statusCode))"
            }
            return "\(what) (HTTP \(http.statusCode))"
        }
    }

    /// A 200 that could not be parsed almost always means the address is a web
    /// server rather than this API — a frontend-only host answers every path
    /// with its index.html, which is not JSON. Saying "failed to load" there
    /// sends people hunting for an outage that isn't happening.
    static func describeUnreadable(_ what: String, host: String?, data: Data?) -> String {
        let looksLikeHTML = data
            .flatMap { String(data: $0.prefix(200), encoding: .utf8) }?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .hasPrefix("<") ?? false

        if looksLikeHTML {
            let where_ = host ?? "that address"
            return "\(what): \(where_) returned a web page, not API data. "
                + "It is probably the dashboard's address without the API behind it."
        }
        return "\(what): the server's response could not be read."
    }
}
