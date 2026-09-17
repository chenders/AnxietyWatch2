import Foundation

/// A watch-created severity event. The watch sends only this bounded, immutable payload to iPhone.
public struct WatchQuickLog: Codable, Equatable, Identifiable, Sendable {
  public let id: UUID
  public let capturedAt: String
  public let anxiety: AnxietyRating

  public init(id: UUID = UUID(), capturedAt: String, anxietyRawValue: UInt8) throws {
    guard let anxiety = AnxietyRating(rawValue: anxietyRawValue) else {
      throw DomainCommandError.invalidRating(anxietyRawValue)
    }
    guard isValidUTCTimestamp(capturedAt) else {
      throw DomainCommandError.invalidRecordedAt(capturedAt)
    }
    self.id = id
    self.capturedAt = capturedAt
    self.anxiety = anxiety
  }

  public static func capture(
    id: UUID = UUID(),
    capturedAt: Date,
    anxietyRawValue: UInt8
  ) throws -> Self {
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    return try Self(
      id: id,
      capturedAt: formatter.string(from: capturedAt),
      anxietyRawValue: anxietyRawValue
    )
  }

  public func record() throws -> DomainRecordPayload {
    try DomainRecordPayload(
      id: id,
      kind: "journal_entry",
      recordedAt: capturedAt,
      content: .object([
        "anxiety": .number(Double(anxiety.rawValue)),
        "express_mode": .bool(true),
        "note": .null,
        "song_id": .null,
        "subject_at": .string(capturedAt),
        "tags": .array([]),
        "watch_relay": .bool(true)
      ])
    )
  }
}

/// Pure queue behavior used by the watch persistence adapter and its tests.
public struct WatchRelayQueue: Codable, Equatable, Sendable {
  public static let maximumItems = 100
  public private(set) var items: [WatchQuickLog]

  public init(items: [WatchQuickLog] = []) {
    self.items = items
  }

  private enum CodingKeys: String, CodingKey {
    case items
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    let decoded = try container.decode([WatchQuickLog].self, forKey: .items)
    guard decoded.count <= Self.maximumItems else {
      throw DomainCommandError.watchRelayQueueFull(decoded.count)
    }
    items = decoded
  }

  /// Re-delivery of the same watch ID is a no-op; a different payload is a conflict.
  public mutating func enqueue(_ item: WatchQuickLog) throws {
    guard let existing = items.first(where: { $0.id == item.id }) else {
      guard items.count < Self.maximumItems else {
        throw DomainCommandError.watchRelayQueueFull(Self.maximumItems)
      }
      items.append(item)
      return
    }
    guard existing == item else { throw DomainCommandError.watchRelayConflict }
  }

  public mutating func acknowledge(id: UUID) {
    items.removeAll { $0.id == id }
  }
}
