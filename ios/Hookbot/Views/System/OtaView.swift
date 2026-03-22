import SwiftUI

struct OtaView: View {
    @EnvironmentObject var engine: AvatarEngine
    @State private var firmwareVersions: [FirmwareVersion] = []
    @State private var otaJobs: [OTAJob] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if isLoading && firmwareVersions.isEmpty {
                    LoadingStateView(message: "Loading firmware info...")
                } else if let errorMessage {
                    ErrorStateView(message: errorMessage) { fetchAll() }
                } else {
                    if !firmwareVersions.isEmpty {
                        firmwareSection
                    }
                    if !otaJobs.isEmpty {
                        jobsSection
                    }
                    if firmwareVersions.isEmpty && otaJobs.isEmpty {
                        EmptyStateView(icon: "arrow.down.circle", message: "No firmware data available")
                    }
                }
            }
            .padding()
        }
        .background(Color.black)
        .navigationTitle("OTA Updates")
        .onAppear { fetchAll() }
        .refreshable { fetchAll() }
    }

    // MARK: - Firmware Section

    private var firmwareSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("FIRMWARE VERSIONS")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            ForEach(firmwareVersions) { fw in
                HStack(spacing: 12) {
                    Image(systemName: "cpu")
                        .font(.system(size: 16))
                        .foregroundColor(.cyan)
                        .frame(width: 24)

                    VStack(alignment: .leading, spacing: 3) {
                        Text("v\(fw.version)")
                            .font(.system(size: 14, weight: .bold, design: .monospaced))
                            .foregroundColor(.white)

                        if let notes = fw.releaseNotes, !notes.isEmpty {
                            Text(notes)
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundColor(.gray)
                                .lineLimit(2)
                        }
                    }

                    Spacer()

                    if fw.isLatest {
                        Text("LATEST")
                            .font(.system(size: 10, weight: .bold, design: .monospaced))
                            .foregroundColor(.green)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Capsule().fill(Color.green.opacity(0.15)))
                    }

                    if let date = fw.createdAt {
                        Text(String(date.prefix(10)))
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundColor(.gray)
                    }
                }
                .padding(12)
                .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.06)))
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    // MARK: - Jobs Section

    private var jobsSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("DEPLOYMENT JOBS")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundColor(.gray)

            ForEach(otaJobs) { job in
                HStack(spacing: 12) {
                    Image(systemName: jobIcon(job.status))
                        .font(.system(size: 16))
                        .foregroundColor(jobColor(job.status))
                        .frame(width: 24)

                    VStack(alignment: .leading, spacing: 3) {
                        Text(job.deviceName)
                            .font(.system(size: 13, weight: .semibold, design: .monospaced))
                            .foregroundColor(.white)

                        Text("v\(job.targetVersion)")
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundColor(.cyan)
                    }

                    Spacer()

                    Text(job.status.uppercased())
                        .font(.system(size: 10, weight: .bold, design: .monospaced))
                        .foregroundColor(jobColor(job.status))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Capsule().fill(jobColor(job.status).opacity(0.15)))
                }
                .padding(12)
                .background(RoundedRectangle(cornerRadius: 10).fill(Color(white: 0.06)))
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }

    private func jobIcon(_ status: String) -> String {
        switch status.lowercased() {
        case "pending": return "clock"
        case "downloading", "in_progress": return "arrow.down.circle"
        case "completed", "success": return "checkmark.circle.fill"
        case "failed": return "xmark.circle.fill"
        default: return "circle"
        }
    }

    private func jobColor(_ status: String) -> Color {
        switch status.lowercased() {
        case "pending": return .orange
        case "downloading", "in_progress": return .cyan
        case "completed", "success": return .green
        case "failed": return .red
        default: return .gray
        }
    }

    // MARK: - API

    private func fetchAll() {
        fetchFirmware()
        fetchJobs()
    }

    private func fetchFirmware() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/firmware") else { return }
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
                    errorMessage = "Failed to load firmware"
                    return
                }
                firmwareVersions = (try? JSONDecoder().decode([FirmwareVersion].self, from: data)) ?? []
            }
        }.resume()
    }

    private func fetchJobs() {
        guard !engine.config.serverURL.isEmpty,
              let url = URL(string: "\(engine.config.serverURL)/api/ota/jobs") else { return }

        var request = URLRequest(url: url)
        if !engine.config.apiKey.isEmpty {
            request.setValue(engine.config.apiKey, forHTTPHeaderField: "X-API-Key")
        }

        URLSession.shared.dataTask(with: request) { data, _, _ in
            DispatchQueue.main.async {
                guard let data else { return }
                otaJobs = (try? JSONDecoder().decode([OTAJob].self, from: data)) ?? []
            }
        }.resume()
    }
}

// MARK: - Models

struct FirmwareVersion: Codable, Identifiable {
    var id: String { version }
    let version: String
    let releaseNotes: String?
    let isLatest: Bool
    let createdAt: String?

    enum CodingKeys: String, CodingKey {
        case version
        case releaseNotes = "release_notes"
        case isLatest = "is_latest"
        case createdAt = "created_at"
    }
}

struct OTAJob: Codable, Identifiable {
    var id: String { "\(deviceName)-\(targetVersion)" }
    let deviceName: String
    let targetVersion: String
    let status: String

    enum CodingKeys: String, CodingKey {
        case deviceName = "device_name"
        case targetVersion = "target_version"
        case status
    }
}
