import Foundation

public struct EnrollmentClaimRequest: Codable, Equatable, Sendable {
  public let apiMajor: UInt
  public let enrollmentID: UUID
  public let token: String
  public let serverInstanceID: UUID
  public let deviceLabel: String
  public let publicKeySPKIDERBase64: String
  public let algorithm: String
  public let appVersion: String

  enum CodingKeys: String, CodingKey {
    case apiMajor = "api_major"
    case enrollmentID = "enrollment_id"
    case token
    case serverInstanceID = "server_instance_id"
    case deviceLabel = "device_label"
    case publicKeySPKIDERBase64 = "public_key_spki_der_b64"
    case algorithm
    case appVersion = "app_version"
  }
}

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

  public init(batchID: UUID, recordedAt: String, items: [ClientWriteItem]) {
    self.batchID = batchID
    self.recordedAt = recordedAt
    self.items = items
  }

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

  public init(id: UUID, recordClass: String, schemaVersion: UInt, content: JSONValue) {
    self.id = id
    self.`class` = recordClass
    self.schemaVersion = schemaVersion
    self.content = content
  }

  enum CodingKeys: String, CodingKey {
    case id, `class`, content
    case schemaVersion = "schema_version"
  }
}

public struct ClientProjectionQuery: Codable, Equatable, Hashable, Sendable {
  public let metric: String?
  public let windowStart: String?
  public let windowEnd: String?
  public let cursor: String?

  public init(metric: String?, windowStart: String?, windowEnd: String?, cursor: String?) {
    self.metric = metric
    self.windowStart = windowStart
    self.windowEnd = windowEnd
    self.cursor = cursor
  }

  enum CodingKeys: String, CodingKey {
    case metric
    case windowStart = "window_start"
    case windowEnd = "window_end"
    case cursor
  }
}

public struct ClientProjectionPage: Codable, Equatable, Sendable {
  public let apiVersion: String
  public let metric: String
  public let observedAt: String?
  public let generatedAt: String
  public let points: [ClientProjectionPoint]
  public let missingIntervals: [[String]]
  public let nextCursor: String?

  public init(
    apiVersion: String,
    metric: String,
    observedAt: String?,
    generatedAt: String,
    points: [ClientProjectionPoint],
    missingIntervals: [[String]],
    nextCursor: String?
  ) {
    self.apiVersion = apiVersion
    self.metric = metric
    self.observedAt = observedAt
    self.generatedAt = generatedAt
    self.points = points
    self.missingIntervals = missingIntervals
    self.nextCursor = nextCursor
  }

  enum CodingKeys: String, CodingKey {
    case apiVersion = "api_version"
    case metric
    case observedAt = "observed_at"
    case generatedAt = "generated_at"
    case points
    case missingIntervals = "missing_intervals"
    case nextCursor = "next_cursor"
  }
}

public struct ClientProjectionPoint: Codable, Equatable, Sendable {
  public let timestamp: String
  public let value: Double?

  public init(timestamp: String, value: Double?) {
    self.timestamp = timestamp
    self.value = value
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
