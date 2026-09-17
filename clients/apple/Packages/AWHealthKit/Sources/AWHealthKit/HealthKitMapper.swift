import Foundation

/// A source revision retained before the Pi performs canonical mapping.
public struct HealthKitSourceRevision: Codable, Equatable, Sendable {
  public let bundleIdentifier: String?
  public let sourceName: String?
  public let productType: String?
  public let version: String?

  public init(
    bundleIdentifier: String? = nil,
    sourceName: String? = nil,
    productType: String? = nil,
    version: String? = nil
  ) {
    self.bundleIdentifier = bundleIdentifier
    self.sourceName = sourceName
    self.productType = productType
    self.version = version
  }

  enum CodingKeys: String, CodingKey {
    case bundleIdentifier = "bundle_identifier"
    case sourceName = "source_name"
    case productType = "product_type"
    case version
  }
}

/// Device details retained as HealthKit sub-provenance.
public struct HealthKitDevice: Codable, Equatable, Sendable {
  public let manufacturer: String?
  public let model: String?
  public let hardwareVersion: String?
  public let osVersion: String?

  public init(
    manufacturer: String? = nil,
    model: String? = nil,
    hardwareVersion: String? = nil,
    osVersion: String? = nil
  ) {
    self.manufacturer = manufacturer
    self.model = model
    self.hardwareVersion = hardwareVersion
    self.osVersion = osVersion
  }

  enum CodingKeys: String, CodingKey {
    case manufacturer, model
    case hardwareVersion = "hardware_version"
    case osVersion = "os_version"
  }
}

/// Original HealthKit data sent to the platform-sample envelope.
public struct HealthKitTransportSample: Codable, Equatable, Sendable {
  public let healthKitSampleUUID: UUID
  public let healthKitType: PlatformTypeIdentifier
  public let startAt: String
  public let endAt: String
  public let originalValue: Double?
  public let originalUnit: String?
  public let originalCategory: String?
  public let sourceRevision: HealthKitSourceRevision
  public let device: HealthKitDevice?
  public let metadata: [String: String]
  public let queryBatchID: UUID
  public let recordedAt: String

  public init(
    healthKitSampleUUID: UUID,
    healthKitType: PlatformTypeIdentifier,
    startAt: String,
    endAt: String,
    originalValue: Double? = nil,
    originalUnit: String? = nil,
    originalCategory: String? = nil,
    sourceRevision: HealthKitSourceRevision = HealthKitSourceRevision(),
    device: HealthKitDevice? = nil,
    metadata: [String: String] = [:],
    queryBatchID: UUID,
    recordedAt: String
  ) {
    self.healthKitSampleUUID = healthKitSampleUUID
    self.healthKitType = healthKitType
    self.startAt = startAt
    self.endAt = endAt
    self.originalValue = originalValue
    self.originalUnit = originalUnit
    self.originalCategory = originalCategory
    self.sourceRevision = sourceRevision
    self.device = device
    self.metadata = metadata
    self.queryBatchID = queryBatchID
    self.recordedAt = recordedAt
  }

  enum CodingKeys: String, CodingKey {
    case healthKitSampleUUID = "healthkit_sample_uuid"
    case healthKitType = "healthkit_type"
    case startAt = "start_at"
    case endAt = "end_at"
    case originalValue = "original_value"
    case originalUnit = "original_unit"
    case originalCategory = "original_category"
    case sourceRevision = "source_revision"
    case device, metadata
    case queryBatchID = "query_batch_id"
    case recordedAt = "recorded_at"
  }
}

/// The platform-sample envelope content. Canonical conversion is server-owned.
public struct HealthKitEnvelopePayload: Codable, Equatable, Sendable {
  public let `class`: String
  public let schemaVersion: UInt
  public let samples: [HealthKitTransportSample]

  public init(samples: [HealthKitTransportSample]) {
    self.`class` = "platform_sample_batch"
    self.schemaVersion = 1
    self.samples = samples
  }

