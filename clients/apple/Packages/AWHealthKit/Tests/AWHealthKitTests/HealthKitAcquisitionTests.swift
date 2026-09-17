import Foundation
import Testing
@testable import AWHealthKit

private actor AuthorizationClientFixture: HealthKitAuthorizationRequesting {
  private(set) var requestCount = 0
  let status: String

  init(status: String = "should_request") {
    self.status = status
  }

  func requestReadAuthorization(for types: [HealthKitReadTypeDefinition]) async throws {
    #expect(!types.isEmpty)
    #expect(types.allSatisfy { !$0.rationale.isEmpty })
    requestCount += 1
  }

  func requestStatus(for types: [HealthKitReadTypeDefinition]) async -> String {
    #expect(!types.isEmpty)
    return status
  }
}

private actor AuthorizationFactsStoreFixture: HealthKitAuthorizationFactStore {
  private var stored = HealthKitAuthorizationFacts()

  func loadAuthorizationFacts() async throws -> HealthKitAuthorizationFacts { stored }

  func saveAuthorizationFacts(_ facts: HealthKitAuthorizationFacts) async throws {
    stored = facts
  }
}

@Test("first release plan retains the prior app's source metrics without write access")
func firstReleasePlanIsReadOnlyAndExplained() {
  let plan = HealthKitReadPlan.firstRelease
  #expect(plan.count == 26)
  #expect(plan.allSatisfy { !$0.rationale.isEmpty })
  #expect(plan.allSatisfy { $0.kind == .category || $0.platformUnit != nil })
  #expect(!plan.contains { $0.identifier.contains("Clinical") })
  #expect(!plan.contains { $0.identifier.contains("Heartbeat") })
}

@Test("authorization facts distinguish requested from granted")
func authorizationFactsAreTruthful() async {
  let client = AuthorizationClientFixture(status: "unnecessary")
  let store = AuthorizationFactsStoreFixture()
  let coordinator = HealthKitAuthorizationCoordinator(
    client: client,
    factsStore: store,
    now: { Date(timeIntervalSince1970: 1_735_689_600) }
  )

  let first = await coordinator.requestIfNeeded()
  #expect(first.outcome == "requested")
  #expect(first.requestStatus == "unnecessary")
  #expect(first.outcome != "granted")

  let second = await coordinator.requestIfNeeded()
  #expect(second == first)
  #expect(await client.requestCount == 1)
}

@Test("forced authorization retry records a failed request without claiming access")
func failedAuthorizationIsVisible() async {
  let client = FailingAuthorizationClientFixture()
  let store = AuthorizationFactsStoreFixture()
  let coordinator = HealthKitAuthorizationCoordinator(
    client: client,
    factsStore: store,
    now: { Date(timeIntervalSince1970: 1_735_689_600) }
  )

  let result = await coordinator.requestIfNeeded()
  #expect(result.outcome?.hasPrefix("failed:") == true)
  #expect(result.requestStatus == "status_failed")
}

private actor FailingAuthorizationClientFixture: HealthKitAuthorizationRequesting {
  func requestReadAuthorization(for types: [HealthKitReadTypeDefinition]) async throws {
    struct FixtureError: Error {}
    throw FixtureError()
  }

  func requestStatus(for types: [HealthKitReadTypeDefinition]) async -> String { "status_failed" }
}

@Test("anchor codec preserves opaque anchor bytes")
func anchorCodecIsLossless() {
  let input = Data([0, 1, 2, 127, 255])
  #expect(HealthKitAnchorCodec.decode(HealthKitAnchorCodec.encode(input)) == input)
  #expect(HealthKitAnchorCodec.decode(nil) == nil)
}
