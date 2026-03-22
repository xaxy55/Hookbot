import SwiftUI

struct ControlPanelView: View {
    @EnvironmentObject var engine: AvatarEngine

    private let states: [(AvatarState, Color)] = [
        (.idle,      Color(red: 0.55, green: 0, blue: 0)),
        (.thinking,  Color(red: 0.29, green: 0, blue: 0.5)),
        (.waiting,   Color(red: 0.72, green: 0.53, blue: 0.04)),
        (.success,   Color(red: 0, green: 0.39, blue: 0)),
        (.taskcheck, Color(red: 0, green: 0.33, blue: 0.33)),
        (.error,     Color.red),
    ]

    var body: some View {
        LazyVGrid(columns: [
            GridItem(.flexible()),
            GridItem(.flexible()),
            GridItem(.flexible()),
        ], spacing: 10) {
            ForEach(states, id: \.0) { state, color in
                Button {
                    engine.setState(state)
                } label: {
                    Text(state.displayName)
                        .font(.system(size: 14, weight: .semibold, design: .default))
                        .foregroundColor(.white)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 14)
                        .background(
                            RoundedRectangle(cornerRadius: 12)
                                .fill(color)
                                .overlay(
                                    RoundedRectangle(cornerRadius: 12)
                                        .fill(engine.currentState == state ? Color.white.opacity(0.2) : .clear)
                                )
                        )
                }
                .buttonStyle(.plain)
<<<<<<< Updated upstream
                .accessibilityIdentifier("state_\(state.rawValue)")
=======
>>>>>>> Stashed changes
            }
        }
        .padding(.horizontal)
    }
}
