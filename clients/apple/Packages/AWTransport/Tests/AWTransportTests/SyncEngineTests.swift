import AWDomain
import AWOutbox
import Foundation
import Testing

@testable import AWTransport

private let syncNow = Date(timeIntervalSince1970: 1_735_689_600)

private func fixturePrivateIPv4Host() -> String {
  [192, 168, 1, 10].map(String.init).joined(separator: ".")
}

@Test("sync acknowledges created and replayed items and removes their payloads")
func syncAcknowledgesItems() async throws {
  let directory = try temporaryDirectory()
  let outbox = try FileOutboxStore(directory: directory)
  let firstID = UUID()
  let secondID = UUID()
  try await outbox.enqueue(try item(id: firstID, payload: Data("{}".utf8)))
  try await outbox.enqueue(try item(id: secondID, payload: Data("{\"rating\":6}".utf8)))
  let credentialStore = MockCredentialStore()
  let transport = MockItemTransport(response: ClientWriteResponse(
    apiVersion: "1.0",
    batchID: UUID(),
    acknowledgements: [
      ItemAcknowledgement(itemID: firstID, status: "created", code: "new", message: "created", retryable: false),
      ItemAcknowledgement(itemID: secondID, status: "replayed", code: "same_content", message: "replayed", retryable: false),
    ],
    serverTime: "2026-01-01T00:00:01Z"
  ))
  let stateSink = MockSyncStateSink()
  let engine = SyncEngine(
    outbox: outbox,
    credentialStore: credentialStore,
    transport: transport,
    stateSink: stateSink,
    configuration: try SyncConfiguration(jitter: { _ in 0 }),
    now: { syncNow }
  )

  let report = try await engine.drain()
  #expect(report.attempted == 2)
  #expect(report.acknowledged == 2)
  #expect(report.replayed == 1)
  #expect(report.pendingAfter == 0)
  #expect(transport.sent?.items.count == 2)
  #expect(await stateSink.state(for: firstID) == .acknowledged)
  #expect(await stateSink.state(for: secondID) == .acknowledged)
}

@Test("retryable transport failures preserve payloads and use bounded exponential backoff")
func retryableTransportFailure() async throws {
  let directory = try temporaryDirectory()
  let outbox = try FileOutboxStore(directory: directory)
  let id = UUID()
  try await outbox.enqueue(try item(id: id, payload: Data("{}".utf8)))
  let transport = MockItemTransport(error: .http(status: 503, retryAfter: nil))
  let stateSink = MockSyncStateSink()
  let engine = SyncEngine(
    outbox: outbox,
    credentialStore: MockCredentialStore(),
    transport: transport,
    stateSink: stateSink,
    configuration: try SyncConfiguration(jitter: { _ in 0 }),
    now: { syncNow }
  )

  let report = try await engine.drain()
  let pending = try await outbox.pendingItems(limit: 1)
  let stored = try #require(pending.first)
  #expect(report.retryable == 1)
  #expect(stored.payload == Data("{}".utf8))
  #expect(stored.attemptCount == 1)
  #expect(stored.nextEligibleAttemptAt == syncNow.addingTimeInterval(1))
  #expect(await stateSink.state(for: id) == .queued)
}

@Test("retry-after is honored without exceeding the configured ceiling")
func retryAfterIsBounded() async throws {
  let directory = try temporaryDirectory()
  let outbox = try FileOutboxStore(directory: directory)
  try await outbox.enqueue(try item(id: UUID(), payload: Data("{}".utf8)))
  let transport = MockItemTransport(error: .http(status: 429, retryAfter: 9999))
  let engine = SyncEngine(
    outbox: outbox,
    credentialStore: MockCredentialStore(),
    transport: transport,
    configuration: try SyncConfiguration(maximumRetryAfter: 30, jitter: { _ in 0 }),
    now: { syncNow }
  )

  _ = try await engine.drain()
  let pending = try await outbox.pendingItems(limit: 1)
  #expect(pending.first?.nextEligibleAttemptAt == syncNow.addingTimeInterval(30))
}

