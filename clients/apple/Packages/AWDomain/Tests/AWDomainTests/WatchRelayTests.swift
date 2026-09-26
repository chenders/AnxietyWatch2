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

@Test("watch relay queue is bounded without dropping pending logs")
func watchRelayQueueRejectsOverflow() throws {
  var queue = WatchRelayQueue()
  for index in 0..<WatchRelayQueue.maximumItems {
    let id = UUID(uuidString: String(format: "00000000-0000-7000-8000-%012x", index + 1))!
    try queue.enqueue(
      WatchQuickLog(id: id, capturedAt: "2025-01-15T12:00:00Z", anxietyRawValue: 5)
    )
  }
  let overflowID = UUID(uuidString: "00000000-0000-7000-8000-00000000ffff")!
  let overflow = try WatchQuickLog(
    id: overflowID,
    capturedAt: "2025-01-15T12:00:00Z",
    anxietyRawValue: 5
  )

  #expect(throws: DomainCommandError.watchRelayQueueFull(WatchRelayQueue.maximumItems)) {
    try queue.enqueue(overflow)
  }
  #expect(queue.items.count == WatchRelayQueue.maximumItems)
}

@Test("watch relay rejects an oversized persisted queue")
func watchRelayQueueRejectsOversizedPersistence() throws {
  var items: [WatchQuickLog] = []
  for index in 0...WatchRelayQueue.maximumItems {
    let id = UUID(uuidString: String(format: "00000000-0000-7000-8000-%012x", index + 1))!
    items.append(
      try WatchQuickLog(id: id, capturedAt: "2025-01-15T12:00:00Z", anxietyRawValue: 5)
    )
  }
  let data = try JSONEncoder().encode(["items": items])
  #expect(throws: DomainCommandError.watchRelayQueueFull(WatchRelayQueue.maximumItems + 1)) {
    try JSONDecoder().decode(WatchRelayQueue.self, from: data)
  }
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
  if case .object(let content) = record.content {
    #expect(content["watch_relay"] == .bool(true))
  } else {
    Issue.record("watch Quick Log record content was not an object")
  }
}
