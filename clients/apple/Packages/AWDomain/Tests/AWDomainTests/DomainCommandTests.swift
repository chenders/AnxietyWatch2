import Foundation
import Testing

@testable import AWDomain

@Test("journal capture preserves rating, optional absence, and fixed UTC capture time")
func journalCommandBuildsImmutableRecord() throws {
  let id = UUID(uuidString: "00000000-0000-7000-8000-000000000101")!
  let capturedAt = Date(timeIntervalSince1970: 1_736_942_400)
  let command = try JournalCommand.capture(
    id: id,
    capturedAt: capturedAt,
    anxietyRawValue: 7,
    note: nil,
    tags: ["meeting"],
    expressMode: true
  )
  let record = try command.record()

  #expect(record.id == id)
  #expect(record.kind == "journal_entry")
  #expect(record.schemaVersion == domainSchemaVersion)
  #expect(record.recordedAt == "2025-01-15T12:00:00.000Z")
  guard case .object(let content) = record.content else {
    Issue.record("journal content must be an object")
    return
  }
  #expect(content["subject_at"] == .string("2025-01-15T12:00:00.000Z"))
  #expect(content["anxiety"] == .number(7))
  #expect(content["note"] == .null)
  #expect(content["express_mode"] == .bool(true))
}

@Test("invalid anxiety values are rejected rather than clamped")
func journalCommandRejectsInvalidRating() {
  do {
    _ = try JournalCommand.capture(
      id: UUID(uuidString: "00000000-0000-7000-8000-000000000102")!,
      capturedAt: Date(timeIntervalSince1970: 1_736_942_400),
      anxietyRawValue: 11
    )
    Issue.record("invalid rating was accepted")
  } catch let error as DomainCommandError {
    #expect(error == .invalidRating(11))
  } catch {
    Issue.record("unexpected error: \\(error)")
  }
}

@Test("superseding and tombstone commands preserve separate identities")
func immutableEditsAndDeletionAreExplicit() throws {
  let original = UUID(uuidString: "00000000-0000-7000-8000-000000000103")!
  let edit = UUID(uuidString: "00000000-0000-7000-8000-000000000104")!
  let deletion = UUID(uuidString: "00000000-0000-7000-8000-000000000105")!
  let time = "2025-01-15T12:00:00Z"

  let version = try SupersedingCommand(
    id: edit,
    recordedAt: time,
    kind: "journal_entry",
    supersedesID: original,
    content: .object(["anxiety": .number(8)])
  )
  let tombstone = try TombstoneCommand(
    id: deletion,
    recordedAt: time,
    targetID: original,
    reason: "duplicate entry"
  )

  #expect(try version.record().supersedesID == original)
  #expect(try version.record().tombstoneTargetID == nil)
  #expect(try tombstone.record().tombstoneTargetID == original)
  #expect(try tombstone.record().supersedesID == nil)
}

@Test("record metadata rejects nil and non-UTC timestamps")
func recordMetadataRejectsUnsafeIdentityAndTime() {
  let nilID = UUID(uuidString: "00000000-0000-0000-0000-000000000000")!
  do {
    _ = try DomainRecordPayload(
      id: nilID,
      kind: "journal_entry",
      recordedAt: "2025-01-15T12:00:00+02:00",
      content: .object([:])
    )
    Issue.record("nil UUID was accepted")
  } catch let error as DomainCommandError {
    #expect(error == .invalidID)
  } catch {
    Issue.record("unexpected error: \\(error)")
  }

  let validID = UUID(uuidString: "00000000-0000-7000-8000-000000000106")!
  do {
    _ = try DomainRecordPayload(
      id: validID,
      kind: "journal_entry",
      recordedAt: "2025-01-15T12:00:00+02:00",
      content: .object([:])
    )
    Issue.record("non-UTC timestamp was accepted")
  } catch let error as DomainCommandError {
    #expect(error == .invalidRecordedAt("2025-01-15T12:00:00+02:00"))
  } catch {
    Issue.record("unexpected error: \\(error)")
  }
}

@Test("same IDs replay only when immutable content is identical")
func sameIDReplayAndConflictAreDistinct() throws {
  let id = UUID(uuidString: "00000000-0000-7000-8000-000000000109")!
  let first = try DomainRecordPayload(
    id: id,
    kind: "journal_entry",
    recordedAt: "2025-01-15T12:00:00Z",
    content: .object(["anxiety": .number(6)])
  )
  let replay = try DomainRecordPayload(
    id: id,
    kind: "journal_entry",
    recordedAt: "2025-01-15T12:01:00Z",
    content: .object(["anxiety": .number(6)])
  )
  let conflict = try DomainRecordPayload(
    id: id,
    kind: "journal_entry",
    recordedAt: "2025-01-15T12:01:00Z",
    content: .object(["anxiety": .number(8)])
  )

  #expect(first.disposition(comparedWith: nil) == .newRecord)
  #expect(replay.disposition(comparedWith: first) == .replay)
  #expect(conflict.disposition(comparedWith: first) == .conflict)
}

@Test("domain record payload round trips its snake case wire keys")
func domainRecordPayloadRoundTrips() throws {
  let id = UUID(uuidString: "00000000-0000-7000-8000-000000000107")!
  let original = try DomainRecordPayload(
    id: id,
    kind: "journal_entry",
    recordedAt: "2025-01-15T12:00:00Z",
    supersedesID: UUID(uuidString: "00000000-0000-7000-8000-000000000108"),
    content: .object(["anxiety": .number(6)])
  )
  let data = try JSONEncoder().encode(original)
  let decoded = try JSONDecoder().decode(DomainRecordPayload.self, from: data)
  #expect(decoded == original)
  guard let wire = String(bytes: data, encoding: .utf8) else {
    Issue.record("encoded payload was not UTF-8")
    return
  }
  #expect(wire.contains("schema_version"))
  #expect(wire.contains("supersedes_id"))

  let envelope = try JSONDecoder().decode(
    DomainEnvelope.self,
    from: JSONEncoder().encode(DomainEnvelope(record: original))
  )
  #expect(envelope.`class` == "domain_record")
  #expect(envelope.record == original)
}
