import Foundation

/// Errors raised before a domain command is handed to the durable outbox.
public enum DomainCommandError: Error, Equatable, Sendable {
  case invalidID
  case invalidKind
  case invalidSchemaVersion(UInt)
  case invalidRecordedAt(String)
  case invalidRating(UInt8)
  case mixedVersionAndTombstone
  case selfReference
}

/// The first client/server domain-record schema revision.
public let domainSchemaVersion: UInt = 1

private func isValidUTCTimestamp(_ value: String) -> Bool {
  let formatter = ISO8601DateFormatter()
  formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
  let date = formatter.date(from: value) ?? {
    formatter.formatOptions = [.withInternetDateTime]
    return formatter.date(from: value)
  }()
  guard let date else { return false }
  return value.hasSuffix("Z") && date.timeIntervalSince1970.isFinite
}

/// Lossless JSON used by domain commands without introducing a local database or archive.
public enum DomainJSONValue: Codable, Equatable, Sendable {
  case null
  case bool(Bool)
  case number(Double)
  case string(String)
  case array([DomainJSONValue])
  case object([String: DomainJSONValue])

  public init(from decoder: Decoder) throws {
    let container = try decoder.singleValueContainer()
    if container.decodeNil() {
      self = .null
    } else if let value = try? container.decode(Bool.self) {
      self = .bool(value)
    } else if let value = try? container.decode(Double.self) {
      self = .number(value)
    } else if let value = try? container.decode(String.self) {
      self = .string(value)
    } else if let value = try? container.decode([DomainJSONValue].self) {
      self = .array(value)
    } else {
      self = .object(try container.decode([String: DomainJSONValue].self))
    }
  }

  public func encode(to encoder: Encoder) throws {
    var container = encoder.singleValueContainer()
    switch self {
    case .null:
      try container.encodeNil()
    case .bool(let value):
      try container.encode(value)
    case .number(let value):
      try container.encode(value)
    case .string(let value):
      try container.encode(value)
    case .array(let value):
      try container.encode(value)
    case .object(let value):
      try container.encode(value)
    }
  }
}

/// The encrypted API payload wrapper for a single domain record.
public struct DomainEnvelope: Codable, Equatable, Sendable {
  public let `class`: String
  public let record: DomainRecordPayload

  public init(record: DomainRecordPayload) {
    self.`class` = "domain_record"
    self.record = record
  }
}

/// The result of comparing an incoming client command with an already-known ID.
public enum DomainRecordDisposition: Equatable, Sendable {
  case newRecord
  case replay
  case conflict
}

/// The wire envelope stored in an outbox item and sent to the Pi.
public struct DomainRecordPayload: Codable, Equatable, Sendable {
  public let id: UUID
  public let kind: String
  public let schemaVersion: UInt
  public let recordedAt: String
  public let supersedesID: UUID?
  public let tombstoneTargetID: UUID?
  public let content: DomainJSONValue

  /// Apply the server's same-ID rule without mutating either payload.
  public func disposition(comparedWith existing: Self?) -> DomainRecordDisposition {
    guard let existing else { return .newRecord }
    guard existing.id == id else { return .newRecord }
    let sameContent = existing.kind == kind
      && existing.schemaVersion == schemaVersion
      && existing.supersedesID == supersedesID
      && existing.tombstoneTargetID == tombstoneTargetID
      && existing.content == content
    return sameContent ? .replay : .conflict
  }

