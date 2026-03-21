import SwiftUI
import ARKit
import SceneKit

// Shared AR Space — places both Hookbot avatars on the same real-world surface.
// Uses ARKit plane detection + MultipeerConnectivity to sync anchor positions.
// Avatars interact: high-five, dance, wave animations. (Phase 13.5)

// MARK: - Interaction Type

enum SharedARInteraction: String, CaseIterable {
    case highFive = "highFive"
    case dance    = "dance"
    case wave     = "wave"

    var displayName: String {
        switch self {
        case .highFive: return "High Five"
        case .dance:    return "Dance"
        case .wave:     return "Wave"
        }
    }

    var icon: String {
        switch self {
        case .highFive: return "hand.raised.fill"
        case .dance:    return "music.note"
        case .wave:     return "hand.wave.fill"
        }
    }
}

// MARK: - View Model

@MainActor
final class SharedARViewModel: NSObject, ObservableObject {

    @Published var myScreenPosition: CGPoint = CGPoint(x: 150, y: 500)
    @Published var friendScreenPosition: CGPoint = CGPoint(x: 250, y: 500)
    @Published var hasPlacedAvatar = false
    @Published var friendIsPresent = false
    @Published var currentInteraction: SharedARInteraction? = nil
    @Published var interactionProgress: Float = 0
    @Published var statusMessage = "Point at a flat surface to place your avatar"
    @Published var showPlaneIndicator = false

    // Position offsets for interaction animations
    @Published var myAvatarOffset: CGSize = .zero
    @Published var friendAvatarOffset: CGSize = .zero
    @Published var myAvatarScale: CGFloat = 1.0
    @Published var friendAvatarScale: CGFloat = 1.0

    weak var arView: ARSCNView?
    private var myAnchorNode: SCNNode?
    private var friendAnchorNode: SCNNode?
    private var interactionTimer: Timer?
    private var interactionPhase: Float = 0
    private var multipeer = MultipeerService.shared
    var avatarEngine: AvatarEngine?

    override init() {
        super.init()
        setupMultipeerHandlers()
    }

    // MARK: - AR Setup

    func setupAR(_ view: ARSCNView) {
        arView = view
        view.delegate = self
        view.session.delegate = self

        let config = ARWorldTrackingConfiguration()
        config.planeDetection = [.horizontal]
        config.environmentTexturing = .automatic
        view.session.run(config, options: [.resetTracking, .removeExistingAnchors])
        view.automaticallyUpdatesLighting = true
    }

    func placeAvatar(at position: SCNVector3) {
        guard let view = arView, !hasPlacedAvatar else { return }

        // Create invisible marker node at placement position
        let node = SCNNode()
        node.position = position
        view.scene.rootNode.addChildNode(node)
        myAnchorNode = node
        hasPlacedAvatar = true
        statusMessage = "Avatar placed! Waiting for friend…"

        // Share transform with peer
        let matrix = node.worldTransform
        let flat: [Float] = [
            matrix.m11, matrix.m12, matrix.m13, matrix.m14,
            matrix.m21, matrix.m22, matrix.m23, matrix.m24,
            matrix.m31, matrix.m32, matrix.m33, matrix.m34,
            matrix.m41, matrix.m42, matrix.m43, matrix.m44
        ]
        multipeer.send(.arAnchorMatrix(flat))
        updateScreenPositions()
    }

    func receiveARMatrix(_ matrix: [Float]) {
        guard matrix.count == 16, let view = arView else { return }
        let transform = SCNMatrix4(
            m11: matrix[0], m12: matrix[1], m13: matrix[2], m14: matrix[3],
            m21: matrix[4], m22: matrix[5], m23: matrix[6], m24: matrix[7],
            m31: matrix[8], m32: matrix[9], m33: matrix[10], m34: matrix[11],
            m41: matrix[12], m42: matrix[13], m43: matrix[14], m44: matrix[15]
        )
        if let existing = friendAnchorNode {
            existing.transform = transform
        } else {
            let node = SCNNode()
            node.transform = transform
            // Offset friend's avatar 30cm to the right on the same surface
            node.position = SCNVector3(
                transform.m41 + 0.30,
                transform.m42,
                transform.m43
            )
            view.scene.rootNode.addChildNode(node)
            friendAnchorNode = node
        }
        friendIsPresent = true
        statusMessage = "\(multipeer.peerName) is here!"
        updateScreenPositions()
    }

