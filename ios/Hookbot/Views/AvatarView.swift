import SwiftUI

struct AvatarView: View {
    @EnvironmentObject var engine: AvatarEngine

    var body: some View {
        // Canvas redraws whenever engine's @Published properties change
        Canvas { context, size in
            AvatarRenderer.draw(
                context: &context,
                size: size,
                params: engine.current,
                state: engine.currentState,
                stateTime: engine.stateTime,
                totalTime: engine.totalTime,
                accessories: engine.config.accessories,
                tool: engine.currentTool,
                tasks: engine.tasks,
                activeTaskIndex: engine.activeTaskIndex
            )
        }
        .background(Color.black)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(Color(white: 0.15), lineWidth: 1)
        )
        .aspectRatio(2, contentMode: .fit)
    }
}