  enum CodingKeys: String, CodingKey {
    case `class`
    case schemaVersion = "schema_version"
    case samples
  }
}

/// Named rejection reasons for data that must remain diagnosable and unqueued.
public enum HealthKitMappingError: Error, Equatable, Sendable {
  case unsupportedType(String)
  case registrySourceMissing
  case registryTypeMissing(String)
  case invalidInterval(String)
  case missingValue(String)
  case nonFiniteValue(String)
  case emptyUnit(String)
  case unknownMetadata(String)
  case invalidRecordedAt(String)
}

/// Pure mapper from original HealthKit source records to the server envelope.
public struct HealthKitMapper: Sendable {
  private let registry: RegistryDocument

  public init(registry: RegistryDocument) throws {
    try registry.validate()
    self.registry = registry
  }

  /// Maps one source sample without converting canonical units or inventing identity.
  public func map(_ sample: HealthKitTransportSample) throws -> HealthKitTransportSample {
    let registryName = try registryName(for: sample.healthKitType.rawValue)
    guard registry.source(named: "healthkit")?.provides.contains(registryName) == true else {
      throw HealthKitMappingError.registrySourceMissing
    }
    guard registry.type(named: registryName) != nil else {
      throw HealthKitMappingError.registryTypeMissing(registryName)
    }
    try validate(sample, registryName: registryName)
    return sample
  }

  /// Builds a bounded envelope after every sample has passed the registry gate.
  public func envelope(for samples: [HealthKitTransportSample]) throws -> HealthKitEnvelopePayload {
    guard !samples.isEmpty else { throw HealthKitMappingError.missingValue("batch is empty") }
    guard samples.count <= 100 else {
      throw HealthKitMappingError.missingValue("batch count=\(samples.count); allowed=1...100")
    }
    let mapped = try samples.map(map)
    return HealthKitEnvelopePayload(samples: mapped)
  }

  private func registryName(for identifier: String) throws -> String {
    switch identifier {
    case "HKQuantityTypeIdentifierHeartRate": return "hr"
    case "HKQuantityTypeIdentifierHeartRateVariabilitySDNN": return "hrv_rmssd"
    case "HKCategoryTypeIdentifierSleepAnalysis": return "sleep_stage"
    default: throw HealthKitMappingError.unsupportedType(identifier)
    }
  }

  private func validate(_ sample: HealthKitTransportSample, registryName: String) throws {
    let formatter = ISO8601DateFormatter()
    guard let start = formatter.date(from: sample.startAt),
          let end = formatter.date(from: sample.endAt),
          end >= start else {
      throw HealthKitMappingError.invalidInterval(
        "start_at=\(sample.startAt), end_at=\(sample.endAt)"
      )
    }
    guard formatter.date(from: sample.recordedAt) != nil else {
      throw HealthKitMappingError.invalidRecordedAt(sample.recordedAt)
    }
    guard sample.metadata.keys.allSatisfy(Self.allowedMetadata.contains) else {
      let unknown = sample.metadata.keys
        .filter { !Self.allowedMetadata.contains($0) }
        .sorted()
        .joined(separator: ",")
      throw HealthKitMappingError.unknownMetadata(unknown)
    }
    if registryName == "sleep_stage" {
      guard let category = sample.originalCategory, !category.isEmpty else {
        throw HealthKitMappingError.missingValue("sleep_stage original_category=null")
      }
    } else {
      guard let value = sample.originalValue else {
        throw HealthKitMappingError.missingValue("\(registryName) original_value=null")
      }
      guard value.isFinite else {
        throw HealthKitMappingError.nonFiniteValue("\(registryName) value=\(value)")
      }
      guard let unit = sample.originalUnit, !unit.isEmpty else {
        throw HealthKitMappingError.emptyUnit(registryName)
      }
    }
  }

  private static let allowedMetadata: Set<String> = [
    "HKMetadataKeyWasUserEntered",
    "HKMetadataKeySyncIdentifier",
    "HKMetadataKeySyncVersion",
  ]
}
