import SwiftUI
import SpriteKit

// Cross-screen Pong — two paired phones side by side form one wide screen.
// Each phone controls one paddle (your Hookbot avatar).
// When the ball exits toward your friend's phone, it's handed off via MultipeerConnectivity.
// (Phase 13.5)

// MARK: - Pong Game State

enum PongRole {
    case left   // Host: ball starts here, left paddle
    case right  // Guest: right paddle
}

// MARK: - SpriteKit Scene

final class PongScene: SKScene {

    // Nodes
    private var leftPaddle: SKShapeNode!
    private var rightPaddle: SKShapeNode!
    private var ball: SKShapeNode!
    private var leftScoreLabel: SKLabelNode!
    private var rightScoreLabel: SKLabelNode!
    private var seam: SKShapeNode!        // Vertical "edge" showing where friend's screen begins
    private var divider: SKShapeNode!

    // State
    var role: PongRole = .left
    var leftScore = 0
    var rightScore = 0
    private var ballVelocity = CGVector(dx: 280, dy: 220)
    private var gameRunning = false
    private var multipeer = MultipeerService.shared

    // Callbacks to SwiftUI
    var onScoreChange: ((Int, Int) -> Void)?
    var onBallHandoff: ((CGFloat, CGFloat, CGFloat, Int, Int) -> Void)?  // velX, velY, ballY, myScore, theirScore
    var onMyPaddleMove: ((CGFloat) -> Void)?

    // Constants
    private let paddleW: CGFloat = 16
    private let paddleH: CGFloat = 80
    private let ballRadius: CGFloat = 12

    override func didMove(to view: SKView) {
        backgroundColor = .black
        physicsWorld.gravity = .zero
        physicsWorld.contactDelegate = self

        setupNodes()
        setupBoundaries()

        // Ball starts only on the left phone
        if role == .right { ball.isHidden = true }
    }

    // MARK: - Setup

    private func setupNodes() {
        let w = size.width
        let h = size.height
        let midY = h / 2

        // Seam line (where the two screens meet)
        let seamPath = CGMutablePath()
        seamPath.move(to: CGPoint(x: role == .left ? w : 0, y: 0))
        seamPath.addLine(to: CGPoint(x: role == .left ? w : 0, y: h))
        seam = SKShapeNode(path: seamPath)
        seam.strokeColor = UIColor(white: 1, alpha: 0.12)
        seam.lineWidth = 2
        addChild(seam)

        // Center divider (visual reference)
        let divPath = CGMutablePath()
        for y in stride(from: 10.0, through: h, by: 20) {
            divPath.move(to: CGPoint(x: w / 2, y: y))
            divPath.addLine(to: CGPoint(x: w / 2, y: y + 10))
        }
        divider = SKShapeNode(path: divPath)
        divider.strokeColor = UIColor(white: 1, alpha: 0.08)
        divider.lineWidth = 1.5
        addChild(divider)

        // Left paddle
        let lRect = CGRect(x: -paddleW / 2, y: -paddleH / 2, width: paddleW, height: paddleH)
        leftPaddle = SKShapeNode(rect: lRect, cornerRadius: paddleW / 2)
        leftPaddle.fillColor = UIColor(red: 0, green: 0.8, blue: 1, alpha: 1)
        leftPaddle.strokeColor = .clear
        leftPaddle.position = CGPoint(x: 30, y: midY)
        leftPaddle.physicsBody = makeStaticBody(size: CGSize(width: paddleW, height: paddleH))
        leftPaddle.name = "leftPaddle"
        addChild(leftPaddle)
        addAvatarFace(to: leftPaddle, color: .cyan)

        // Right paddle
        let rRect = CGRect(x: -paddleW / 2, y: -paddleH / 2, width: paddleW, height: paddleH)
        rightPaddle = SKShapeNode(rect: rRect, cornerRadius: paddleW / 2)
        rightPaddle.fillColor = UIColor(red: 0.6, green: 0.3, blue: 1, alpha: 1)
        rightPaddle.strokeColor = .clear
        rightPaddle.position = CGPoint(x: w - 30, y: midY)
        rightPaddle.physicsBody = makeStaticBody(size: CGSize(width: paddleW, height: paddleH))
        rightPaddle.name = "rightPaddle"
        addChild(rightPaddle)
        addAvatarFace(to: rightPaddle, color: UIColor(red: 0.6, green: 0.3, blue: 1, alpha: 1))

        // Ball
        ball = SKShapeNode(circleOfRadius: ballRadius)
        ball.fillColor = .white
        ball.strokeColor = UIColor(white: 1, alpha: 0.5)
        ball.lineWidth = 1
        ball.position = CGPoint(x: w / 2, y: midY)
        let ballBody = SKPhysicsBody(circleOfRadius: ballRadius)
        ballBody.isDynamic = true
        ballBody.friction = 0
        ballBody.restitution = 1
        ballBody.linearDamping = 0
        ballBody.angularDamping = 0
        ballBody.categoryBitMask = 1
        ballBody.contactTestBitMask = 2
        ball.physicsBody = ballBody
        addChild(ball)

        // Score labels
        leftScoreLabel = SKLabelNode(fontNamed: "Courier-Bold")
        leftScoreLabel.fontSize = 28
        leftScoreLabel.fontColor = UIColor(white: 1, alpha: 0.5)
        leftScoreLabel.position = CGPoint(x: w / 4, y: h - 50)
        leftScoreLabel.text = "0"
        addChild(leftScoreLabel)

        rightScoreLabel = SKLabelNode(fontNamed: "Courier-Bold")
        rightScoreLabel.fontSize = 28
        rightScoreLabel.fontColor = UIColor(white: 1, alpha: 0.5)
        rightScoreLabel.position = CGPoint(x: w * 3 / 4, y: h - 50)
        rightScoreLabel.text = "0"
        addChild(rightScoreLabel)
    }

