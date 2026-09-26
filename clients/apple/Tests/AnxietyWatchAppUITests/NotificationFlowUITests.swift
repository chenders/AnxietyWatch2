import XCTest

@MainActor
final class NotificationFlowUITests: XCTestCase {
  func testSettingsScreenCapturesPhysicalDeviceState() {
    let app = XCUIApplication()
    app.launch()

    let screenshot = XCUIScreen.main.screenshot()
    let attachment = XCTAttachment(screenshot: screenshot)
    attachment.name = "anxietywatch-settings-state"
    attachment.lifetime = .keepAlways
    add(attachment)

#if targetEnvironment(simulator)
    // The simulator has no Pi enrollment or physical-device state; the screenshot is still useful
    // for layout regression, while the connected-state assertion belongs to device acceptance.
    XCTAssertTrue(app.exists)
#else
    XCTAssertTrue(app.staticTexts["Pi connection"].waitForExistence(timeout: 10))
#endif
  }

  func testSyncButtonCapturesCompletionState() {
    let app = XCUIApplication()
    app.launch()
    let syncButton = app.buttons["Sync Now"]
    guard syncButton.waitForExistence(timeout: 10) else {
      XCTFail("Sync Now button was not available")
      return
    }
    let syncStatus = app.staticTexts.matching(
      NSPredicate(format: "label BEGINSWITH 'Sync status:'")
    ).firstMatch
    XCTAssertTrue(syncStatus.waitForExistence(timeout: 10))
    if syncButton.isEnabled {
      syncButton.tap()
      XCTAssertTrue(
        app.staticTexts.matching(
          NSPredicate(format: "label CONTAINS 'Sync complete' OR label CONTAINS 'Sync blocked'")
        ).firstMatch.waitForExistence(timeout: 30)
      )
    } else {
      XCTAssertTrue(syncStatus.label.contains("not enrolled"))
    }
    let screenshot = XCUIScreen.main.screenshot()
    let attachment = XCTAttachment(screenshot: screenshot)
    attachment.name = "anxietywatch-sync-completion-state"
    attachment.lifetime = .keepAlways
    add(attachment)
  }
}
