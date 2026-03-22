import Foundation

/// Shared async/await API service — eliminates boilerplate in every view.
/// Uses engine.config.serverURL + apiKey for all requests.
final class APIService {
    static let shared = APIService()

    private let session: URLSession

    private init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 15
        self.session = URLSession(configuration: config)
    }

    // MARK: - Config Access

    private var baseURL: String {
        UserDefaults.standard.string(forKey: "hookbot_server_url") ?? ""
    }

    private var apiKey: String {
        UserDefaults.standard.string(forKey: "hookbot_api_key") ?? ""
    }

    /// Configure from engine config (call once after login)
    static func configure(serverURL: String, apiKey: String) {
        UserDefaults.standard.set(serverURL, forKey: "hookbot_server_url")
        UserDefaults.standard.set(apiKey, forKey: "hookbot_api_key")
    }

    /// Get the device ID from stored config
    static var deviceId: String {
        if let data = UserDefaults.standard.data(forKey: "hookbot_config"),
           let config = try? JSONDecoder().decode(RuntimeConfig.self, from: data) {
            return config.deviceId
        }
        return ""
    }

    // MARK: - HTTP Methods

    func get<T: Decodable>(_ path: String, query: [String: String] = [:]) async throws -> T {
        let url = try buildURL(path, query: query)
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        applyAuth(&request)
        return try await execute(request)
    }

    func post<T: Decodable>(_ path: String, body: Encodable? = nil) async throws -> T {
        let url = try buildURL(path)
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        applyAuth(&request)
        if let body {
            request.httpBody = try JSONEncoder().encode(body)
        }
        return try await execute(request)
    }

    func put<T: Decodable>(_ path: String, body: Encodable? = nil) async throws -> T {
        let url = try buildURL(path)
        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        applyAuth(&request)
        if let body {
            request.httpBody = try JSONEncoder().encode(body)
        }
        return try await execute(request)
    }

    func delete(_ path: String) async throws {
        let url = try buildURL(path)
        var request = URLRequest(url: url)
        request.httpMethod = "DELETE"
        applyAuth(&request)
        let (_, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
            throw APIError.requestFailed
        }
    }

    /// POST without expecting a typed response
    func postRaw(_ path: String, body: [String: Any]) async throws -> [String: Any] {
        let url = try buildURL(path)
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        applyAuth(&request)
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
            throw APIError.requestFailed
        }
        return (try? JSONSerialization.jsonObject(with: data) as? [String: Any]) ?? [:]
    }

    // MARK: - Helpers

    private func buildURL(_ path: String, query: [String: String] = [:]) throws -> URL {
        let base = baseURL.isEmpty ? engineServerURL() : baseURL
        guard !base.isEmpty else { throw APIError.noServer }
        var urlStr = "\(base)/api\(path)"
        if !query.isEmpty {
            let qs = query.map { "\($0.key)=\($0.value)" }.joined(separator: "&")
            urlStr += "?\(qs)"
        }
        guard let url = URL(string: urlStr) else { throw APIError.invalidURL }
        return url
    }

    private func engineServerURL() -> String {
        if let data = UserDefaults.standard.data(forKey: "hookbot_config"),
           let config = try? JSONDecoder().decode(RuntimeConfig.self, from: data) {
            return config.serverURL
        }
        return ""
    }

    private func engineApiKey() -> String {
        if let data = UserDefaults.standard.data(forKey: "hookbot_config"),
           let config = try? JSONDecoder().decode(RuntimeConfig.self, from: data) {
            return config.apiKey
        }
        return ""
    }

    private func applyAuth(_ request: inout URLRequest) {
        let key = apiKey.isEmpty ? engineApiKey() : apiKey
        if !key.isEmpty {
            request.setValue(key, forHTTPHeaderField: "X-API-Key")
        }
    }

    private func execute<T: Decodable>(_ request: URLRequest) async throws -> T {
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
            throw APIError.requestFailed
        }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode(T.self, from: data)
    }
}

enum APIError: Error, LocalizedError {
    case noServer
    case invalidURL
    case requestFailed

    var errorDescription: String? {
        switch self {
        case .noServer: return "No server configured"
        case .invalidURL: return "Invalid URL"
        case .requestFailed: return "Request failed"
        }
    }
}
