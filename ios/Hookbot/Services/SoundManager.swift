import AVFoundation
<<<<<<< Updated upstream
import CoreAudioTypes
=======
>>>>>>> Stashed changes

// Port of sound.cpp - generates buzzer-style tones matching the ESP firmware

final class SoundManager: ObservableObject {
    private var audioEngine: AVAudioEngine?
    private var sourceNode: AVAudioSourceNode?
    private var noteQueue: [(freq: Float, duration: Float)] = []
    private var currentNoteIndex = 0
    private var noteStartTime: Double = 0
    private var currentFreq: Float = 0
    private var phase: Double = 0
    private var isPlaying = false

    init() {
        setupAudio()
    }

    private func setupAudio() {
        audioEngine = AVAudioEngine()
        guard let audioEngine else { return }

        let sampleRate = audioEngine.outputNode.outputFormat(forBus: 0).sampleRate

        sourceNode = AVAudioSourceNode { [weak self] _, _, frameCount, audioBufferList -> OSStatus in
            guard let self else { return noErr }
<<<<<<< Updated upstream
            let bufferCount = Int(audioBufferList.pointee.mNumberBuffers)
=======
            let ablPointer = UnsafeMutableAudioBufferListPointer(audioBufferList)
>>>>>>> Stashed changes

            for frame in 0..<Int(frameCount) {
                let sample: Float
                if self.currentFreq > 0 && self.isPlaying {
                    // Square wave (matches passive buzzer sound)
                    let value = sin(self.phase * 2.0 * .pi)
                    sample = value > 0 ? 0.15 : -0.15
                    self.phase += Double(self.currentFreq) / sampleRate
                } else {
                    sample = 0
                }

<<<<<<< Updated upstream
                // Write to each audio buffer directly
                withUnsafeMutablePointer(to: &audioBufferList.pointee.mBuffers) { ptr in
                    let buffers = UnsafeMutableRawPointer(ptr).assumingMemoryBound(to: AudioBuffer.self)
                    for i in 0..<bufferCount {
                        let buf = buffers[i].mData?.assumingMemoryBound(to: Float.self)
                        buf?[frame] = sample
                    }
=======
                for buffer in ablPointer {
                    let buf = buffer.mData?.assumingMemoryBound(to: Float.self)
                    buf?[frame] = sample
>>>>>>> Stashed changes
                }
            }
            return noErr
        }

        guard let sourceNode else { return }
        audioEngine.attach(sourceNode)

        let format = audioEngine.outputNode.inputFormat(forBus: 0)
        audioEngine.connect(sourceNode, to: audioEngine.mainMixerNode, format: format)

        do {
<<<<<<< Updated upstream
            #if !targetEnvironment(macCatalyst)
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default, options: .mixWithOthers)
            try AVAudioSession.sharedInstance().setActive(true)
            #endif
=======
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default, options: .mixWithOthers)
            try AVAudioSession.sharedInstance().setActive(true)
>>>>>>> Stashed changes
            try audioEngine.start()
        } catch {
            print("[Sound] Audio engine failed: \(error)")
        }
    }

    // MARK: - State Sounds (matches sound.cpp exactly)

    func playStateSound(_ state: AvatarState) {
        noteQueue.removeAll()
        currentNoteIndex = 0
        isPlaying = false
        currentFreq = 0

        switch state {
        case .idle:
            enqueue(110, 120)
            enqueue(0, 30)
            enqueue(110, 80)

        case .thinking:
            enqueue(330, 50)
            enqueue(0, 20)
            enqueue(370, 50)
            enqueue(0, 20)
            enqueue(415, 60)

        case .waiting:
            enqueue(100, 200)

        case .success:
            enqueue(440, 80)
            enqueue(0, 20)
            enqueue(554, 80)
            enqueue(0, 20)
            enqueue(659, 80)
            enqueue(0, 20)
            enqueue(880, 200)

        case .taskcheck:
            enqueue(660, 60)
            enqueue(0, 60)
            enqueue(880, 80)

        case .error:
            enqueue(880, 80)
            enqueue(0, 20)
            enqueue(440, 80)
            enqueue(0, 20)
            enqueue(220, 120)
            enqueue(0, 20)
            enqueue(110, 200)
        }

        startPlayback()
    }

    // MARK: - Escalation Beeps

    func playEscalationBeep(secs: Float) {
        guard !isPlaying else { return }

        noteQueue.removeAll()
        currentNoteIndex = 0

        if secs < 6.0 {
            enqueue(300, 40)
        } else if secs < 10.0 {
            enqueue(500, 50)
            enqueue(0, 30)
            enqueue(600, 60)
        } else {
            enqueue(800, 60)
            enqueue(0, 20)
            enqueue(400, 60)
            enqueue(0, 20)
            enqueue(800, 60)
        }

        startPlayback()
    }

    // MARK: - Note Queue

    private func enqueue(_ freq: Float, _ duration: Float) {
        noteQueue.append((freq: freq, duration: duration))
    }

    private func startPlayback() {
        guard !noteQueue.isEmpty else { return }
        currentNoteIndex = 0
        noteStartTime = CACurrentMediaTime() * 1000
        currentFreq = noteQueue[0].freq
        isPlaying = true
        phase = 0

        // Schedule advancement
        advanceNotes()
    }

    private func advanceNotes() {
        guard currentNoteIndex < noteQueue.count else {
            isPlaying = false
            currentFreq = 0
            return
        }

        let note = noteQueue[currentNoteIndex]
        currentFreq = note.freq
        phase = 0

        DispatchQueue.global().asyncAfter(deadline: .now() + .milliseconds(Int(note.duration))) { [weak self] in
            guard let self else { return }
            self.currentNoteIndex += 1
            if self.currentNoteIndex < self.noteQueue.count {
                self.advanceNotes()
            } else {
                self.isPlaying = false
                self.currentFreq = 0
            }
        }
    }
}