    private func addAvatarFace(to paddle: SKShapeNode, color: UIColor) {
        // Draw minimal avatar eyes on the paddle
        let eyeLeft = SKShapeNode(ellipseOf: CGSize(width: 4, height: 5))
        eyeLeft.fillColor = .black
        eyeLeft.position = CGPoint(x: -2, y: 8)
        paddle.addChild(eyeLeft)

        let eyeRight = SKShapeNode(ellipseOf: CGSize(width: 4, height: 5))
        eyeRight.fillColor = .black
        eyeRight.position = CGPoint(x: 2, y: 8)
        paddle.addChild(eyeRight)

        // Smile
        let smilePath = CGMutablePath()
        smilePath.move(to: CGPoint(x: -4, y: -4))
        for i in 0...8 {
            let t = CGFloat(i) / 8.0
            smilePath.addLine(to: CGPoint(x: -4 + t * 8, y: -4 + sinf(Float(t) * .pi) * 3))
        }
        let smile = SKShapeNode(path: smilePath)
        smile.strokeColor = .black
        smile.lineWidth = 1
        paddle.addChild(smile)
    }

    private func setupBoundaries() {
        let w = size.width
        let h = size.height

        // Top wall
        let top = SKNode()
        top.position = CGPoint(x: w / 2, y: h + 1)
        let topBody = SKPhysicsBody(rectangleOf: CGSize(width: w, height: 2))
        topBody.isDynamic = false
        topBody.friction = 0
        topBody.restitution = 1
        topBody.categoryBitMask = 2
        top.physicsBody = topBody
        addChild(top)

        // Bottom wall
        let bottom = SKNode()
        bottom.position = CGPoint(x: w / 2, y: -1)
        let botBody = SKPhysicsBody(rectangleOf: CGSize(width: w, height: 2))
        botBody.isDynamic = false
        botBody.friction = 0
        botBody.restitution = 1
        botBody.categoryBitMask = 2
        bottom.physicsBody = botBody
        addChild(bottom)
    }

    private func makeStaticBody(size: CGSize) -> SKPhysicsBody {
        let body = SKPhysicsBody(rectangleOf: size)
        body.isDynamic = false
        body.friction = 0
        body.restitution = 1.05
        body.categoryBitMask = 2
        body.contactTestBitMask = 1
        return body
    }

    // MARK: - Game Control

    func startGame() {
        guard !gameRunning else { return }
        gameRunning = true
        ball.isHidden = false
        ball.position = CGPoint(x: size.width / 2, y: size.height / 2)
        let dx: CGFloat = role == .left ? 280 : -280
        ballVelocity = CGVector(dx: dx, dy: 200)
        ball.physicsBody?.velocity = ballVelocity
    }

