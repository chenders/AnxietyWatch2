import Foundation
import Testing

@testable import AWDomain

@Test("watch relay queue preserves identity and makes duplicate delivery a no-op")
func watchRelayQueueDeduplicates() throws {
  let id = UUID(uuidString: "00000000-0000-7000-8000-000000000301")!
  let item = try WatchQuickLog(id: id, capturedAt: "2025-01-15T12:00:00Z", anxietyRawValue: 7)
  var queue = WatchRelayQueue()

  try queue.enqueue(item)
  try queue.enqueue(item)

  #expect(queue.items == [item])
  queue.acknowledge(id: id)
  #expect(queue.items.isEmpty)
}

@Test("watch relay rejects a same-ID payload conflict")
func watchRelayQueueRejectsConflict() throws {
  let id = UUID(uuidString: "00000000-0000-7000-8000-000000000302")!
  let first = try WatchQuickLog(id: id, capturedAt: "2025-01-15T12:00:00Z", anxietyRawValue: 4)
  let second = try WatchQuickLog(id: id, capturedAt: "2025-01-15T12:00:00Z", anxietyRawValue: 5)
  var queue = WatchRelayQueue()
  try queue.enqueue(first)

  #expect(throws: DomainCommandError.watchRelayConflict) {
    try queue.enqueue(second)
  }
  #expect(queue.items == [first])
}

@Test("watch quick log produces an express journal record")
func watchQuickLogProducesJournalRecord() throws {
  let item = try WatchQuickLog(
    id: UUID(uuidString: "00000000-0000-7000-8000-000000000303")!,
    capturedAt: "2025-01-15T12:00:00Z",
    anxietyRawValue: 9
  )
  let record = try item.record()

  #expect(record.kind == "journal_entry")
  #expect(record.recordedAt == item.capturedAt)
  #expect(record.content != .null)
}
