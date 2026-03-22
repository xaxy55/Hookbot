import SwiftUI

struct DiscoveryView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var discoveredDevices: [DiscoveredDevice] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                if isLoading && discoveredDevices.isEmpty {
                    LoadingStateView(message: "Scanning network...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchDiscovery() }
                } else if discoveredDevices.isEmpty {
                    EmptyStateView(icon: "wifi.exclamationmark", message: "No devices discovered\non the network")
                } else {
                    Text("\(discoveredDevices.count) device(s) found")
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundColor(.gray)
                        .frame(maxWidth: .infinity, alignment: .leading)

                    ForEach(discoveredDevices) { device in
                        discoveredRow(device)
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Discovery")
        .onAppear { fetchDiscovery() }
        .refreshable { fetchDiscovery() }
    }

    // MARK: - Row

    private func discoveredRow(_ device: DiscoveredDevice) -> some View {
        HStack(spacing: 12) {
            Image(systemName: "wifi")
                .font(.system(size: 16))
                .foregroundColor(.cyan)
                .frame(width: 32, height: 32)
                .background(Circle().fill(Color.cyan.opacity(0.15)))

            VStack(alignment: .leading, spacing: 3) {
                Text(device.hostname ?? device.ipAddress)
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                HStack(spacing: 8) {
                    Text(device.ipAddress)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundColor(.gray)

                    if let mac = device.macAddress {
                        Text(mac)
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundColor(Color(white: 0.4))
                    }
                }
            }

            Spacer()

            if device.isClaimed {
                Text("CLAIMED")
                    .font(.system(size: 10, weight: .bold, design: .monospaced))
                    .foregroundColor(.green)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Capsule().fill(Color.green.opacity(0.15)))
            } else {
                Text("NEW")
                    .font(.system(size: 10, weight: .bold, design: .monospaced))
                    .foregroundColor(.orange)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Capsule().fill(Color.orange.opacity(0.15)))
            }
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - API

    private func fetchDiscovery() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/discovery") else { return }
        isLoading = true
        errorMessage = nil

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                isLoading = false
                if let error { errorMessage = error.localizedDescription; return }
                guard let data,
                      let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    errorMessage = "Failed to scan network"
                    return
                }
                discoveredDevices = (try? JSONDecoder().decode([DiscoveredDevice].self, from: data)) ?? []
            }
        }.resume()
    }
}

// MARK: - Model

struct DiscoveredDevice: Codable, Identifiable {
    var id: String { ipAddress }
    let ipAddress: String
    let hostname: String?
    let macAddress: String?
    let isClaimed: Bool

    enum CodingKeys: String, CodingKey {
        case ipAddress = "ip_address"
        case hostname
        case macAddress = "mac_address"
        case isClaimed = "is_claimed"
    }
}
