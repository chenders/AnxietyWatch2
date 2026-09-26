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

/// A deletion reported by an anchored HealthKit query. The Pi records a tombstone rather than
/// deleting a sample row, preserving the platform's deletion fact for reconciliation.
public struct HealthKitDeletedSample: Codable, Equatable, Sendable {
  public let healthKitSampleUUID: UUID
  public let healthKitType: PlatformTypeIdentifier
  public let queryBatchID: UUID
  public let recordedAt: String

  public init(
    healthKitSampleUUID: UUID,
    healthKitType: PlatformTypeIdentifier,
    queryBatchID: UUID,
    recordedAt: String
  ) {
    self.healthKitSampleUUID = healthKitSampleUUID
    self.healthKitType = healthKitType
    self.queryBatchID = queryBatchID
    self.recordedAt = recordedAt
  }

  enum CodingKeys: String, CodingKey {
    case healthKitSampleUUID = "healthkit_sample_uuid"
    case healthKitType = "healthkit_type"
    case queryBatchID = "query_batch_id"
    case recordedAt = "recorded_at"
  }
}

/// The platform-sample envelope content. Canonical conversion is server-owned.
public struct HealthKitEnvelopePayload: Codable, Equatable, Sendable {
  public let `class`: String
  public let schemaVersion: UInt
  public let samples: [HealthKitTransportSample]
  public let deletions: [HealthKitDeletedSample]

  public init(
    samples: [HealthKitTransportSample],
    deletions: [HealthKitDeletedSample] = []
  ) {
    self.`class` = "platform_sample_batch"
    self.schemaVersion = 1
    self.samples = samples
    self.deletions = deletions
  }

  enum CodingKeys: String, CodingKey {
    case `class`
    case schemaVersion = "schema_version"
    case samples, deletions
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    `class` = try container.decode(String.self, forKey: .class)
    schemaVersion = try container.decode(UInt.self, forKey: .schemaVersion)
    samples = try container.decode([HealthKitTransportSample].self, forKey: .samples)
    deletions = try container.decodeIfPresent([HealthKitDeletedSample].self, forKey: .deletions) ?? []
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
    case "HKQuantityTypeIdentifierOxygenSaturation": return "spo2"
    case "HKCategoryTypeIdentifierSleepAnalysis": return "sleep_stage"
    case "HKQuantityTypeIdentifierRestingHeartRate": return "resting_hr"
    case "HKQuantityTypeIdentifierRespiratoryRate": return "respiratory_rate"
    case "HKQuantityTypeIdentifierAppleSleepingWristTemperature": return "wrist_temperature"
    case "HKQuantityTypeIdentifierStepCount": return "step_count"
    case "HKQuantityTypeIdentifierActiveEnergyBurned": return "active_energy"
    case "HKQuantityTypeIdentifierAppleExerciseTime": return "exercise_time"
    case "HKQuantityTypeIdentifierEnvironmentalAudioExposure": return "environmental_audio"
    case "HKQuantityTypeIdentifierBloodPressureSystolic": return "bp_systolic"
    case "HKQuantityTypeIdentifierBloodPressureDiastolic": return "bp_diastolic"
    case "HKQuantityTypeIdentifierBloodGlucose": return "blood_glucose"
    case "HKQuantityTypeIdentifierVO2Max": return "vo2_max"
    case "HKQuantityTypeIdentifierWalkingHeartRateAverage": return "walking_hr"
    case "HKQuantityTypeIdentifierHeadphoneAudioExposure": return "headphone_audio"
    case "HKQuantityTypeIdentifierAppleWalkingSteadiness": return "walking_steadiness"
    case "HKQuantityTypeIdentifierAtrialFibrillationBurden": return "afib_burden"
    case "HKQuantityTypeIdentifierWalkingSpeed": return "walking_speed"
    case "HKQuantityTypeIdentifierWalkingStepLength": return "walking_step_length"
    case "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage": return "walking_double_support"
    case "HKQuantityTypeIdentifierWalkingAsymmetryPercentage": return "walking_asymmetry"
    case "HKQuantityTypeIdentifierTimeInDaylight": return "time_in_daylight"
    case "HKQuantityTypeIdentifierPhysicalEffort": return "physical_effort"
    case "HKQuantityTypeIdentifierBodyMass": return "body_mass"
    default: throw HealthKitMappingError.unsupportedType(identifier)
    }
  }

  private func validate(_ sample: HealthKitTransportSample, registryName: String) throws {
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    let fallbackFormatter = ISO8601DateFormatter()
    let start = formatter.date(from: sample.startAt) ?? fallbackFormatter.date(from: sample.startAt)
    let end = formatter.date(from: sample.endAt) ?? fallbackFormatter.date(from: sample.endAt)
    guard let start, let end, end >= start else {
      throw HealthKitMappingError.invalidInterval(
        "start_at=\(sample.startAt), end_at=\(sample.endAt)"
      )
    }
    guard formatter.date(from: sample.recordedAt) != nil
      || fallbackFormatter.date(from: sample.recordedAt) != nil else {
      throw HealthKitMappingError.invalidRecordedAt(sample.recordedAt)
    }
    guard sample.metadata.keys.allSatisfy(Self.allowedMetadataKeys.contains) else {
      let unknown = sample.metadata.keys
        .filter { !Self.allowedMetadataKeys.contains($0) }
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

  public static let allowedMetadataKeys: Set<String> = [
    "HKMetadataKeyWasUserEntered",
    "HKMetadataKeySyncIdentifier",
    "HKMetadataKeySyncVersion",
  ]
}