  public init(
    id: UUID,
    kind: String,
    schemaVersion: UInt = domainSchemaVersion,
    recordedAt: String,
    supersedesID: UUID? = nil,
    tombstoneTargetID: UUID? = nil,
    content: DomainJSONValue
  ) throws {
    guard id.uuidString != "00000000-0000-0000-0000-000000000000" else {
      throw DomainCommandError.invalidID
    }
    guard schemaVersion > 0 else {
      throw DomainCommandError.invalidSchemaVersion(schemaVersion)
    }
    guard !kind.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw DomainCommandError.invalidKind
    }
    guard Self.isUTCDate(recordedAt) else {
      throw DomainCommandError.invalidRecordedAt(recordedAt)
    }
    guard supersedesID != id, tombstoneTargetID != id else {
      throw DomainCommandError.selfReference
    }
    guard supersedesID == nil || tombstoneTargetID == nil else {
      throw DomainCommandError.mixedVersionAndTombstone
    }
    self.id = id
    self.kind = kind
    self.schemaVersion = schemaVersion
    self.recordedAt = recordedAt
    self.supersedesID = supersedesID
    self.tombstoneTargetID = tombstoneTargetID
    self.content = content
  }

  private static func isUTCDate(_ value: String) -> Bool {
    isValidUTCTimestamp(value)
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    try self.init(
      id: container.decode(UUID.self, forKey: .id),
      kind: container.decode(String.self, forKey: .kind),
      schemaVersion: container.decode(UInt.self, forKey: .schemaVersion),
      recordedAt: container.decode(String.self, forKey: .recordedAt),
      supersedesID: container.decodeIfPresent(UUID.self, forKey: .supersedesID),
      tombstoneTargetID: container.decodeIfPresent(UUID.self, forKey: .tombstoneTargetID),
      content: container.decode(DomainJSONValue.self, forKey: .content)
    )
  }

  enum CodingKeys: String, CodingKey {
    case id, kind, content
    case schemaVersion = "schema_version"
    case recordedAt = "recorded_at"
    case supersedesID = "supersedes_id"
    case tombstoneTargetID = "tombstone_target_id"
  }
}

/// A journal/anxiety command. Capture time is assigned by `capture`, not by a user-entered date.
public struct JournalCommand: Codable, Equatable, Sendable {
  public let id: UUID
  public let subjectAt: String
  public let recordedAt: String
  public let anxiety: AnxietyRating
  public let note: String?
  public let tags: [String]
  public let songID: UUID?
  public let expressMode: Bool

  public init(
    id: UUID,
    subjectAt: String,
    recordedAt: String,
    anxiety: AnxietyRating,
    note: String?,
    tags: [String],
    songID: UUID?,
    expressMode: Bool
  ) throws {
    guard isValidUTCTimestamp(subjectAt) else {
      throw DomainCommandError.invalidRecordedAt(subjectAt)
    }
    guard isValidUTCTimestamp(recordedAt) else {
      throw DomainCommandError.invalidRecordedAt(recordedAt)
    }
    self.id = id
    self.subjectAt = subjectAt
    self.recordedAt = recordedAt
    self.anxiety = anxiety
    self.note = note
    self.tags = tags
    self.songID = songID
    self.expressMode = expressMode
  }

  public static func capture(
    id: UUID = UUID(),
    capturedAt: Date,
    anxietyRawValue: UInt8,
    note: String? = nil,
    tags: [String] = [],
    songID: UUID? = nil,
    expressMode: Bool = false
  ) throws -> Self {
    guard let anxiety = AnxietyRating(rawValue: anxietyRawValue) else {
      throw DomainCommandError.invalidRating(anxietyRawValue)
    }
    return try Self(
      id: id,
      subjectAt: Self.utcString(capturedAt),
      recordedAt: Self.utcString(capturedAt),
      anxiety: anxiety,
      note: note,
      tags: tags,
      songID: songID,
      expressMode: expressMode
    )
  }

  public func record() throws -> DomainRecordPayload {
    var object: [String: DomainJSONValue] = [
      "anxiety": .number(Double(anxiety.rawValue)),
      "express_mode": .bool(expressMode),
      "tags": .array(tags.map(DomainJSONValue.string))
    ]
    object["note"] = note.map(DomainJSONValue.string) ?? .null
    object["song_id"] = songID.map { .string($0.uuidString) } ?? .null
    object["subject_at"] = .string(subjectAt)
    return try DomainRecordPayload(
      id: id,
      kind: "journal_entry",
      recordedAt: recordedAt,
      content: .object(object)
    )
  }

