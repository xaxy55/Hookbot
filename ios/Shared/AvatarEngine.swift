import Foundation
import Combine

// Port of avatar.cpp animation state machine

final class AvatarEngine: ObservableObject {
    // Published for SwiftUI rendering (read-only from views)
    @Published private(set) var current = AvatarParams()
    @Published var currentState: AvatarState = .idle
    @Published var currentTool = ToolInfo()
    @Published var tasks: [TaskItem] = []
    @Published var activeTaskIndex: Int = 0
    @Published private(set) var ledColor = LEDColor(r: 0, g: 0, b: 0.12)

    // Exposed for renderer
    private(set) var stateTime: Float = 0
    private(set) var totalTime: Float = 0
    var stateEnteredAt: Date = Date()

    private var target = AvatarParams()

    // Blink state
    private var nextBlink: Float = 4.0
    private var blinkStart: Float = 0
    private var blinking = false

    // Waiting escalation
    private var lastBeepTime: Float = 0

    // LED
    private var ledStateTime: Float = 0

    var config = RuntimeConfig()

    // Callbacks
    var onPlaySound: ((AvatarState) -> Void)?
    var onPlayEscalationBeep: ((Float) -> Void)?
    var onHaptic: ((AvatarState) -> Void)?

    // Frame timer - drives the animation loop at 30fps
    let frameTimer = Timer.publish(every: 1.0 / 30.0, on: .main, in: .common).autoconnect()
    private var timerCancellable: AnyCancellable?

    init() {
        setIdleTarget()
        current = target

        // Start the animation loop
        timerCancellable = frameTimer.sink { [weak self] _ in
            self?.tick()
        }
    }

    private func tick() {
        let deltaMs: Float = 33.0 // ~30fps
        update(deltaMs: deltaMs)
    }

    // MARK: - State Management

    func setState(_ state: AvatarState) {
        let changed = state != currentState
        currentState = state
        stateTime = 0
        stateEnteredAt = Date()
<<<<<<< Updated upstream
        UserDefaults.standard.set(state.rawValue, forKey: "hookbot_current_state")
=======
>>>>>>> Stashed changes
        blinking = false
        lastBeepTime = 0
        ledStateTime = 0

        switch state {
        case .idle:      setIdleTarget()
        case .thinking:  setThinkingTarget()
        case .waiting:   setWaitingTarget()
        case .success:   setSuccessTarget()
        case .taskcheck: setTaskcheckTarget()
        case .error:     setErrorTarget()
        }

        if changed {
            onPlaySound?(state)
            onHaptic?(state)
        }
    }

    func setTool(name: String, detail: String) {
        currentTool = ToolInfo(name: name, detail: detail)
    }

    // MARK: - Target Parameter Generators (CEO personality)

    private func setIdleTarget() {
        target.eyeX = 0
        target.eyeY = 0
        target.eyeOpen = 0.9
        target.mouthCurve = 0.15
        target.mouthOpen = 0
        target.bounce = 0
        target.shake = 0
        target.browAngle = -0.4
        target.browY = 0
    }

    private func setThinkingTarget() {
        target.eyeOpen = 0.7
        target.mouthCurve = -0.2
        target.mouthOpen = 0
        target.bounce = 0
        target.shake = 0
        target.browAngle = -0.7
        target.browY = -1.0
    }

    private func setWaitingTarget() {
        target.eyeX = 0
        target.eyeY = 0
        target.eyeOpen = 0.85
        target.mouthCurve = -0.2
        target.mouthOpen = 0
        target.bounce = 0
        target.shake = 0
        target.browAngle = -0.4
        target.browY = 0
    }

    private func setSuccessTarget() {
        target.eyeX = 0
        target.eyeY = 0
        target.eyeOpen = 0.6
        target.mouthCurve = 1.0
        target.mouthOpen = 0.3
        target.bounce = 0
        target.shake = 0
        target.browAngle = -0.6
        target.browY = 0.5
    }

    private func setTaskcheckTarget() {
        target.eyeX = 0
        target.eyeY = 0
        target.eyeOpen = 0.75
        target.mouthCurve = 0.3
        target.mouthOpen = 0
        target.bounce = 0
        target.shake = 0
        target.browAngle = -0.3
        target.browY = 0.3
    }

