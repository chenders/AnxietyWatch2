import Foundation
import Testing

@testable import AWTransport

private let fixedNow = Date(timeIntervalSince1970: 1_735_689_600)

private func fixturePrivateIPv4Host() -> String {
  [192, 168, 1, 42].map(String.init).joined(separator: ".")
}

@Test("origin validation accepts production HTTPS and private-LAN HTTP only")
func originValidation() throws {
  let production = try ClientOrigin(string: "https://CLIENT.example.com:443/")
  #expect(production.normalizedString == "https://client.example.com")
  let lanHost = fixturePrivateIPv4Host()
  let lan = try ClientOrigin(string: "http://\(lanHost):8480/")
  #expect(lan.normalizedString == "http://\(lanHost):8480")
  let lanIPv6Host = "fd00::42"
  let lanIPv6 = try ClientOrigin(string: "http://[\(lanIPv6Host)]:8480")
  #expect(lanIPv6.normalizedString == "http://[\(lanIPv6Host)]:8480")

  for value in [
    "http://client.example.com",
    "http://8.8.8.8:8480",
    "http://172.15.0.1:8480",
    "http://[2001:db8::1]:8480",
    "https://192.0.2.10",
    "https://[fd00::42]",
    "https://client.example.com:8443",
    "https://user:password@client.example.com",
    "https://client.example.com/path",
    "https://client.example.com/?token=secret",
  ] {
    do {
      _ = try ClientOrigin(string: value)
      Issue.record("accepted invalid origin \(value)")
    } catch is EnrollmentError {
      // Expected: each invalid origin is rejected before any request can be made.
    }
  }
}

@Test("confirmation binds the displayed origin and server identity")
func confirmationBindsIdentity() throws {
  let origin = try ClientOrigin(string: "https://client.example.com")
  let serverID = UUID(uuidString: "00000000-0000-0000-0000-000000000001")!
  let artifact = try EnrollmentArtifact(
    apiMajor: 1,
    origin: origin,
    serverInstanceID: serverID,
    enrollmentID: UUID(uuidString: "00000000-0000-0000-0000-000000000002")!,
    token: "fixture-token",
    serverDisplayLabel: "Fixture Pi",
    scopes: ["read:projections", "submit:domain_records"],
    expiresAt: fixedNow.addingTimeInterval(600)
  )

  let confirmation = try EnrollmentConfirmation(
    artifact: artifact,
    confirmedOrigin: origin,
    confirmedServerInstanceID: serverID,
    accepted: true,
    now: fixedNow
  )
  #expect(confirmation.artifact.origin == origin)

  do {
    _ = try EnrollmentConfirmation(
      artifact: artifact,
      confirmedOrigin: origin,
      confirmedServerInstanceID: UUID(),
      accepted: true,
      now: fixedNow
    )
    Issue.record("accepted an unmatching server identity")
  } catch let error as EnrollmentError {
    #expect(error == .identityMismatch("confirmed server instance does not match the artifact"))
  }
}

@Test("expired artifacts and declined confirmations fail closed")
func expiredArtifactAndDeclinedConfirmationFailClosed() throws {
  let origin = try ClientOrigin(string: "https://client.example.com")
  let artifact = try EnrollmentArtifact(
    apiMajor: 1,
    origin: origin,
    serverInstanceID: UUID(),
    enrollmentID: UUID(),
    token: "fixture-token",
    serverDisplayLabel: "Fixture Pi",
    scopes: ["read:projections"],
    expiresAt: fixedNow
  )

  do {
    try artifact.validate(at: fixedNow)
    Issue.record("accepted an expired artifact")
  } catch let error as EnrollmentError {
    #expect(error == .artifactExpired("enrollment artifact expired at \(fixedNow)"))
  }

  do {
    _ = try EnrollmentConfirmation(
      artifact: artifact,
      confirmedOrigin: origin,
      confirmedServerInstanceID: artifact.serverInstanceID,
      accepted: false,
      now: fixedNow.addingTimeInterval(-1)
    )
    Issue.record("accepted a declined enrollment confirmation")
  } catch let error as EnrollmentError {
    #expect(error == .confirmationRequired)
  }
}