  private static func utcString(_ date: Date) -> String {
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    return formatter.string(from: date)
  }
}

/// An offline-capable song reference or free-form song stub.
public struct SongCommand: Codable, Equatable, Sendable {
  public let id: UUID
  public let subjectAt: String
  public let recordedAt: String
  public let title: String
  public let artist: String
  public let album: String?
  public let provider: String?
  public let providerID: String?

  public init(
    id: UUID = UUID(),
    subjectAt: String? = nil,
    recordedAt: String,
    title: String,
    artist: String,
    album: String? = nil,
    provider: String? = nil,
    providerID: String? = nil
  ) throws {
    guard !title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
          !artist.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw DomainCommandError.invalidKind
    }
    guard isValidUTCTimestamp(recordedAt) else {
      throw DomainCommandError.invalidRecordedAt(recordedAt)
    }
    if let subjectAt, !isValidUTCTimestamp(subjectAt) {
      throw DomainCommandError.invalidRecordedAt(subjectAt)
    }
    self.id = id
    self.subjectAt = subjectAt ?? recordedAt
    self.recordedAt = recordedAt
    self.title = title
    self.artist = artist
    self.album = album
    self.provider = provider
    self.providerID = providerID
  }

  public func record() throws -> DomainRecordPayload {
    try DomainRecordPayload(
      id: id,
      kind: "song",
      recordedAt: recordedAt,
      content: .object([
        "subject_at": .string(subjectAt),
        "title": .string(title),
        "artist": .string(artist),
        "album": album.map(DomainJSONValue.string) ?? .null,
        "provider": provider.map(DomainJSONValue.string) ?? .null,
        "provider_id": providerID.map(DomainJSONValue.string) ?? .null,
        "lyrics": .null,
        "lyrics_source": .string("unavailable")
      ])
    )
  }
}

/// A medication dose event linked to a server-owned medication definition.
public struct MedicationDoseCommand: Codable, Equatable, Sendable {
  public let id: UUID
  public let subjectAt: String
  public let recordedAt: String
  public let medicationID: UUID
  public let doseDescription: String?

  public init(
    id: UUID = UUID(),
    subjectAt: String? = nil,
    recordedAt: String,
    medicationID: UUID,
    doseDescription: String? = nil
  ) throws {
    guard isValidUTCTimestamp(recordedAt) else {
      throw DomainCommandError.invalidRecordedAt(recordedAt)
    }
    if let subjectAt, !isValidUTCTimestamp(subjectAt) {
      throw DomainCommandError.invalidRecordedAt(subjectAt)
    }
    self.id = id
    self.subjectAt = subjectAt ?? recordedAt
    self.recordedAt = recordedAt
    self.medicationID = medicationID
    self.doseDescription = doseDescription
  }

  public func record() throws -> DomainRecordPayload {
    try DomainRecordPayload(
      id: id,
      kind: "medication_dose",
      recordedAt: recordedAt,
      content: .object([
        "subject_at": .string(subjectAt),
        "medication_id": .string(medicationID.uuidString),
        "dose_description": doseDescription.map(DomainJSONValue.string) ?? .null
      ])
    )
  }
}

/// A check-in response event; delivery and unanswered states remain explicit strings.
public struct CheckInCommand: Codable, Equatable, Sendable {
  public let id: UUID
  public let subjectAt: String
  public let recordedAt: String
  public let promptID: UUID
  public let outcome: String
  public let anxiety: AnxietyRating?