    func receiveBall(velX: CGFloat, velY: CGFloat, ballY: CGFloat) {
        ball.isHidden = false
        // Ball enters from the appropriate edge
        let entryX: CGFloat = velX > 0 ? ballRadius + 5 : size.width - ballRadius - 5
        ball.position = CGPoint(x: entryX, y: ballY)
        ball.physicsBody?.velocity = CGVector(dx: velX, dy: velY)
        gameRunning = true
    }

    func movePaddle(to normalizedY: CGFloat) {
        let y = normalizedY * size.height
        let myPaddle = role == .left ? leftPaddle! : rightPaddle!
        let clamped = max(paddleH / 2 + 4, min(size.height - paddleH / 2 - 4, y))
        myPaddle.position = CGPoint(x: myPaddle.position.x, y: clamped)
        onMyPaddleMove?(clamped / size.height)
    }

    func updateFriendPaddle(normalizedY: Float) {
        let y = CGFloat(normalizedY) * size.height
        let friendPaddle = role == .left ? rightPaddle! : leftPaddle!
        let clamped = max(paddleH / 2 + 4, min(size.height - paddleH / 2 - 4, y))
        friendPaddle.position = CGPoint(x: friendPaddle.position.x, y: clamped)
    }

    // MARK: - Per-Frame Update

    override func update(_ currentTime: TimeInterval) {
        guard gameRunning, !ball.isHidden else { return }

        let w = size.width

        // Check ball exit
        if role == .left && ball.position.x > w + ballRadius {
            // Ball exits right → hand off to right phone
            let vel = ball.physicsBody!.velocity
            let normalY = ball.position.y / size.height
            onBallHandoff?(-abs(vel.dx), vel.dy, ball.position.y, leftScore, rightScore)
            ball.isHidden = true
            gameRunning = false

        } else if role == .right && ball.position.x < -ballRadius {
            // Ball exits left → hand off to left phone
            let vel = ball.physicsBody!.velocity
            onBallHandoff?(abs(vel.dx), vel.dy, ball.position.y, rightScore, leftScore)
            ball.isHidden = true
            gameRunning = false
        }

        // Score: left phone's left wall → right scores; right phone's right wall → left scores
        if role == .left && ball.position.x < -ballRadius {
            rightScore += 1
            rightScoreLabel.text = "\(rightScore)"
            onScoreChange?(leftScore, rightScore)
            resetBall()
        } else if role == .right && ball.position.x > w + ballRadius {
            leftScore += 1
            leftScoreLabel.text = "\(leftScore)"
            onScoreChange?(leftScore, rightScore)
            resetBall()
        }

        // Keep minimum ball speed
        let spd = hypot(ball.physicsBody!.velocity.dx, ball.physicsBody!.velocity.dy)
        if spd < 200 && spd > 0 {
            let scale = 250 / spd
            ball.physicsBody!.velocity = CGVector(
                dx: ball.physicsBody!.velocity.dx * scale,
                dy: ball.physicsBody!.velocity.dy * scale
            )
        }
    }

    private func resetBall() {
        ball.physicsBody?.velocity = .zero
        ball.position = CGPoint(x: size.width / 2, y: size.height / 2)
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) { [weak self] in
            guard let self, !self.ball.isHidden else { return }
            let dx: CGFloat = Bool.random() ? 280 : -280
            self.ball.physicsBody?.velocity = CGVector(dx: dx, dy: 200)
            self.gameRunning = true
        }
    }
}

// MARK: - Physics Contact

extension PongScene: SKPhysicsContactDelegate {
    func didBegin(_ contact: SKPhysicsContact) {
        // Slight speed bump on paddle hit
        let vel = ball.physicsBody!.velocity
        let spd = hypot(vel.dx, vel.dy)
        let newSpd = min(spd * 1.05, 500)
        let ratio = newSpd / max(spd, 1)
        ball.physicsBody!.velocity = CGVector(dx: vel.dx * ratio, dy: vel.dy * ratio)
    }
}

// MARK: - SwiftUI View

struct PongGameView: View {
    @EnvironmentObject var engine: AvatarEngine
    @Environment(\.dismiss) var dismiss
    @StateObject private var multipeer = MultipeerService.shared

    @State private var role: PongRole = .left
    @State private var scene = PongScene()
    @State private var myScore = 0
    @State private var friendScore = 0
    @State private var gameStarted = false
    @State private var dragY: CGFloat = 0

