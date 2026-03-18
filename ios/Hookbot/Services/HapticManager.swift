#if canImport(UIKit)
import UIKit
#endif

// Haptic feedback as the iOS equivalent of the WS2812B LED

enum HapticManager {
    static func playStateHaptic(_ state: AvatarState) {
        #if targetEnvironment(macCatalyst)
        // No haptic engine on Mac
        #else
        switch state {
        case .idle:
            let generator = UIImpactFeedbackGenerator(style: .light)
            generator.impactOccurred()

        case .thinking:
            let generator = UIImpactFeedbackGenerator(style: .medium)
            generator.impactOccurred()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                generator.impactOccurred()
            }

        case .waiting:
            let generator = UIImpactFeedbackGenerator(style: .heavy)
            generator.impactOccurred()

        case .success:
            let generator = UINotificationFeedbackGenerator()
            generator.notificationOccurred(.success)

        case .taskcheck:
            let generator = UIImpactFeedbackGenerator(style: .medium)
            generator.impactOccurred()

        case .error:
            let generator = UINotificationFeedbackGenerator()
            generator.notificationOccurred(.error)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
                generator.notificationOccurred(.error)
            }
        }
        #endif
    }
}