    // MARK: - Screen Position Update

    func updateScreenPositions() {
        guard let view = arView else { return }

        if let myNode = myAnchorNode {
            let projected = view.projectPoint(myNode.worldPosition)
            myScreenPosition = CGPoint(x: CGFloat(projected.x), y: CGFloat(projected.y))
        }

        if let friendNode = friendAnchorNode {
            let projected = view.projectPoint(friendNode.worldPosition)
            friendScreenPosition = CGPoint(x: CGFloat(projected.x), y: CGFloat(projected.y))
        }
    }

    // MARK: - Interactions

    func sendInteraction(_ type: SharedARInteraction) {
        multipeer.send(.interaction(type: type.rawValue))
        playInteraction(type)
    }

    func playInteraction(_ type: SharedARInteraction) {
        interactionTimer?.invalidate()
        currentInteraction = type
        interactionPhase = 0
        interactionProgress = 0

        switch type {
        case .highFive:
            avatarEngine?.setState(.success)
            runHighFiveAnimation()
        case .dance:
            avatarEngine?.setState(.success)
            runDanceAnimation()
        case .wave:
            avatarEngine?.setState(.taskcheck)
            runWaveAnimation()
        }
    }

    private func runHighFiveAnimation() {
        var frame = 0
        interactionTimer = Timer.scheduledTimer(withTimeInterval: 1.0 / 30.0, repeats: true) { [weak self] t in
            frame += 1
            let progress = Float(frame) / 60.0
            let ease = sinf(progress * .pi)
            let moveAmount = CGFloat(ease * 30)
            let scale = 1.0 + CGFloat(ease * 0.3)
            let isDone = frame >= 60

            Task { @MainActor in
                guard let self else { t.invalidate(); return }
                self.interactionProgress = min(1.0, progress)
                self.myAvatarOffset = CGSize(width: moveAmount, height: -CGFloat(ease * 10))
                self.friendAvatarOffset = CGSize(width: -moveAmount, height: -CGFloat(ease * 10))
                self.myAvatarScale = scale
                self.friendAvatarScale = scale

                if isDone {
                    t.invalidate()
                    withAnimation(.spring()) {
                        self.myAvatarOffset = .zero
                        self.friendAvatarOffset = .zero
                        self.myAvatarScale = 1.0
                        self.friendAvatarScale = 1.0
                        self.currentInteraction = nil
                    }
                }
            }
        }
    }

    private func runDanceAnimation() {
        var frame = 0
        interactionTimer = Timer.scheduledTimer(withTimeInterval: 1.0 / 30.0, repeats: true) { [weak self] t in
            frame += 1
            let progress = Float(frame) / 120.0
            let bounce = sinf(progress * .pi * 8) * 15
            let sway = cosf(progress * .pi * 4) * 8
            let scale = 1.0 + CGFloat(abs(bounce) / 150)
            let isDone = frame >= 120

            Task { @MainActor in
                guard let self else { t.invalidate(); return }
                self.myAvatarOffset = CGSize(width: CGFloat(sway), height: CGFloat(-abs(bounce)))
                self.friendAvatarOffset = CGSize(width: CGFloat(-sway), height: CGFloat(-abs(bounce)))
                self.myAvatarScale = scale
                self.friendAvatarScale = scale

                if isDone {
                    t.invalidate()
                    withAnimation(.spring()) {
                        self.myAvatarOffset = .zero
                        self.friendAvatarOffset = .zero
                        self.myAvatarScale = 1.0
                        self.friendAvatarScale = 1.0
                        self.currentInteraction = nil
                    }
                }
            }
        }
    }

