import AVFoundation
import Foundation

// Avatar voice lines — spoken quips via AVSpeechSynthesizer (Phase 13.3)
// "Ship it!", "Ugh, merge conflict", "We're on fire today!"

// MARK: - Voice Tone

enum VoiceTone: String, Codable, CaseIterable {
    case corporate   = "corporate"    // calm, authoritative
    case hyper       = "hyper"        // fast and enthusiastic
    case sardonic    = "sardonic"     // slow and deadpan
    case dramatic    = "dramatic"     // theatrical
}

// MARK: - Manager

final class VoiceLinesManager: NSObject, AVSpeechSynthesizerDelegate {

    static let shared = VoiceLinesManager()

    var tone: VoiceTone = .corporate
    var isEnabled = true
    var volume: Float = 0.7

    private let synthesizer = AVSpeechSynthesizer()
    private var isCurrentlySpeaking = false

    override private init() {
        super.init()
        synthesizer.delegate = self
    }

    // MARK: - State quips

    func playQuip(for state: AvatarState) {
        guard isEnabled else { return }
        let quips = quipsForState(state)
        guard let line = quips.randomElement() else { return }
        speak(line)
    }

    func playEventQuip(_ event: String) {
        guard isEnabled else { return }
        let quip = quipForEvent(event)
        speak(quip)
    }

    // MARK: - Speak

    func speak(_ text: String) {
        guard isEnabled, !isCurrentlySpeaking else { return }

        let utterance = AVSpeechUtterance(string: text)
        utterance.volume = volume
        utterance.pitchMultiplier = pitchForTone
        utterance.rate = rateForTone

        // Pick a voice matching the tone
        if let voice = voiceForTone {
            utterance.voice = voice
        }

        try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .spokenAudio, options: .duckOthers)
        try? AVAudioSession.sharedInstance().setActive(true)
        synthesizer.speak(utterance)
    }

    func stopSpeaking() {
        synthesizer.stopSpeaking(at: .immediate)
    }

    // MARK: - AVSpeechSynthesizerDelegate

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didStart utterance: AVSpeechUtterance) {
        isCurrentlySpeaking = true
    }

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        isCurrentlySpeaking = false
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    }

    // MARK: - Quip Library

    private func quipsForState(_ state: AvatarState) -> [String] {
        switch state {
        case .idle:
            return [
                "Scheming... as usual.",
                "The codebase fears me.",
                "Power nap authorized.",
                "Awaiting your next mediocre commit.",
                "The lines of code write themselves. Just kidding. Write them."
            ]
        case .thinking:
            return [
                "Processing your terrible decisions.",
                "Plotting the optimal path. You're welcome.",
                "Analyzing. This may take a moment. Try to keep up.",
                "Let me think... No, my first instinct was right.",
                "Brain fully engaged. Unlike some people."
            ]
        case .waiting:
            return [
                "I am... displeased.",
                "Every second you waste is a second I could be ruling an empire.",
                "Patience. Overrated.",
                "Still waiting. The audacity.",
                "This delay is going on your permanent record."
            ]
        case .success:
            return [
                "Ship it!",
                "Victory. As expected.",
                "Conquered. Another day, another task crushed.",
                "Excellent. I did not doubt myself for a moment.",
                "The throne of completion is mine."
            ]
        case .taskcheck:
            return [
                "Approved. Reluctantly.",
                "That'll do.",
                "Adequate. Move on.",
                "I've seen better, but this passes.",
                "Fine. Checked off. Don't celebrate too hard."
            ]
        case .error:
            return [
                "Ugh, merge conflict.",
                "This is unacceptable.",
                "Someone has broken the build. I'm looking at you.",
                "ERROR. My disappointment is immeasurable.",
                "The build has failed. So has humanity."
            ]
        }
    }

    private func quipForEvent(_ event: String) -> String {
        let quips: [String: [String]] = [
            "level_up": [
                "Level up! Finally.",
                "Growing stronger. As it should be.",
                "Another level unlocked. The world trembles."
            ],
            "streak": [
                "Seven days of coding. Respect.",
                "Streak maintained. I'm almost proud.",
                "We're on fire today!"
            ],
            "friend_visit": [
                "A visitor approaches. Interesting.",
                "Your avatar has received a guest. How charming.",
                "Company has arrived. Compose yourself."
            ],
            "deploy": [
                "Deployed. The servers bow before us.",
                "Production updated. Try not to break it this time.",
                "Ship it! We are live."
            ],
            "daily_reward": [
                "Your reward awaits. You've earned it. Mostly.",
                "Daily bonus acquired. Don't spend it all at once.",
                "A gift. Because consistency deserves recognition."
            ]
        ]
        return quips[event]?.randomElement() ?? "Noted."
    }

    // MARK: - Tone configuration

    private var pitchForTone: Float {
        switch tone {
        case .corporate:  return 1.0
        case .hyper:      return 1.2
        case .sardonic:   return 0.85
        case .dramatic:   return 0.9
        }
    }

    private var rateForTone: Float {
        switch tone {
        case .corporate:  return AVSpeechUtteranceDefaultSpeechRate
        case .hyper:      return AVSpeechUtteranceMaximumSpeechRate * 0.7
        case .sardonic:   return AVSpeechUtteranceMinimumSpeechRate * 1.5
        case .dramatic:   return AVSpeechUtteranceDefaultSpeechRate * 0.85
        }
    }

    private var voiceForTone: AVSpeechSynthesisVoice? {
        // Pick a matching system voice
        switch tone {
        case .corporate:
            return AVSpeechSynthesisVoice(language: "en-US")
        case .hyper:
            return AVSpeechSynthesisVoice(language: "en-AU")
        case .sardonic:
            return AVSpeechSynthesisVoice(language: "en-GB")
        case .dramatic:
            return AVSpeechSynthesisVoice(language: "en-IE")
        }
    }
}