@Test("non-retryable credential failures preserve the item and stop automatic retry")
func credentialFailurePreservesItem() async throws {
  let directory = try temporaryDirectory()
  let outbox = try FileOutboxStore(directory: directory)
  let id = UUID()
  try await outbox.enqueue(try item(id: id, payload: Data("{}".utf8)))
  let transport = MockItemTransport(error: .credentialRevoked("client c1"))
  let engine = SyncEngine(
    outbox: outbox,
    credentialStore: MockCredentialStore(),
    transport: transport,
    now: { syncNow }
  )

  do {
    _ = try await engine.drain()
    Issue.record("revoked credential unexpectedly allowed sync")
  } catch let error as SyncError {
    #expect(error == .blocked(.credentialRevoked("client c1")))
  }
  let allItems = try await outbox.allItems()
  let item = try #require(allItems.first)
  #expect(item.state == .rejected)
  #expect(item.payload == Data("{}".utf8))
}

@Test("missing credentials are distinct from a transport failure")
func missingCredentialIsUnknown() async throws {
  let directory = try temporaryDirectory()
  let outbox = try FileOutboxStore(directory: directory)
  let engine = SyncEngine(
    outbox: outbox,
    credentialStore: EmptyCredentialStore(),
    transport: MockItemTransport(response: emptyResponse()),
    now: { syncNow }
  )

  do {
    _ = try await engine.drain()
    Issue.record("sync unexpectedly ran without a credential")
  } catch let error as SyncError {
    #expect(error == .credentialUnavailable("no enrolled device credential is stored"))
  }
}

private func item(id: UUID, payload: Data) throws -> OutboxItem {
  try OutboxItem(
    id: id,
    recordClass: "journal",
    schemaVersion: 1,
    payload: payload,
    capturedAt: syncNow
  )
}

private func emptyResponse() -> ClientWriteResponse {
  ClientWriteResponse(
    apiVersion: "1.0",
    batchID: UUID(),
    acknowledgements: [],
    serverTime: "2026-01-01T00:00:01Z"
  )
}

private func temporaryDirectory() throws -> URL {
  let directory = FileManager.default.temporaryDirectory
    .appendingPathComponent("aw-sync-\(UUID().uuidString)", isDirectory: true)
  try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
  return directory
}

private final class MockCredentialStore: DeviceCredentialStore, @unchecked Sendable {
  func load() throws -> StoredDeviceCredential? {
    try StoredDeviceCredential(
      origin: try ClientOrigin(string: "http://\(fixturePrivateIPv4Host()):8480"),
      clientID: UUID(uuidString: "00000000-0000-0000-0000-000000000001")!,
      keyID: UUID(uuidString: "00000000-0000-0000-0000-000000000002")!,
      keyReference: try DeviceKeyReference(reference: "fixture-key", publicKeySPKIDERBase64: "fixture-key"),
      scopes: ["submit:domain_records"],
      issuedAt: "2026-01-01T00:00:00Z",
      expiresAt: "2026-06-30T00:00:00Z",
      serverTime: "2026-01-01T00:00:01Z"
    )
  }

  func save(_ credential: StoredDeviceCredential) throws {}
  func delete() throws {}
}

private final class EmptyCredentialStore: DeviceCredentialStore, @unchecked Sendable {
  func load() throws -> StoredDeviceCredential? { nil }
  func save(_ credential: StoredDeviceCredential) throws {}
  func delete() throws {}
}

private actor MockSyncStateSink: DomainSyncStateSink {
  private var states: [UUID: DomainSyncState] = [:]

  func update(recordID: UUID, state: DomainSyncState) async throws {
    states[recordID] = state
  }

  func state(for recordID: UUID) -> DomainSyncState? {
    states[recordID]
  }
}

private final class MockItemTransport: ClientItemTransport, @unchecked Sendable {
  let response: ClientWriteResponse?
  let error: TransportFailure?
  var sent: ClientWriteBatch?

  init(response: ClientWriteResponse) {
    self.response = response
    self.error = nil
  }

  init(error: TransportFailure) {
    self.response = nil
    self.error = error
  }

  func send(
    batch: ClientWriteBatch,
    credential: StoredDeviceCredential
  ) async throws -> ClientWriteResponse {
    sent = batch
    if let error { throw error }
    guard let response else { throw TransportFailure.network("test transport response missing") }
    return response
  }
}