    private func runWaveAnimation() {
        var frame = 0
        interactionTimer = Timer.scheduledTimer(withTimeInterval: 1.0 / 30.0, repeats: true) { [weak self] t in
            frame += 1
            let progress = Float(frame) / 45.0
            let wave = sinf(progress * .pi * 6) * 10
            let isDone = frame >= 45

            Task { @MainActor in
                guard let self else { t.invalidate(); return }
                self.myAvatarOffset = CGSize(width: CGFloat(wave), height: 0)

                if isDone {
                    t.invalidate()
                    withAnimation(.spring()) {
                        self.myAvatarOffset = .zero
                        self.currentInteraction = nil
                    }
                }
            }
        }
    }

    // MARK: - Multipeer Handlers

    private func setupMultipeerHandlers() {
        MultipeerService.shared.onARMatrix = { [weak self] matrix in
            Task { @MainActor in
                self?.receiveARMatrix(matrix)
            }
        }
        MultipeerService.shared.onInteraction = { [weak self] typeStr in
            Task { @MainActor in
                if let type = SharedARInteraction(rawValue: typeStr) {
                    self?.playInteraction(type)
                }
            }
        }
    }
}

// MARK: - ARSCNViewDelegate

extension SharedARViewModel: ARSCNViewDelegate {

    nonisolated func renderer(_ renderer: SCNSceneRenderer, updateAtTime time: TimeInterval) {
        Task { @MainActor in
            updateScreenPositions()
        }
    }

    nonisolated func renderer(_ renderer: SCNSceneRenderer, didAdd node: SCNNode, for anchor: ARAnchor) {
        guard anchor is ARPlaneAnchor else { return }
        Task { @MainActor in
            showPlaneIndicator = true
            if !hasPlacedAvatar {
                statusMessage = "Tap to place your avatar here"
            }
        }
    }
}

// MARK: - ARSessionDelegate

extension SharedARViewModel: ARSessionDelegate {

    nonisolated func session(_ session: ARSession, didFailWithError error: Error) {
        Task { @MainActor in
            statusMessage = "AR session failed. Please restart."
        }
    }

    nonisolated func sessionWasInterrupted(_ session: ARSession) {}
    nonisolated func sessionInterruptionEnded(_ session: ARSession) {}
}

// MARK: - ARSCNView Representable

struct ARSceneViewRepresentable: UIViewRepresentable {

    @ObservedObject var viewModel: SharedARViewModel
    var onTap: (SCNVector3) -> Void

    func makeUIView(context: Context) -> ARSCNView {
        let view = ARSCNView()
        view.showsStatistics = false
        view.autoenablesDefaultLighting = true

        // Tap to place
        let tap = UITapGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handleTap(_:)))
        view.addGestureRecognizer(tap)

        viewModel.setupAR(view)
        return view
    }

    func updateUIView(_ uiView: ARSCNView, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(onTap: onTap)
    }

    class Coordinator: NSObject {
        var onTap: (SCNVector3) -> Void

        init(onTap: @escaping (SCNVector3) -> Void) {
            self.onTap = onTap
        }

        @objc func handleTap(_ gesture: UITapGestureRecognizer) {
            guard let arView = gesture.view as? ARSCNView else { return }
            let location = gesture.location(in: arView)
            guard let query = arView.raycastQuery(from: location, allowing: .estimatedPlane, alignment: .horizontal) else { return }
            let results = arView.session.raycast(query)
            if let hit = results.first {
                let pos = SCNVector3(
                    hit.worldTransform.columns.3.x,
                    hit.worldTransform.columns.3.y,
                    hit.worldTransform.columns.3.z
                )
                onTap(pos)
            }
        }
    }
}

// MARK: - Main View