    private func setErrorTarget() {
        target.eyeX = 0
        target.eyeY = 0
        target.eyeOpen = 1.2
        target.mouthCurve = -1.0
        target.mouthOpen = 0.4
        target.bounce = 0
        target.shake = 0
        target.browAngle = -1.0
        target.browY = -2.0
    }

    // MARK: - Update (called by timer at ~30fps)

    private func update(deltaMs: Float) {
        stateTime += deltaMs
        totalTime += deltaMs
        let dt = deltaMs
        let phase = stateTime / 1000.0

        // Auto-return from transient states (3s)
        if (currentState == .success || currentState == .taskcheck) && stateTime > 3000 {
            setState(.idle)
            return
        }

        // Blink logic - CEOs blink less
        if !blinking && stateTime / 1000.0 > nextBlink {
            blinking = true
            blinkStart = stateTime / 1000.0
            nextBlink = stateTime / 1000.0 + 4.0 + Float.random(in: 0...3)
        }
        if blinking {
            let blinkAge = (stateTime / 1000.0 - blinkStart) * 1000
            if blinkAge < 60 {
                target.eyeOpen = 0
            } else if blinkAge < 120 {
                blinking = false
            }
        }

        // State-specific animations
        switch currentState {
        case .idle:
            if !blinking { setIdleTarget() }
            let secs = stateTime / 1000.0

            if secs < 60 {
                let breath = sinf(phase * 0.5) * 1.2
                target.bounce = breath
                let glance = sinf(phase * 0.3)
                if glance > 0.85 {
                    target.eyeX = sinf(phase * 1.5) * 0.5
                }
            } else {
                // CEO nap
                let sleepDepth = min(1.0, (secs - 60) / 10.0)
                target.eyeOpen = 0.1 * (1.0 - sleepDepth)
                target.mouthCurve = 0
                target.mouthOpen = 0
                target.browAngle = -0.1
                target.browY = 0.5
                target.eyeX = 0
                target.eyeY = 0
                let snore = sinf(phase * 0.3)
                target.bounce = snore * 2.0
                if snore > 0.7 {
                    target.mouthOpen = 0.15
                }
            }

        case .thinking:
            if !blinking { setThinkingTarget() }
            target.eyeX = sinf(phase * 3.5) * 0.9
            target.eyeY = sinf(phase * 1.8) * 0.4
            target.shake = sinf(phase * 8.0) * 0.3

        case .waiting:
            if !blinking { setWaitingTarget() }
            let secs = stateTime / 1000.0

            if secs < 3.0 {
                let tap = sinf(phase * 3.0)
                if tap > 0.7 { target.bounce = -1.5 }
                target.mouthCurve = -0.2
                target.browAngle = -0.4
                target.eyeX = sinf(phase * 1.0) * 0.4
            } else if secs < 6.0 {
                let tap = sinf(phase * 5.0)
                if tap > 0.5 { target.bounce = -2.0 }
                target.mouthCurve = -0.5
                target.browAngle = -0.6
                target.browY = -1.0
                target.eyeOpen = 0.75
                target.eyeX = sinf(phase * 2.0) * 0.7
            } else if secs < 10.0 {
                target.shake = sinf(phase * 6.0) * 2.0
                let stomp = sinf(phase * 7.0)
                if stomp > 0.3 { target.bounce = -3.0 * stomp }
                target.mouthCurve = -0.8
                target.mouthOpen = 0.2
                target.browAngle = -0.8
                target.browY = -1.5
                target.eyeOpen = 0.65
                target.eyeX = sinf(phase * 3.0) * 0.5
            } else {
                // FULL TANTRUM
                target.shake = sinf(phase * 12.0) * 4.0
                target.bounce = sinf(phase * 9.0) * 4.0
                target.mouthCurve = -1.0
                target.mouthOpen = 0.5
                target.browAngle = -1.0
                target.browY = -2.0
                let eyePulse = sinf(phase * 4.0)
                target.eyeOpen = eyePulse > 0 ? 1.2 : 0.5
                target.eyeX = sinf(phase * 5.0) * 1.0
                target.eyeY = sinf(phase * 3.5) * 0.5
            }

            // Escalation beeps
            if secs > 3.0 {
                let interval: Float
                if secs < 6.0 { interval = 4.0 }
                else if secs < 10.0 { interval = 2.0 }
                else { interval = 0.8 }

                if secs - lastBeepTime >= interval {
                    lastBeepTime = secs
                    onPlayEscalationBeep?(secs)
                }
            }

        case .success:
            if !blinking { setSuccessTarget() }
            if stateTime < 1000 {
                let t = stateTime / 1000.0
                target.bounce = sinf(t * .pi * 5) * (1.0 - t) * 5.0
            } else {
                target.bounce = 0
                target.mouthOpen = 0.1
            }

        case .taskcheck:
            if !blinking { setTaskcheckTarget() }
            if stateTime < 400 {
                let t = stateTime / 400.0
                target.bounce = sinf(t * .pi) * 3.0
            } else {
                target.bounce = 0
            }

        case .error:
            if !blinking { setErrorTarget() }
            if stateTime < 800 {
                let t = stateTime / 800.0
                target.shake = sinf(t * .pi * 10) * (1.0 - t * 0.5) * 5.0
            } else {
                target.shake = sinf(phase * 6.0) * 0.5
                target.eyeOpen = 0.6
                target.mouthOpen = 0
                target.mouthCurve = -0.9
            }
        }

        // Smooth interpolation
        let speed: Float = 8.0
        current.eyeX = smoothStep(current.eyeX, target.eyeX, speed, dt)
        current.eyeY = smoothStep(current.eyeY, target.eyeY, speed, dt)
        current.eyeOpen = smoothStep(current.eyeOpen, target.eyeOpen, 12.0, dt)
        current.mouthCurve = smoothStep(current.mouthCurve, target.mouthCurve, speed, dt)
        current.mouthOpen = smoothStep(current.mouthOpen, target.mouthOpen, speed, dt)
        current.bounce = smoothStep(current.bounce, target.bounce, 10.0, dt)
        current.shake = smoothStep(current.shake, target.shake, 12.0, dt)
        current.browAngle = smoothStep(current.browAngle, target.browAngle, speed, dt)
        current.browY = smoothStep(current.browY, target.browY, speed, dt)

        // Update LED color
        updateLED(deltaMs: deltaMs)
    }

