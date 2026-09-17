import AWOutbox
import Foundation
import Testing

@testable import AWTransport

private let readNow = Date(timeIntervalSince1970: 1_735_689_600)

private func fixturePrivateIPv4Host() -> String {
  [192, 168, 1, 10].map(String.init).joined(separator: ".")
}

@Test("successful projection reads are live and cached by device and query")
func successfulProjectionRead() async throws {
  let cache = try ProjectionCache(maximumEntries: 2)
  let transport = MockProjectionTransport(page: page(value: 72))
  let service = ProjectionReadService(
    endpoint: "https://client.example.com",
    cache: cache,
    credentialStore: ReadCredentialStore(),
    transport: transport,
    now: { readNow }
  )

  let state = await service.read(
    query: ClientProjectionQuery(metric: "heart_rate", windowStart: "2026-01-01T00:00:00Z", windowEnd: nil, cursor: nil)
  )
  guard case .live(let result, let age) = state else {
    Issue.record("successful projection was not live")
    return
  }
  #expect(result.points.first?.value == 72)
  #expect(age == 0)
  #expect(transport.readCount == 1)
}

@Test("network failure with a cache returns stale data and a named reason")
func staleProjectionRead() async throws {
  let cache = try ProjectionCache()
  let transport = MockProjectionTransport(page: page(value: 72))
  let service = ProjectionReadService(
    endpoint: "https://client.example.com",
    cache: cache,
    credentialStore: ReadCredentialStore(),
    transport: transport,
    now: { readNow }
  )
  let query = ClientProjectionQuery(metric: "heart_rate", windowStart: nil, windowEnd: nil, cursor: nil)
  _ = await service.read(query: query, cacheLifetime: 60)
  transport.error = .network("Pi unreachable")

  let state = await service.read(query: query, cacheLifetime: 60)
  guard case .stale(let result, let age, let reason) = state else {
    Issue.record("cached projection was not marked stale")
    return
  }
  #expect(result.points.first?.value == 72)
  #expect(age == 0)
  #expect(reason == "Pi unreachable")
}

@Test("read failure without a cache is unknown rather than healthy or empty")
func unknownProjectionRead() async throws {
  let cache = try ProjectionCache()
  let service = ProjectionReadService(
    endpoint: "https://client.example.com",
    cache: cache,
    credentialStore: ReadCredentialStore(),
    transport: MockProjectionTransport(error: .server(code: "unavailable", retryable: true)),
    now: { readNow }
  )
  let state = await service.read(
    query: ClientProjectionQuery(metric: "heart_rate", windowStart: nil, windowEnd: nil, cursor: nil)
  )
  guard case .unknown(let reason) = state else {
    Issue.record("uncached projection failure was not unknown")
    return
  }
  #expect(reason.contains("server rejected request with unavailable"))
}

@Test("null projection points and opaque cursors are preserved")
func projectionGapsAndCursorArePreserved() async throws {
  let expected = ClientProjectionPage(
    apiVersion: "1.0",
    metric: "heart_rate",
    observedAt: nil,
    generatedAt: "2026-01-01T00:00:00Z",
    points: [ClientProjectionPoint(timestamp: "2026-01-01T00:00:00Z", value: nil)],
    missingIntervals: [["2026-01-01T00:00:00Z", "2026-01-01T00:05:00Z"]],
    nextCursor: "opaque-server-cursor"
  )
  let cache = try ProjectionCache()
  let transport = MockProjectionTransport(page: expected)
  let service = ProjectionReadService(
    endpoint: "https://client.example.com",
    cache: cache,
    credentialStore: ReadCredentialStore(),
    transport: transport,
    now: { readNow }
  )

  let state = await service.read(
    query: ClientProjectionQuery(metric: "heart_rate", windowStart: nil, windowEnd: nil, cursor: nil)
  )
  guard case .live(let page, _) = state else {
    Issue.record("projection with a null point was not live")
    return
  }
  #expect(page.points.first?.value == nil)
  #expect(page.missingIntervals.count == 1)
  #expect(page.nextCursor == "opaque-server-cursor")
}

@Test("incompatible response versions become unknown")
func incompatibleProjectionVersion() async throws {
  let cache = try ProjectionCache()
  let service = ProjectionReadService(
    endpoint: "https://client.example.com",
    cache: cache,
    credentialStore: ReadCredentialStore(),
    transport: MockProjectionTransport(page: page(value: 72, apiVersion: "2.0")),
    now: { readNow }
  )
  let state = await service.read(
    query: ClientProjectionQuery(metric: "heart_rate", windowStart: nil, windowEnd: nil, cursor: nil)
  )
  guard case .unknown(let reason) = state else {
    Issue.record("incompatible projection was not unknown")
    return
  }
  #expect(reason.contains("does not match client major 1"))
}

private func page(value: Double, apiVersion: String = "1.0") -> ClientProjectionPage {
  ClientProjectionPage(
    apiVersion: apiVersion,
    metric: "heart_rate",
    observedAt: "2026-01-01T00:00:00Z",
    generatedAt: "2026-01-01T00:00:01Z",
    points: [ClientProjectionPoint(timestamp: "2026-01-01T00:00:00Z", value: value)],
    missingIntervals: [],
    nextCursor: nil
  )
}

private final class ReadCredentialStore: DeviceCredentialStore, @unchecked Sendable {
  func load() throws -> StoredDeviceCredential? {
    try StoredDeviceCredential(
      origin: try ClientOrigin(string: "http://\(fixturePrivateIPv4Host()):8480"),
      clientID: UUID(uuidString: "00000000-0000-0000-0000-000000000001")!,
      keyID: UUID(uuidString: "00000000-0000-0000-0000-000000000002")!,
      keyReference: try DeviceKeyReference(reference: "fixture-key", publicKeySPKIDERBase64: "fixture-key"),
      scopes: ["read:projections"],
      issuedAt: "2026-01-01T00:00:00Z",
      expiresAt: "2026-06-30T00:00:00Z",
      serverTime: "2026-01-01T00:00:01Z"
    )
  }

  func save(_ credential: StoredDeviceCredential) throws {}
  func delete() throws {}
}

private final class MockProjectionTransport: ProjectionReading, @unchecked Sendable {
  var page: ClientProjectionPage?
  var error: TransportFailure?
  var readCount = 0

  init(page: ClientProjectionPage) { self.page = page }
  init(error: TransportFailure) { self.error = error }

  func read(
    query: ClientProjectionQuery,
    credential: StoredDeviceCredential
  ) async throws -> ClientProjectionPage {
    readCount += 1
    if let error { throw error }
    guard let page else { throw TransportFailure.network("test projection missing") }
    return page
  }
}
