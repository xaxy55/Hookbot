import SwiftUI

struct WatchSettingsView: View {
    @EnvironmentObject var network: WatchNetworkService
    @EnvironmentObject var engine: AvatarEngine
    @State private var serverInput = ""
    @State private var deviceInput = ""
    @State private var hapticsEnabled = true

    var body: some View {
        ScrollView {
            VStack(spacing: 8) {
                Text("SETTINGS")
                    .font(.system(size: 10, weight: .black, design: .monospaced))
                    .foregroundColor(.gray)
                    .frame(maxWidth: .infinity, alignment: .leading)

                // Haptics toggle
                Toggle(isOn: $hapticsEnabled) {
                    VStack(alignment: .leading, spacing: 1) {
                        Text("Haptics")
                            .font(.system(size: 11, weight: .semibold, design: .monospaced))
                        Text("Vibrate on state changes")
                            .font(.system(size: 8, design: .monospaced))
                            .foregroundColor(.gray)
                    }
                }
                .onChange(of: hapticsEnabled) { _, enabled in
                    UserDefaults.standard.set(enabled, forKey: "hookbot_haptics")
                    if enabled {
                        HapticManager.shared.playStateHaptic(.success)
                    }
                }

                Divider().background(Color.gray.opacity(0.3))

                // Server URL
                VStack(alignment: .leading, spacing: 2) {
                    Text("SERVER URL")
                        .font(.system(size: 8, weight: .bold, design: .monospaced))
                        .foregroundColor(.gray)

                    if network.serverURL.isEmpty {
                        Text("Not configured")
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundColor(.red.opacity(0.7))
                        Text("Set via iPhone app or tap below")
                            .font(.system(size: 8, design: .monospaced))
                            .foregroundColor(.gray)
                    } else {
                        Text(network.serverURL)
                            .font(.system(size: 9, design: .monospaced))
                            .foregroundColor(.green)
                            .lineLimit(2)
                    }

                    TextField("http://...", text: $serverInput)
                        .font(.system(size: 10, design: .monospaced))
                        .textInputAutocapitalization(.never)
                        .onSubmit {
                            if !serverInput.isEmpty {
                                network.serverURL = serverInput
                            }
                        }
                }

                Divider().background(Color.gray.opacity(0.3))

                // Device ID
                VStack(alignment: .leading, spacing: 2) {
                    Text("DEVICE ID")
                        .font(.system(size: 8, weight: .bold, design: .monospaced))
                        .foregroundColor(.gray)

                    if network.deviceId.isEmpty {
                        Text("Not set")
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundColor(.red.opacity(0.7))
                    } else {
                        Text(network.deviceId)
                            .font(.system(size: 9, design: .monospaced))
                            .foregroundColor(.cyan)
                            .lineLimit(1)
                    }

                    TextField("device-id", text: $deviceInput)
                        .font(.system(size: 10, design: .monospaced))
                        .textInputAutocapitalization(.never)
                        .onSubmit {
                            if !deviceInput.isEmpty {
                                network.deviceId = deviceInput
                            }
                        }
                }

                Divider().background(Color.gray.opacity(0.3))

                // Connection status
                VStack(alignment: .leading, spacing: 2) {
                    Text("STATUS")
                        .font(.system(size: 8, weight: .bold, design: .monospaced))
                        .foregroundColor(.gray)

                    HStack(spacing: 4) {
                        Circle()
                            .fill(network.isConnected ? Color.green : Color.red)
                            .frame(width: 6, height: 6)
                        Text(network.isConnected ? "Connected" : "Disconnected")
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundColor(network.isConnected ? .green : .red)
                    }
                }

                // Test haptics button
                Button {
                    HapticManager.shared.playLevelUp()
                } label: {
                    Text("TEST HAPTICS")
                        .font(.system(size: 10, weight: .bold, design: .monospaced))
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .tint(.purple)
            }
            .padding(.horizontal, 4)
        }
        .background(Color.black)
        .onAppear {
            serverInput = network.serverURL
            deviceInput = network.deviceId
            hapticsEnabled = UserDefaults.standard.object(forKey: "hookbot_haptics") as? Bool ?? true
        }
    }
}