    // MARK: - LED Color (glow effect)

    private func updateLED(deltaMs: Float) {
        ledStateTime += deltaMs
        let phase = ledStateTime / 1000.0

        switch currentState {
        case .idle:
            let val = 0.12 + 0.1 * (sinf(phase * 1.2) * 0.5 + 0.5)
            ledColor = LEDColor(r: 0, g: 0, b: val)

        case .thinking:
            let r = 0.24 + 0.16 * (sinf(phase * 3.0) * 0.5 + 0.5)
            let b = 0.32 + 0.16 * (cosf(phase * 3.0) * 0.5 + 0.5)
            ledColor = LEDColor(r: r, g: 0, b: b)

        case .waiting:
            let val = 0.16 + 0.12 * (sinf(phase * 1.5) * 0.5 + 0.5)
            ledColor = LEDColor(r: val, g: val - 0.04, b: val - 0.08)

        case .success:
            if ledStateTime < 200 {
                ledColor = LEDColor(r: 0, g: 0.78, b: 0)
            } else {
                let fade = max(0, 1.0 - (ledStateTime - 200) / 2000.0)
                ledColor = LEDColor(r: 0, g: 0.47 * fade, b: 0)
            }

        case .taskcheck:
            if ledStateTime < 300 {
                let val = 0.31 * (ledStateTime / 300.0)
                ledColor = LEDColor(r: 0, g: val, b: val)
            } else {
                let fade = max(0, 1.0 - (ledStateTime - 300) / 2000.0)
                ledColor = LEDColor(r: 0, g: 0.31 * fade, b: 0.31 * fade)
            }

        case .error:
            let flash = Int(ledStateTime) % 400 < 200
            if ledStateTime < 800 && flash {
                ledColor = LEDColor(r: 0.78, g: 0, b: 0)
            } else if ledStateTime >= 800 {
                let fade = max(0, 1.0 - (ledStateTime - 800) / 1500.0)
                ledColor = LEDColor(r: 0.39 * fade, g: 0, b: 0)
            } else {
                ledColor = LEDColor(r: 0, g: 0, b: 0)
            }
        }
    }

    // MARK: - Easing

    private func smoothStep(_ a: Float, _ b: Float, _ speed: Float, _ dt: Float) -> Float {
        let t = 1.0 - expf(-speed * dt / 1000.0)
        return a + (b - a) * t
    }
}
