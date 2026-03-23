import SwiftUI

struct WatchAvatarView: View {
    @EnvironmentObject var engine: AvatarEngine

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 30.0)) { _ in
            Canvas { context, size in
                AvatarRenderer.draw(
                    context: &context,
                    size: size,
                    params: engine.current,
                    state: engine.currentState,
                    stateTime: Float(Date().timeIntervalSince(engine.stateEnteredAt) * 1000),
                    totalTime: Float(ProcessInfo.processInfo.systemUptime * 1000),
                    accessories: engine.config.accessories,
                    tool: engine.currentTool,
                    tasks: engine.tasks,
                    activeTaskIndex: engine.activeTaskIndex
                )
            }
            .background(Color.black)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .aspectRatio(2, contentMode: .fit)
        }
    }
}
