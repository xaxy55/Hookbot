import XCTest

/// Captures App Store screenshots for each avatar state.
/// Run with: make screenshots
final class ScreenshotTests: XCTestCase {

    let app = XCUIApplication()

    override func setUpWithError() throws {
        continueAfterFailure = false
        app.launch()
    }

    // MARK: - Screenshots

    @MainActor
    func test01_Idle() throws {
        // App launches in idle state by default
        sleep(2) // Let animation settle
        saveScreenshot("01_idle")
    }

    @MainActor
    func test02_Thinking() throws {
        app.buttons["state_thinking"].tap()
        sleep(2)
        saveScreenshot("02_thinking")
    }

    @MainActor
    func test03_Success() throws {
        app.buttons["state_success"].tap()
        sleep(2)
        saveScreenshot("03_success")
    }

    @MainActor
    func test04_Error() throws {
        app.buttons["state_error"].tap()
        sleep(2)
        saveScreenshot("04_error")
    }

    @MainActor
    func test05_Settings() throws {
        app.buttons["settingsButton"].tap()
        sleep(1) // Wait for sheet animation
        saveScreenshot("05_settings")
    }

    // MARK: - Helpers

    private func saveScreenshot(_ name: String) {
        let screenshot = app.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
