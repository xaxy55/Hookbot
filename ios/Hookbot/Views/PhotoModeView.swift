import SwiftUI
import UIKit

// Photo mode — place avatar in the real world, take photos, share with stats (Phase 13.3)

struct PhotoModeView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.dismiss) var dismiss
    @State private var capturedImage: UIImage?
    @State private var showShareSheet = false
    @State private var avatarScale: CGFloat = 1.0
    @State private var avatarOffset: CGSize = .zero
    @State private var showStats = true
    @State private var isCapturing = false
    @GestureState private var dragState: CGSize = .zero
    @GestureState private var scaleState: CGFloat = 1.0

    var body: some View {
        ZStack {
            // Camera placeholder (ARKit would go here on a real device)
            cameraBackground

            // Draggable + scalable avatar overlay
            avatarOverlay
                .scaleEffect(avatarScale * scaleState)
                .offset(x: avatarOffset.width + dragState.width,
                        y: avatarOffset.height + dragState.height)
                .gesture(dragGesture)
                .gesture(pinchGesture)

            // Stats overlay
            if showStats {
                statsOverlay
            }

            // Capture button + controls
            VStack {
                topControls
                Spacer()
                bottomControls
            }
        }
        .ignoresSafeArea()
        .preferredColorScheme(.dark)
        .sheet(isPresented: $showShareSheet) {
            if let image = capturedImage {
                ShareSheet(items: [image])
            }
        }
    }

    // MARK: - Camera background

    private var cameraBackground: some View {
        ZStack {
            Color(white: 0.05)
                .ignoresSafeArea()

            // AR indicator
            VStack {
                Spacer()
                HStack {
                    Image(systemName: "arkit")
                        .font(.system(size: 20))
                        .foregroundStyle(.cyan.opacity(0.5))
                    Text("AR MODE")
                        .font(.system(size: 11, weight: .bold, design: .monospaced))
                        .foregroundStyle(.cyan.opacity(0.5))
                }
                .padding(.bottom, 120)
            }

            // Grid overlay for AR feel
            Canvas { ctx, size in
                let spacing: CGFloat = 60
                var linePath = Path()
                var x: CGFloat = 0
                while x < size.width {
                    linePath.move(to: CGPoint(x: x, y: 0))
                    linePath.addLine(to: CGPoint(x: x, y: size.height))
                    x += spacing
                }
                var y: CGFloat = 0
                while y < size.height {
                    linePath.move(to: CGPoint(x: 0, y: y))
                    linePath.addLine(to: CGPoint(x: size.width, y: y))
                    y += spacing
                }
                ctx.stroke(linePath, with: .color(.cyan.opacity(0.07)), lineWidth: 0.5)
            }
            .ignoresSafeArea()
        }
    }

    // MARK: - Avatar overlay

    private var avatarOverlay: some View {
        ZStack {
            // Shadow
            Ellipse()
                .fill(Color.black.opacity(0.4))
                .frame(width: 80 * avatarScale, height: 20)
                .offset(y: 55 * avatarScale)
                .blur(radius: 4)

            // Avatar canvas
            AvatarView()
                .environmentObject(engine)
                .frame(width: 100, height: 100)
                .clipShape(RoundedRectangle(cornerRadius: 12))
        }
    }

    // MARK: - Stats overlay

    private var statsOverlay: some View {
        VStack {
            HStack {
                Spacer()
                VStack(alignment: .trailing, spacing: 4) {
                    statBadge(icon: "flame.fill", value: "\(UserDefaults.standard.integer(forKey: "hookbot_streak"))d", color: .orange)
                    statBadge(icon: "star.fill", value: "\(UserDefaults.standard.integer(forKey: "hookbot_xp")) XP", color: .yellow)
                    statBadge(icon: "cpu", value: engine.currentState.displayName, color: .cyan)
                }
                .padding(12)
            }
            Spacer()
        }
        .padding(.top, 100)
    }

    private func statBadge(icon: String, value: String, color: Color) -> some View {
        HStack(spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 10, weight: .bold))
            Text(value)
                .font(.system(size: 11, weight: .bold, design: .monospaced))
        }
        .foregroundStyle(color)
        .padding(.horizontal, 10)
        .padding(.vertical, 5)
        .background(.ultraThinMaterial)
        .clipShape(Capsule())
    }

    // MARK: - Controls

    private var topControls: some View {
        HStack {
            Button {
                dismiss()
            } label: {
                Image(systemName: "xmark.circle.fill")
                    .font(.system(size: 28))
                    .foregroundStyle(.white.opacity(0.8))
                    .background(Circle().fill(Color.black.opacity(0.4)))
            }
            .padding()

            Spacer()

            Button {
                withAnimation { showStats.toggle() }
            } label: {
                Image(systemName: showStats ? "chart.bar.fill" : "chart.bar")
                    .font(.system(size: 22))
                    .foregroundStyle(.white.opacity(0.8))
                    .padding(10)
                    .background(Circle().fill(Color.black.opacity(0.4)))
            }
            .padding()
        }
        .padding(.top, 50)
    }

    private var bottomControls: some View {
        HStack(spacing: 30) {
            // Reset position
            Button {
                withAnimation(.spring()) {
                    avatarOffset = .zero
                    avatarScale = 1.0
                }
            } label: {
                Image(systemName: "arrow.counterclockwise")
                    .font(.system(size: 20))
                    .foregroundStyle(.white)
                    .frame(width: 50, height: 50)
                    .background(Circle().fill(Color(white: 0.2)))
            }

            // Capture button
            Button {
                capturePhoto()
            } label: {
                ZStack {
                    Circle()
                        .fill(.white)
                        .frame(width: 72, height: 72)
                    Circle()
                        .stroke(Color.white.opacity(0.6), lineWidth: 3)
                        .frame(width: 82, height: 82)
                    if isCapturing {
                        ProgressView()
                            .tint(.black)
                    }
                }
            }
            .disabled(isCapturing)

            // Share last capture
            Button {
                showShareSheet = true
            } label: {
                Image(systemName: "square.and.arrow.up")
                    .font(.system(size: 20))
                    .foregroundStyle(capturedImage == nil ? .gray : .white)
                    .frame(width: 50, height: 50)
                    .background(Circle().fill(Color(white: 0.2)))
            }
            .disabled(capturedImage == nil)
        }
        .padding(.bottom, 50)
    }

    // MARK: - Gestures

    private var dragGesture: some Gesture {
        DragGesture()
            .updating($dragState) { value, state, _ in
                state = value.translation
            }
            .onEnded { value in
                avatarOffset.width += value.translation.width
                avatarOffset.height += value.translation.height
            }
    }

    private var pinchGesture: some Gesture {
        MagnificationGesture()
            .updating($scaleState) { value, state, _ in
                state = value
            }
            .onEnded { value in
                avatarScale = max(0.3, min(3.0, avatarScale * value))
            }
    }

    // MARK: - Capture

    private func capturePhoto() {
        isCapturing = true
        HapticManager.playEvent(.taskComplete)

        // Render current view to image
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            let renderer = UIGraphicsImageRenderer(size: UIScreen.main.bounds.size)
            let image = renderer.image { _ in
                UIApplication.shared.connectedScenes
                    .compactMap { ($0 as? UIWindowScene)?.windows.first }
                    .first?
                    .drawHierarchy(in: UIScreen.main.bounds, afterScreenUpdates: true)
            }
            capturedImage = image
            isCapturing = false
            showShareSheet = true
        }
    }
}

// MARK: - Share Sheet

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]

    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }

    func updateUIViewController(_ uiViewController: UIActivityViewController, context: Context) {}
}
