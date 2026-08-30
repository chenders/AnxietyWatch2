import Foundation
import Testing

@testable import AWTransport

private func fixture(named name: String) throws -> Data {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0..<7 {
    root.deleteLastPathComponent()
  }
  return try Data(contentsOf: root.appending(path: "api/fixtures/client-v1/\(name)"))
}

@Test("shared enrollment fixture preserves algorithm and server identity")
func enrollmentFixtureDecodes() throws {
  let request = try JSONDecoder().decode(
    EnrollmentClaimRequest.self,
    from: fixture(named: "enrollment-request.json")
  )
  #expect(request.apiMajor == 1)
  #expect(request.algorithm == "ecdsa-p256-sha256")
  #expect(request.deviceLabel == "Fixture iPhone")
}

@Test("shared item acknowledgement fixture preserves terminal wire status")
func acknowledgementFixtureDecodes() throws {
  let response = try JSONDecoder().decode(
    ClientWriteResponse.self,
    from: fixture(named: "write-batch-ack-response.json")
  )
  #expect(response.acknowledgements.first?.status == "created")
  #expect(response.acknowledgements.first?.retryable == false)
}

@Test("shared request fixture ignores additive fields and preserves opaque content")
func requestFixtureIsForwardCompatible() throws {
  let request = try JSONDecoder().decode(
    ClientWriteBatch.self,
    from: fixture(named: "write-batch-request.json")
  )
  #expect(request.items.count == 1)
  if case .object(let content) = request.items.first?.content {
    #expect(content["anxiety_rating"] == .number(6))
  } else {
    Issue.record("fixture content was not preserved as an object")
  }
}

@Test("unknown future error fields do not turn an incompatible major into success")
func versionErrorFixtureDecodes() throws {
  let error = try JSONDecoder().decode(
    ClientErrorResponse.self,
    from: fixture(named: "error-version-incompatible.json")
  )
  #expect(error.code == "api_version_incompatible")
  #expect(error.retryable == false)
  #expect(error.requestedMajor == 2)
}

@Test("authentication and replay blockers remain distinct")
func authenticationFixturesDecode() throws {
  let authentication = try JSONDecoder().decode(
    ClientErrorResponse.self,
    from: fixture(named: "error-authentication-required.json")
  )
  let replay = try JSONDecoder().decode(
    ClientErrorResponse.self,
    from: fixture(named: "error-request-replayed.json")
  )
  #expect(authentication.code == "request_signature_missing")
  #expect(replay.code == "request_replayed")
  #expect(authentication.retryable == false)
  #expect(replay.retryable == false)
}
