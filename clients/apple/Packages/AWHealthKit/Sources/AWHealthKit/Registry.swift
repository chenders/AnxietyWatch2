import Foundation

/// One canonical registry type fetched from the Pi.
public struct RegistryType: Codable, Equatable, Sendable {
  public let id: Int
  public let unit: String
  public let range: [Double]
  public let precision: Double

  public init(id: Int, unit: String, range: [Double], precision: Double) {
    self.id = id
    self.unit = unit
    self.range = range
    self.precision = precision
  }
}

/// One source entry fetched from the Pi registry.
public struct RegistrySource: Codable, Equatable, Sendable {
  public let id: Int
  public let transport: String
  public let provides: [String]
  public let expectedCadenceSeconds: Int?
  public let onDemand: Bool?

  public init(
    id: Int,
    transport: String,
    provides: [String],
    expectedCadenceSeconds: Int? = nil,
    onDemand: Bool? = nil
  ) {
    self.id = id
    self.transport = transport
    self.provides = provides
    self.expectedCadenceSeconds = expectedCadenceSeconds
    self.onDemand = onDemand
  }

  enum CodingKeys: String, CodingKey {
    case id, transport, provides
    case expectedCadenceSeconds = "expected_cadence_s"
    case onDemand = "on_demand"
  }
}

/// The generated registry document returned by the server.
public struct RegistryDocument: Codable, Equatable, Sendable {
  public let types: [String: RegistryType]
  public let sources: [String: RegistrySource]

  public init(types: [String: RegistryType], sources: [String: RegistrySource]) {
    self.types = types
    self.sources = sources
  }

  enum CodingKeys: String, CodingKey {
    case types = "type"
    case sources = "source"
  }

  /// Decode and validate a registry before any mapper can use it.
  public static func decode(_ data: Data) throws -> RegistryDocument {
    let document = try JSONDecoder().decode(RegistryDocument.self, from: data)
    try document.validate()
    return document
  }

  /// Reject malformed ranges and missing source/type identifiers by name.
  public func validate() throws {
    guard !types.isEmpty else { throw RegistryError.emptyTypes }
    guard !sources.isEmpty else { throw RegistryError.emptySources }
    for (name, type) in types {
      guard type.range.count == 2,
            let lower = type.range.first,
            let upper = type.range.last,
            lower.isFinite,
            upper.isFinite,
            lower <= upper,
            type.precision.isFinite,
            type.precision > 0,
            !type.unit.isEmpty else {
        throw RegistryError.invalidType(name)
      }
    }
    for (name, source) in sources {
      guard !source.transport.isEmpty,
            !source.provides.isEmpty else {
        throw RegistryError.invalidSource(name)
      }
      guard source.provides.allSatisfy({ types[$0] != nil }) else {
        throw RegistryError.sourceProvidesUnknownType(name)
      }
    }
  }

  /// Returns a type only when the fetched registry declares it.
  public func type(named name: String) -> RegistryType? { types[name] }

  /// Returns a source only when the fetched registry declares it.
  public func source(named name: String) -> RegistrySource? { sources[name] }
}

/// Named registry validation failures.
public enum RegistryError: Error, Equatable, Sendable {
  case emptyTypes
  case emptySources
  case invalidType(String)
  case invalidSource(String)
  case sourceProvidesUnknownType(String)
  case invalidAPIVersion(String)
}

/// In-memory cache for the last validated registry, with explicit freshness.
public actor RegistryCache {
  public struct Entry: Equatable, Sendable {
    public let document: RegistryDocument
    public let apiVersion: String
    public let fetchedAt: Date

    public init(document: RegistryDocument, apiVersion: String, fetchedAt: Date) {
      self.document = document
      self.apiVersion = apiVersion
      self.fetchedAt = fetchedAt
    }
  }

  private var entry: Entry?

  public init() {}

  public func put(
    _ document: RegistryDocument,
    apiVersion: String,
    fetchedAt: Date
  ) throws {
    try document.validate()
    guard apiVersion.split(separator: ".").first.flatMap({ UInt($0) }) == 1 else {
      throw RegistryError.invalidAPIVersion(apiVersion)
    }
    entry = Entry(document: document, apiVersion: apiVersion, fetchedAt: fetchedAt)
  }

  public func get() -> Entry? { entry }
  public func remove() { entry = nil }
}