struct SharedARView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.dismiss) var dismiss
    @StateObject private var viewModel = SharedARViewModel()
    @StateObject private var multipeer = MultipeerService.shared

    var body: some View {
        ZStack {
            // AR background
            ARSceneViewRepresentable(viewModel: viewModel) { position in
                viewModel.placeAvatar(at: position)
            }
            .ignoresSafeArea()

            // My avatar overlay
            if viewModel.hasPlacedAvatar {
                avatarBubble(engine: engine, label: "You", tint: .cyan)
                    .position(viewModel.myScreenPosition)
                    .offset(viewModel.myAvatarOffset)
                    .scaleEffect(viewModel.myAvatarScale)
                    .animation(.spring(response: 0.3), value: viewModel.myAvatarOffset)
            }

            // Friend avatar overlay
            if viewModel.friendIsPresent {
                friendAvatarBubble(label: multipeer.peerName, tint: .purple)
                    .position(viewModel.friendScreenPosition)
                    .offset(viewModel.friendAvatarOffset)
                    .scaleEffect(viewModel.friendAvatarScale)
                    .animation(.spring(response: 0.3), value: viewModel.friendAvatarOffset)
            }

            // Interaction spark effect
            if let interaction = viewModel.currentInteraction, viewModel.friendIsPresent {
                interactionEffect(interaction)
            }

            // UI overlay
            VStack {
                topBar
                Spacer()
                if viewModel.hasPlacedAvatar && viewModel.friendIsPresent {
                    interactionButtons
                        .padding(.bottom, 40)
                } else {
                    statusBanner
                        .padding(.bottom, 40)
                }
            }
        }
        .preferredColorScheme(.dark)
        .onAppear {
            viewModel.avatarEngine = engine
            MultipeerService.shared.startDiscovery()
            ProximityService.shared.configure(engine: engine)
            ProximityService.shared.start()
        }
        .onDisappear {
            ProximityService.shared.stop()
        }
    }

    // MARK: - Avatar Bubble

    private func avatarBubble(engine: AvatarEngine, label: String, tint: Color) -> some View {
        VStack(spacing: 4) {
            ZStack {
                Circle()
                    .fill(Color.black)
                    .frame(width: 80, height: 80)
                    .overlay(Circle().stroke(tint.opacity(0.8), lineWidth: 2))
                    .shadow(color: tint.opacity(0.5), radius: 10)

                AvatarView()
                    .environmentObject(engine)
                    .frame(width: 70, height: 70)
                    .clipShape(Circle())
            }
            Text(label)
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(tint)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(Capsule().fill(Color.black.opacity(0.7)))
        }
        // Shadow base for AR "grounded" feel
        .background(
            Ellipse()
                .fill(tint.opacity(0.15))
                .frame(width: 60, height: 12)
                .blur(radius: 4)
                .offset(y: 44)
        )
    }

    private func friendAvatarBubble(label: String, tint: Color) -> some View {
        VStack(spacing: 4) {
            ZStack {
                Circle()
                    .fill(Color.black)
                    .frame(width: 80, height: 80)
                    .overlay(Circle().stroke(tint.opacity(0.8), lineWidth: 2))
                    .shadow(color: tint.opacity(0.5), radius: 10)

                // Friend avatar: mirrored/flipped for variety
                Canvas { ctx, size in
                    ctx.scaleBy(x: -1, y: 1)
                    ctx.translateBy(x: -size.width, y: 0)
                    // Simple "friend" face placeholder (dot eyes + smile)
                    let cx = size.width / 2
                    let cy = size.height / 2

                    ctx.fill(Path(ellipseIn: CGRect(x: cx - 15, y: cy - 10, width: 10, height: 10)), with: .color(.white))
                    ctx.fill(Path(ellipseIn: CGRect(x: cx + 5, y: cy - 10, width: 10, height: 10)), with: .color(.white))

                    var smile = Path()
                    smile.move(to: CGPoint(x: cx - 12, y: cy + 8))
                    for i in 0...20 {
                        let t = CGFloat(i) / 20.0
                        let x = cx - 12 + t * 24
                        let y = cy + 8 + CGFloat(sinf(Float(t) * .pi)) * 6
                        if i == 0 { smile.move(to: CGPoint(x: x, y: y)) }
                        else { smile.addLine(to: CGPoint(x: x, y: y)) }
                    }
                    ctx.stroke(smile, with: .color(.white), lineWidth: 2)
                }
                .frame(width: 70, height: 70)
                .clipShape(Circle())
            }
            Text(label.isEmpty ? "Friend" : label)
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(tint)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(Capsule().fill(Color.black.opacity(0.7)))
        }
        .background(
            Ellipse()
                .fill(tint.opacity(0.15))
                .frame(width: 60, height: 12)
                .blur(radius: 4)
                .offset(y: 44)
        )
    }

    // MARK: - Interaction Effect

    private func interactionEffect(_ interaction: SharedARInteraction) -> some View {
        let midX = (viewModel.myScreenPosition.x + viewModel.friendScreenPosition.x) / 2
        let midY = (viewModel.myScreenPosition.y + viewModel.friendScreenPosition.y) / 2

        return ZStack {
            // Spark burst
            ForEach(0..<8, id: \.self) { i in
                Circle()
                    .fill(Color.yellow.opacity(0.8))
                    .frame(width: 6, height: 6)
                    .offset(
                        x: cos(Double(i) * .pi / 4) * Double(viewModel.interactionProgress) * 40,
                        y: sin(Double(i) * .pi / 4) * Double(viewModel.interactionProgress) * 40
                    )
                    .opacity(Double(1 - viewModel.interactionProgress))
            }
            Text(interaction == .highFive ? "✋" : interaction == .dance ? "💃" : "👋")
                .font(.system(size: 28))
                .opacity(Double(1 - viewModel.interactionProgress))
                .scaleEffect(1 + CGFloat(viewModel.interactionProgress) * 0.5)
        }
        .position(x: midX, y: midY)
    }

    // MARK: - Top Bar

    private var topBar: some View {
        HStack {
            Button { dismiss() } label: {
                Image(systemName: "xmark.circle.fill")
                    .font(.system(size: 28))
                    .foregroundStyle(.white.opacity(0.8))
                    .background(Circle().fill(Color.black.opacity(0.4)))
            }
            .padding()

            Spacer()

            // Peer connection indicator
            HStack(spacing: 6) {
                Circle()
                    .fill(multipeer.isConnected ? Color.green : Color.orange)
                    .frame(width: 8, height: 8)
                Text(multipeer.isConnected ? multipeer.peerName : "Searching…")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundStyle(.white)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(.ultraThinMaterial)
            .clipShape(Capsule())
            .padding()
        }
        .padding(.top, 50)
    }

    // MARK: - Interaction Buttons

    private var interactionButtons: some View {
        VStack(spacing: 12) {
            Text("TRIGGER INTERACTION")
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundStyle(.white.opacity(0.5))

            HStack(spacing: 16) {
                ForEach(SharedARInteraction.allCases, id: \.self) { interaction in
                    Button {
                        viewModel.sendInteraction(interaction)
                    } label: {
                        VStack(spacing: 4) {
                            Image(systemName: interaction.icon)
                                .font(.system(size: 22))
                            Text(interaction.displayName)
                                .font(.system(size: 9, design: .monospaced))
                        }
                        .foregroundStyle(.white)
                        .frame(width: 72, height: 60)
                        .background(
                            RoundedRectangle(cornerRadius: 12)
                                .fill(Color(white: 0.15))
                                .overlay(
                                    RoundedRectangle(cornerRadius: 12)
                                        .stroke(Color.cyan.opacity(0.4), lineWidth: 1)
                                )
                        )
                    }
                    .disabled(viewModel.currentInteraction != nil)
                }
            }
        }
        .padding(16)
        .background(.ultraThinMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .padding(.horizontal, 24)
    }

    // MARK: - Status Banner

    private var statusBanner: some View {
        HStack(spacing: 10) {
            Image(systemName: "arkit")
                .font(.system(size: 16))
                .foregroundStyle(.cyan)
            Text(viewModel.statusMessage)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.white)
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 12)
        .background(.ultraThinMaterial)
        .clipShape(Capsule())
        .padding(.horizontal, 32)
    }
}
