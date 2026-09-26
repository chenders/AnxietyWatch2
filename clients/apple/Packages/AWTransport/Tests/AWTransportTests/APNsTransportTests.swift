import Foundation
import Testing

@testable import AWTransport

private let notificationDeviceID = UUID(uuidString: "00000000-0000-0000-0000-000000000101")!
private let notificationMessageID = UUID(uuidString: "00000000-0000-0000-0000-000000000102")!

@Test("APNs registration normalizes tokens and preserves authorization facts")
func registrationContract() throws {
  let request = try APNsRegistrationRequest(
    deviceID: notificationDeviceID,
    token: "A1B2C3D4",
    environment: "production",
    appIdentifier: "com.example.AnxietyWatch",
    appVersion: "1.2.3",
    build: "42",
    authorizationStatus: "authorized",
    alertsEnabled: true,
    soundsEnabled: false,
    badgesEnabled: true
  )

  #expect(request.token == "a1b2c3d4")
  #expect(request.soundsEnabled == false)
  let encoded = try JSONEncoder().encode(request)
  let decoded = try JSONDecoder().decode(APNsRegistrationRequest.self, from: encoded)
  #expect(decoded == request)
}

@Test("APNs registration rejects malformed token and environment")
func invalidRegistration() {
  for input in [
    ("not-hex", "production"),
    ("aa", "development"),
    ("", "sandbox")
  ] {
    do {
      _ = try APNsRegistrationRequest(
        deviceID: notificationDeviceID,
        token: input.0,
        environment: input.1,
        appIdentifier: "com.example.AnxietyWatch",
        appVersion: "1.0",
        build: "1",
        authorizationStatus: "denied",
        alertsEnabled: false,
        soundsEnabled: false,
        badgesEnabled: false
      )
      Issue.record("accepted invalid APNs registration \(input)")
    } catch is APNsError {
      // Expected: registration fails closed before authenticated transport.
    } catch {
      Issue.record("unexpected error for invalid APNs registration")
    }
  }
}

@Test("remote message parser rejects sensitive payload fields")
func minimalMessagePayload() throws {
  let message = try APNsMessage(userInfo: [
    "aps": ["category": "operator_message"],
    "message_id": notificationMessageID.uuidString,
    "payload_class": "operator_message",
    "category": "operator_message",
    "deep_link": "anxietywatch://operator-message/102",
    "is_test": true
  ])
  #expect(message.messageID == notificationMessageID)
  #expect(message.isTest)

  do {
    _ = try APNsMessage(userInfo: [
      "message_id": notificationMessageID.uuidString,
      "payload_class": "operator_message",
      "body": "a private journal entry"
    ])
    Issue.record("accepted sensitive APNs body")
  } catch let error as APNsError {
    #expect(error == .invalidMessage("remote payload contains unsupported field body"))
  }
}

@Test("display and acknowledgement evidence remain independent and idempotent")
func independentOutcomeEvidence() async throws {
  let ledger = try APNsOutcomeLedger(maximumEntries: 2)
  #expect(await ledger.shouldReport(messageID: notificationMessageID, outcome: .displayed))
  await ledger.markReported(messageID: notificationMessageID, outcome: .displayed)
  #expect(!(await ledger.shouldReport(messageID: notificationMessageID, outcome: .displayed)))
  #expect(await ledger.shouldReport(messageID: notificationMessageID, outcome: .acknowledged))
  await ledger.markReported(messageID: notificationMessageID, outcome: .acknowledged)
  #expect(!(await ledger.shouldReport(messageID: notificationMessageID, outcome: .acknowledged)))
}

@Test("outcome contract requires an observation timestamp")
func outcomeContract() throws {
  let outcome = try APNsOutcomeRequest(
    messageID: notificationMessageID,
    outcome: .acknowledged,
    observedAt: "2026-09-18T00:00:00Z"
  )
  let decoded = try JSONDecoder().decode(
    APNsOutcomeRequest.self,
    from: JSONEncoder().encode(outcome)
  )
  #expect(decoded == outcome)
}
