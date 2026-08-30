import Foundation

/// Open wire values remain strings so a newer server value is preserved instead of defaulted.
public struct ClientErrorResponse: Codable, Equatable, Sendable {
  public let apiVersion: String
  public let code: String
  public let message: String
  public let retryable: Bool
  public let blocker: String
  public let requestedMajor: UInt?
  public let supportedMajor: UInt
  public let serverTime: String

  enum CodingKeys: String, CodingKey {
    case apiVersion = "api_version"
    case code, message, retryable, blocker
    case requestedMajor = "requested_major"
    case supportedMajor = "supported_major"
    case serverTime = "server_time"
  }
}

public struct ClientWriteBatch: Codable, Equatable, Sendable {
  public let batchID: UUID
  public let recordedAt: String
  public let items: [ClientWriteItem]

  enum CodingKeys: String, CodingKey {
    case batchID = "batch_id"
    case recordedAt = "recorded_at"
    case items
  }
}

public struct ClientWriteItem: Codable, Equatable, Sendable {
  public let id: UUID
  public let `class`: String
  public let schemaVersion: UInt
  public let content: JSONValue

  enum CodingKeys: String, CodingKey {
    case id, `class`, content
    case schemaVersion = "schema_version"
  }
}

public struct ClientWriteResponse: Codable, Equatable, Sendable {
  public let apiVersion: String
  public let batchID: UUID
  public let acknowledgements: [ItemAcknowledgement]
  public let serverTime: String

  enum CodingKeys: String, CodingKey {
    case apiVersion = "api_version"
    case batchID = "batch_id"
    case acknowledgements
    case serverTime = "server_time"
  }
}

public struct ItemAcknowledgement: Codable, Equatable, Sendable {
  public let itemID: UUID
  public let status: String
  public let code: String?
  public let message: String
  public let retryable: Bool

  enum CodingKeys: String, CodingKey {
    case itemID = "item_id"
    case status, code, message, retryable
  }
}

/// Minimal lossless JSON representation for forward-compatible payload fixtures.
public enum JSONValue: Codable, Equatable, Sendable {
  case null
  case bool(Bool)
  case number(Double)
  case string(String)
  case array([JSONValue])
  case object([String: JSONValue])

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
    } else if let value = try? container.decode([JSONValue].self) {
      self = .array(value)
    } else {
      self = .object(try container.decode([String: JSONValue].self))
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