    private let sceneSize = CGSize(width: UIScreen.main.bounds.width,
                                   height: UIScreen.main.bounds.height - 140)

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            // SpriteKit game
            SpriteView(scene: scene)
                .frame(width: sceneSize.width, height: sceneSize.height)
                .ignoresSafeArea(edges: .bottom)
                .gesture(dragGesture)

            // HUD
            VStack {
                gameHeader
                Spacer()
                gameFooter
            }
        }
        .preferredColorScheme(.dark)
        .onAppear(perform: setupGame)
        .onDisappear { multipeer.disconnect() }
    }

    // MARK: - Setup

    private func setupGame() {
        scene.size = sceneSize
        scene.scaleMode = .resizeFill
        scene.role = multipeer.isConnected ? .left : .left  // First connector is host
        role = scene.role

        scene.onScoreChange = { l, r in
            myScore = role == .left ? l : r
            friendScore = role == .left ? r : l
        }

        scene.onBallHandoff = { velX, velY, ballY, myS, theirS in
            multipeer.send(.pongBallHandoff(
                velX: Float(velX),
                velY: Float(velY),
                ballY: Float(ballY),
                myScore: myS,
                theirScore: theirS
            ))
        }

        scene.onMyPaddleMove = { normalizedY in
            multipeer.send(.pongPaddleY(normalized: Float(normalizedY)), reliable: false)
        }

        multipeer.onPongPaddleY = { normalizedY in
            scene.updateFriendPaddle(normalizedY: normalizedY)
        }

        multipeer.onPongBallHandoff = { velX, velY, ballY, myS, theirS in
            myScore = theirS
            friendScore = myS
            scene.receiveBall(velX: CGFloat(velX), velY: CGFloat(velY), ballY: CGFloat(ballY))
        }

        multipeer.onPongEvent = { event in
            if event == "start" { scene.startGame() }
        }

        MultipeerService.shared.startDiscovery()
    }

    // MARK: - Drag Gesture (move paddle)

    private var dragGesture: some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                let normalizedY = value.location.y / sceneSize.height
                scene.movePaddle(to: normalizedY)
            }
    }

    // MARK: - Header

    private var gameHeader: some View {
        HStack {
            Button { dismiss() } label: {
                Image(systemName: "xmark")
                    .font(.system(size: 16, weight: .bold))
                    .foregroundStyle(.white.opacity(0.6))
            }
            .padding(.leading, 16)

            Spacer()

            // Scores
            HStack(spacing: 24) {
                VStack(spacing: 2) {
                    Text("\(myScore)")
                        .font(.system(size: 32, weight: .black, design: .monospaced))
                        .foregroundStyle(.cyan)
                    Text("YOU")
                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                        .foregroundStyle(.cyan.opacity(0.6))
                }
                Text("—")
                    .font(.system(size: 20))
                    .foregroundStyle(.white.opacity(0.3))
                VStack(spacing: 2) {
                    Text("\(friendScore)")
                        .font(.system(size: 32, weight: .black, design: .monospaced))
                        .foregroundStyle(.purple)
                    Text(multipeer.peerName.isEmpty ? "FRIEND" : multipeer.peerName.uppercased())
                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                        .foregroundStyle(.purple.opacity(0.6))
                        .lineLimit(1)
                }
            }

            Spacer()

            // Connection status
            Circle()
                .fill(multipeer.isConnected ? Color.green : Color.orange)
                .frame(width: 8, height: 8)
                .padding(.trailing, 16)
        }
        .padding(.top, 56)
        .padding(.bottom, 8)
        .background(.ultraThinMaterial)
    }

    // MARK: - Footer

    private var gameFooter: some View {
        VStack(spacing: 8) {
            if !gameStarted {
                Button {
                    gameStarted = true
                    scene.startGame()
                    multipeer.send(.pongEvent(event: "start"))
                } label: {
                    Text(multipeer.isConnected ? "START GAME" : "PLAY vs CPU")
                        .font(.system(size: 14, weight: .black, design: .monospaced))
                        .foregroundStyle(.black)
                        .padding(.horizontal, 32)
                        .padding(.vertical, 14)
                        .background(Capsule().fill(Color.cyan))
                }
            }

            if !multipeer.isConnected {
                Text("Place phones side by side for the full cross-screen experience")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.white.opacity(0.4))
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 32)
            }
        }
        .padding(.bottom, 24)
        .frame(maxWidth: .infinity)
        .background(.ultraThinMaterial)
    }
}