@Test("enrollment creates and stores a device-bound credential")
func enrollmentStoresCredential() async throws {
  let origin = try ClientOrigin(string: "https://client.example.com")
  let artifact = try EnrollmentArtifact(
    apiMajor: 1,
    origin: origin,
    serverInstanceID: UUID(),
    enrollmentID: UUID(),
    token: "fixture-token",
    serverDisplayLabel: "Fixture Pi",
    scopes: ["read:projections"],
    expiresAt: fixedNow.addingTimeInterval(600)
  )
  let confirmation = try EnrollmentConfirmation(
    artifact: artifact,
    confirmedOrigin: origin,
    confirmedServerInstanceID: artifact.serverInstanceID,
    accepted: true,
    now: fixedNow
  )
  let keyStore = MockKeyStore()
  let credentialStore = MockCredentialStore()
  let transport = MockEnrollmentTransport(
    response: EnrollmentCredentialResponse(
      apiVersion: "1.0",
      clientID: UUID(),
      keyID: UUID(),
      scopes: ["read:projections"],
      issuedAt: "2026-01-01T00:00:00Z",
      expiresAt: "2026-06-30T00:00:00Z",
      serverTime: "2026-01-01T00:00:01Z"
    )
  )

  let coordinator = EnrollmentCoordinator(
    keyStore: keyStore,
    credentialStore: credentialStore,
    transport: transport
  )
  let credential = try await coordinator.enroll(
    confirmation: confirmation,
    deviceLabel: "Fixture iPhone",
    appVersion: "0.1"
  )

  #expect(credential.keyReference.reference == "fixture-key")
  #expect(credential.origin == origin)
  #expect(credentialStore.saved?.clientID == credential.clientID)
  #expect(transport.request?.algorithm == "ecdsa-p256-sha256")
  #expect(transport.request?.serverInstanceID == artifact.serverInstanceID)
  #expect(keyStore.deleted == false)
}

@Test("failed enrollment deletes the newly-created key and stores nothing")
func failedEnrollmentCleansKey() async throws {
  let origin = try ClientOrigin(string: "https://client.example.com")
  let artifact = try EnrollmentArtifact(
    apiMajor: 1,
    origin: origin,
    serverInstanceID: UUID(),
    enrollmentID: UUID(),
    token: "fixture-token",
    serverDisplayLabel: "Fixture Pi",
    scopes: ["read:projections"],
    expiresAt: fixedNow.addingTimeInterval(600)
  )
  let confirmation = try EnrollmentConfirmation(
    artifact: artifact,
    confirmedOrigin: origin,
    confirmedServerInstanceID: artifact.serverInstanceID,
    accepted: true,
    now: fixedNow
  )
  let keyStore = MockKeyStore()
  let credentialStore = MockCredentialStore()
  let transport = MockEnrollmentTransport(error: .serverRejected("request_signature_missing"))
  let coordinator = EnrollmentCoordinator(
    keyStore: keyStore,
    credentialStore: credentialStore,
    transport: transport
  )

  do {
    _ = try await coordinator.enroll(
      confirmation: confirmation,
      deviceLabel: "Fixture iPhone",
      appVersion: "0.1"
    )
    Issue.record("enrollment unexpectedly succeeded")
  } catch let error as EnrollmentError {
    #expect(error == .serverRejected("request_signature_missing"))
  }
  #expect(keyStore.deleted)
  #expect(credentialStore.saved == nil)
}

private final class MockKeyStore: DeviceKeyStore, @unchecked Sendable {
  var deleted = false

  func createKey() throws -> DeviceKeyReference {
    try DeviceKeyReference(reference: "fixture-key", publicKeySPKIDERBase64: "fixture-public-key")
  }

  func sign(_ message: Data, with reference: DeviceKeyReference) throws -> Data {
    Data("fixture-signature".utf8)
  }

  func deleteKey(_ reference: DeviceKeyReference) throws {
    deleted = true
  }
}

private final class MockCredentialStore: DeviceCredentialStore, @unchecked Sendable {
  var saved: StoredDeviceCredential?

  func load() throws -> StoredDeviceCredential? { saved }
  func save(_ credential: StoredDeviceCredential) throws { saved = credential }
  func delete() throws { saved = nil }
}

private final class MockEnrollmentTransport: EnrollmentClaiming, @unchecked Sendable {
  let response: EnrollmentCredentialResponse?
  let error: EnrollmentError?
  var request: EnrollmentClaimRequest?

  init(response: EnrollmentCredentialResponse) {
    self.response = response
    self.error = nil
  }

  init(error: EnrollmentError) {
    self.response = nil
    self.error = error
  }

  func claim(_ request: EnrollmentClaimRequest) async throws -> EnrollmentCredentialResponse {
    self.request = request
    if let error { throw error }
    guard let response else {
      throw EnrollmentError.serverRejected("test transport has no response")
    }
    return response
  }
}
