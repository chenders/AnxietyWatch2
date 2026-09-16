import Foundation
import Testing

@testable import AWOutbox

private let fixedDate = Date(timeIntervalSince1970: 1_735_689_600)

@Test("capture is durable and pending snapshot exposes count and oldest age")
func captureAndSnapshot() async throws {
  let directory = try temporaryDirectory()
  let store = try FileOutboxStore(directory: directory)
  let item = try OutboxItem(
    id: UUID(uuidString: "00000000-0000-0000-0000-000000000001")!,
    recordClass: "journal",
    schemaVersion: 1,
    payload: Data("private journal text".utf8),
    capturedAt: fixedDate
  )

  try await store.enqueue(item)
  let snapshot = try await store.snapshot()
  #expect(snapshot.pendingCount == 1)
  #expect(snapshot.oldestPendingAt == fixedDate)

  let reloaded = try FileOutboxStore(directory: directory)
  let pending = try await reloaded.pendingItems(limit: 10)
  #expect(pending == [item])
}

@Test("retry retains immutable payload and records only classified failure metadata")
func retryRetainsPayload() async throws {
  let directory = try temporaryDirectory()
  let store = try FileOutboxStore(directory: directory)
  let id = UUID()
  let payload = Data("journal content".utf8)
  try await store.enqueue(
    OutboxItem(id: id, recordClass: "journal", schemaVersion: 1, payload: payload, capturedAt: fixedDate)
  )
  let failure = try OutboxFailure(code: "timeout", reason: "Pi did not respond", occurredAt: fixedDate)
  try await store.recordRetry(itemID: id, failure: failure, nextAttemptAt: fixedDate.addingTimeInterval(30))

  let pending = try await store.pendingItems(limit: 1)
  let item = try #require(pending.first)
  #expect(item.payload == payload)
  #expect(item.state == .retryable)
  #expect(item.attemptCount == 1)
  #expect(item.lastFailure == failure)
}

@Test("terminal acknowledgement removes payload but keeps bounded receipt metadata")
func acknowledgementDeletesPayload() async throws {
  let directory = try temporaryDirectory()
  let store = try FileOutboxStore(directory: directory)
  let id = UUID()
  try await store.enqueue(
    OutboxItem(id: id, recordClass: "journal", schemaVersion: 1, payload: Data([1, 2, 3]), capturedAt: fixedDate)
  )
  try await store.recordAcknowledgement(
    itemID: id,
    acknowledgedAt: fixedDate.addingTimeInterval(1),
    serverAttemptID: "attempt-1"
  )

  let snapshot = try await store.snapshot()
  #expect(snapshot.pendingCount == 0)
  #expect(snapshot.receiptCount == 1)
  let pending = try await store.pendingItems(limit: 1)
  #expect(pending.isEmpty)
}

@Test("rejected and conflict items remain until explicit discard")
func nonAcknowledgedTerminalItemsRemain() async throws {
  let directory = try temporaryDirectory()
  let store = try FileOutboxStore(directory: directory)
  let rejectedID = UUID()
  let conflictID = UUID()
  try await store.enqueue(
    OutboxItem(id: rejectedID, recordClass: "journal", schemaVersion: 1, payload: Data([1]), capturedAt: fixedDate)
  )
  try await store.enqueue(
    OutboxItem(id: conflictID, recordClass: "journal", schemaVersion: 1, payload: Data([2]), capturedAt: fixedDate)
  )
  let rejected = try OutboxFailure(code: "invalid", reason: "named validation blocker", occurredAt: fixedDate)
  let conflict = try OutboxFailure(code: "conflict", reason: "same ID has different content", occurredAt: fixedDate)
  try await store.recordRejected(itemID: rejectedID, failure: rejected)
  try await store.recordConflict(itemID: conflictID, failure: conflict)

  let terminalSnapshot = try await store.snapshot()
  #expect(terminalSnapshot.pendingCount == 2)
  let terminalItems = try await store.pendingItems(limit: 10)
  #expect(terminalItems.isEmpty)
  #expect((try await store.allItems()).count == 2)
  try await store.discard(itemID: rejectedID, confirmed: true)
  try await store.discard(itemID: conflictID, confirmed: true)
}

@Test("reset names unsent loss and requires explicit confirmation")
func resetRequiresConfirmation() async throws {
  let directory = try temporaryDirectory()
  let store = try FileOutboxStore(directory: directory)
  try await store.enqueue(
    OutboxItem(id: UUID(), recordClass: "journal", schemaVersion: 1, payload: Data([1]), capturedAt: fixedDate)
  )

  do {
    try await store.reset(allowLoss: false)
    Issue.record("reset unexpectedly discarded an unsent item")
  } catch let error as OutboxError {
    guard case .resetBlocked(let message) = error else {
      Issue.record("unexpected reset error \(error)")
      return
    }
    #expect(message.contains("1 unacknowledged items"))
  }
  try await store.reset(allowLoss: true)
  let resetSnapshot = try await store.snapshot()
  #expect(resetSnapshot.pendingCount == 0)
}

@Test("only one drain owner can run at a time")
func drainOwnership() async throws {
  let directory = try temporaryDirectory()
  let store = try FileOutboxStore(directory: directory)
  try await store.acquireDrainOwnership()
  do {
    try await store.acquireDrainOwnership()
    Issue.record("second drain owner unexpectedly acquired ownership")
  } catch let error as OutboxError {
    guard case .drainAlreadyOwned(let message) = error else {
      Issue.record("unexpected ownership error \(error)")
      return
    }
    #expect(message.contains("active owner"))
  }
  await store.releaseDrainOwnership()
  try await store.acquireDrainOwnership()
}

private func temporaryDirectory() throws -> URL {
  let directory = FileManager.default.temporaryDirectory
    .appendingPathComponent("aw-outbox-\(UUID().uuidString)", isDirectory: true)
  try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
  return directory
}
