import SwiftUI

struct MainView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.verticalSizeClass) var verticalSizeClass
    @State private var showSettings = false

    var isLandscape: Bool { verticalSizeClass == .compact }

    var body: some View {
        NavigationStack {
            ZStack {
                glowBackground

                VStack(spacing: 0) {
                    if !isLandscape {
                        // Status bar (portrait only)
                        HStack {
                            Circle()
                                .fill(Color.green)
                                .frame(width: 8, height: 8)
                            Text(engine.currentState.displayName.uppercased())
                                .font(.system(size: 12, weight: .bold, design: .monospaced))
                                .foregroundColor(.gray)
                            Spacer()
                            Text("DESTROYER OF WORLDS")
                                .font(.system(size: 10, weight: .medium, design: .monospaced))
                                .foregroundColor(Color(white: 0.3))
                        }
                        .padding(.horizontal)
                        .padding(.top, 8)
                    }

                    // Avatar - fills all available space
                    AvatarView()
                        .environmentObject(engine)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .padding(isLandscape ? 4 : 16)

                    if !isLandscape {
                        // Control panel (portrait only)
                        ControlPanelView()
                            .environmentObject(engine)
                            .padding(.bottom, 8)
                    }
                }
            }
            .background(Color.black)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar(isLandscape ? .hidden : .visible, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        showSettings = true
                    } label: {
                        Image(systemName: "gearshape")
                            .foregroundColor(.gray)
                    }
                }
            }
            .sheet(isPresented: $showSettings) {
                SettingsView()
                    .environmentObject(engine)
            }
        }
        .preferredColorScheme(.dark)
    }

    private var glowBackground: some View {
        let led = engine.ledColor
        let glowColor = Color(
            red: Double(led.r),
            green: Double(led.g),
            blue: Double(led.b)
        )
        return RadialGradient(
            gradient: Gradient(colors: [glowColor.opacity(0.4), .clear]),
            center: .center,
            startRadius: 50,
            endRadius: 300
        )
        .ignoresSafeArea()
    }
}
