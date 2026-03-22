import SwiftUI

struct DeviceListView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var devices: [DeviceWithStatus] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 12) {
                if isLoading && devices.isEmpty {
                    LoadingStateView(message: "Loading devices...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchDevices() }
                } else if devices.isEmpty {
                    EmptyStateView(icon: "cpu", message: "No devices found")
                } else {
                    ForEach(devices) { device in
                        NavigationLink(destination: DeviceDetailView(device: device).environmentObject(engine)) {
                            deviceRow(device)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("Devices")
        .onAppear { fetchDevices() }
        .refreshable { fetchDevices() }
    }

    // MARK: - Device Row

    private func deviceRow(_ device: DeviceWithStatus) -> some View {
        HStack(spacing: 12) {
            Circle()
                .fill(device.latestStatus?.state == "idle" || device.latestStatus != nil ? Color.green : Color.gray)
                .frame(width: 10, height: 10)

            VStack(alignment: .leading, spacing: 4) {
                Text(device.name)
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                HStack(spacing: 8) {
                    if let fw = device.firmwareVersion {
                        Text("v\(fw)")
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundColor(.gray)
                    }
                    if let mode = device.connectionMode {
                        Text(mode.uppercased())
                            .font(.system(size: 10, weight: .bold, design: .monospaced))
                            .foregroundColor(.cyan)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Capsule().fill(Color.cyan.opacity(0.15)))
                    }
                }
            }

            Spacer()

            if let state = device.latestStatus?.state {
                Text(state)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundColor(.cyan)
            }

            Image(systemName: "chevron.right")
                .font(.system(size: 12))
                .foregroundColor(Color(white: 0.3))
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - API

    private func fetchDevices() {
        guard !engine.config.serverURL.isEmpty else { return }
        isLoading = true
        errorMessage = nil

        guard let url = URL(string: "\(engine.config.serverURL)/api/devices") else {
            isLoading = false
            return
        }

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                isLoading = false
                if let error {
                    errorMessage = error.localizedDescription
                    return
                }
                guard let data,
                      let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                    errorMessage = "Failed to load devices"
                    return
                }
                if let decoded = try? JSONDecoder().decode([DeviceWithStatus].self, from: data) {
                    devices = decoded
                }
            }
        }.resume()
    }
}