  public init(
    id: UUID = UUID(),
    subjectAt: String? = nil,
    recordedAt: String,
    promptID: UUID,
    outcome: String,
    anxietyRawValue: UInt8?
  ) throws {
    guard isValidUTCTimestamp(recordedAt) else {
      throw DomainCommandError.invalidRecordedAt(recordedAt)
    }
    if let subjectAt, !isValidUTCTimestamp(subjectAt) {
      throw DomainCommandError.invalidRecordedAt(subjectAt)
    }
    guard !outcome.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw DomainCommandError.invalidKind
    }
    if let anxietyRawValue, AnxietyRating(rawValue: anxietyRawValue) == nil {
      throw DomainCommandError.invalidRating(anxietyRawValue)
    }
    self.id = id
    self.subjectAt = subjectAt ?? recordedAt
    self.recordedAt = recordedAt
    self.promptID = promptID
    self.outcome = outcome
    self.anxiety = anxietyRawValue.flatMap(AnxietyRating.init(rawValue:))
  }

  public func record() throws -> DomainRecordPayload {
    try DomainRecordPayload(
      id: id,
      kind: "check_in",
      recordedAt: recordedAt,
      content: .object([
        "subject_at": .string(subjectAt),
        "prompt_id": .string(promptID.uuidString),
        "outcome": .string(outcome),
        "anxiety": anxiety.map { .number(Double($0.rawValue)) } ?? .null
      ])
    )
  }
}

/// An immutable edit that creates a new client identity and supersedes one prior record.
public struct SupersedingCommand: Codable, Equatable, Sendable {
  public let id: UUID
  public let recordedAt: String
  public let kind: String
  public let supersedesID: UUID
  public let content: DomainJSONValue

  public init(
    id: UUID = UUID(),
    recordedAt: String,
    kind: String,
    supersedesID: UUID,
    content: DomainJSONValue
  ) throws {
    guard id != supersedesID else { throw DomainCommandError.selfReference }
    self.id = id
    self.recordedAt = recordedAt
    self.kind = kind
    self.supersedesID = supersedesID
    self.content = content
  }

  public func record() throws -> DomainRecordPayload {
    try DomainRecordPayload(
      id: id,
      kind: kind,
      recordedAt: recordedAt,
      supersedesID: supersedesID,
      content: content
    )
  }
}

/// An append-only deletion request. It never removes the target locally or remotely.
public struct TombstoneCommand: Codable, Equatable, Sendable {
  public let id: UUID
  public let recordedAt: String
  public let targetID: UUID
  public let reason: String?

  public init(id: UUID = UUID(), recordedAt: String, targetID: UUID, reason: String? = nil) throws {
    guard id != targetID else { throw DomainCommandError.selfReference }
    self.id = id
    self.recordedAt = recordedAt
    self.targetID = targetID
    self.reason = reason
  }

  public func record() throws -> DomainRecordPayload {
    try DomainRecordPayload(
      id: id,
      kind: "tombstone",
      recordedAt: recordedAt,
      tombstoneTargetID: targetID,
      content: .object(["reason": reason.map(DomainJSONValue.string) ?? .null])
    )
  }
}

/// Names used by feature code and contract tests for the typed commands.
public typealias JournalEntryCommand = JournalCommand
public typealias SongReferenceCommand = SongCommand
public typealias DoseCommand = MedicationDoseCommand
public typealias RandomCheckInCommand = CheckInCommand

/// All first-draft subjective commands supported by the domain contract.
public enum DomainCommand: Codable, Equatable, Sendable {
  case journal(JournalCommand)
  case song(SongCommand)
  case medicationDose(MedicationDoseCommand)
  case checkIn(CheckInCommand)
  case superseding(SupersedingCommand)
  case tombstone(TombstoneCommand)

  /// Convert the typed command into the immutable server envelope.
  public func record() throws -> DomainRecordPayload {
    switch self {
    case .journal(let command): return try command.record()
    case .song(let command): return try command.record()
    case .medicationDose(let command): return try command.record()
    case .checkIn(let command): return try command.record()
    case .superseding(let command): return try command.record()
    case .tombstone(let command): return try command.record()
    }
  }
}
