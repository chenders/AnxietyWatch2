import Foundation

/// A durable boundary for one HealthKit query page. The anchor must be committed in the same
/// transaction as `payload`; callers must not persist it separately.
public protocol HealthKitBatchStore: Sendable {
  func anchorData(for typeIdentifier: String) async throws -> Data?
  func commit(
    _ payload: HealthKitEnvelopePayload,
    typeIdentifier: String,
    nextAnchorData: Data?,
    capturedAt: Date
  ) async throws
}

/// Testable anchor serialization kept separate from HealthKit query execution.
public enum HealthKitAnchorCodec {
  public static func encode(_ data: Data) -> Data { data }
  public static func decode(_ data: Data?) -> Data? { data }
}

#if canImport(HealthKit)
import HealthKit

/// One completed page and its independent operational facts.
public struct HealthKitQueryPage: Sendable, Equatable {
  public let typeIdentifier: String
  public let payload: HealthKitEnvelopePayload
  public let nextAnchorData: Data?
  public let queriedAt: Date

  public init(
    typeIdentifier: String,
    payload: HealthKitEnvelopePayload,
    nextAnchorData: Data?,
    queriedAt: Date
  ) {
    self.typeIdentifier = typeIdentifier
    self.payload = payload
    self.nextAnchorData = nextAnchorData
    self.queriedAt = queriedAt
  }
}

/// Per-type anchored HealthKit acquisition. It never writes HealthKit and never performs local
/// aggregation. A failed mapping or durable commit leaves the previous anchor intact, so retrying
/// the page is safe and server UUID identity absorbs a replay.
public final class HealthKitAnchoredQueryEngine: @unchecked Sendable {
  private let healthStore: HKHealthStore
  private let mapper: HealthKitMapper
  private let initialLookback: TimeInterval

  public init(
    healthStore: HKHealthStore = HKHealthStore(),
    registry: RegistryDocument,
    initialLookback: TimeInterval = 7 * 24 * 60 * 60
  ) throws {
    guard initialLookback > 0 else {
      throw HealthKitMappingError.invalidInterval("initial_lookback=\(initialLookback)")
    }
    self.healthStore = healthStore
    self.mapper = try HealthKitMapper(registry: registry)
    self.initialLookback = initialLookback
  }

  /// Read and atomically commit one bounded page for a registry-enabled type.
  public func readAndCommit(
    _ definition: HealthKitReadTypeDefinition,
    store: any HealthKitBatchStore,
    now: Date = Date()
  ) async throws -> HealthKitQueryPage {
    guard let sampleType = Self.objectType(for: definition) else {
      throw HealthKitMappingError.unsupportedType(definition.identifier)
    }
    let existingAnchor = try await store.anchorData(for: definition.identifier)
    let anchor = try Self.decodeAnchor(existingAnchor)
    let predicate: NSPredicate?
    if anchor == nil {
      predicate = HKQuery.predicateForSamples(withStart: now.addingTimeInterval(-initialLookback), end: nil)
    } else {
      predicate = nil
    }
    let batch = try await queryPage(
      sampleType: sampleType,
      definition: definition,
      predicate: predicate,
      anchor: anchor,
      now: now
    )
    try await store.commit(
      batch.payload,
      typeIdentifier: definition.identifier,
      nextAnchorData: batch.nextAnchorData,
      capturedAt: now
    )
    return batch
  }

  private func queryPage(
    sampleType: HKSampleType,
    definition: HealthKitReadTypeDefinition,
    predicate: NSPredicate?,
    anchor: HKQueryAnchor?,
    now: Date
  ) async throws -> HealthKitQueryPage {
    let result: (
      samples: [HKSample],
      deletions: [HKDeletedObject],
      anchor: HKQueryAnchor?
    ) = try await withCheckedThrowingContinuation { continuation in
      let query = HKAnchoredObjectQuery(
        type: sampleType,
        predicate: predicate,
        anchor: anchor,
        limit: 100,
        resultsHandler: { _, samples, deletions, nextAnchor, error in
          if let error {
            continuation.resume(throwing: error)
          } else {
            continuation.resume(returning: (
              samples ?? [],
              deletions ?? [],
              nextAnchor
            ))
          }
        }
      )
      healthStore.execute(query)
    }

    let batchID = UUID()
    let recordedAt = Self.iso8601(now)
    let mappedSamples = try result.samples.map {
      try mapper.map(Self.transportSample(from: $0, definition: definition, batchID: batchID, recordedAt: recordedAt))
    }
    let mappedDeletions = try result.deletions.map {
      guard let type = PlatformTypeIdentifier(rawValue: definition.identifier) else {
        throw HealthKitMappingError.unsupportedType(definition.identifier)
      }
      return HealthKitDeletedSample(
        healthKitSampleUUID: $0.uuid,
        healthKitType: type,
        queryBatchID: batchID,
        recordedAt: recordedAt
      )
    }
    let payload = HealthKitEnvelopePayload(samples: mappedSamples, deletions: mappedDeletions)
    return HealthKitQueryPage(
      typeIdentifier: definition.identifier,
      payload: payload,
      nextAnchorData: try Self.encodeAnchor(result.anchor),
      queriedAt: now
    )
  }

  private static func transportSample(
    from sample: HKSample,
    definition: HealthKitReadTypeDefinition,
    batchID: UUID,
    recordedAt: String
  ) throws -> HealthKitTransportSample {
    guard let healthKitType = PlatformTypeIdentifier(rawValue: definition.identifier) else {
      throw HealthKitMappingError.unsupportedType(definition.identifier)
    }
    let revision = HealthKitSourceRevision(
      bundleIdentifier: sample.sourceRevision.source.bundleIdentifier,
      sourceName: sample.sourceRevision.source.name,
      productType: sample.sourceRevision.productType,
      version: sample.sourceRevision.version
    )
    let device = sample.device.map {
      HealthKitDevice(
        manufacturer: $0.manufacturer,
        model: $0.model,
        hardwareVersion: $0.hardwareVersion,
        osVersion: $0.softwareVersion
      )
    }
    let metadata = sample.metadata?.reduce(into: [String: String]()) { values, entry in
      guard HealthKitMapper.allowedMetadataKeys.contains(entry.key) else { return }
      values[entry.key] = String(describing: entry.value)
    } ?? [:]
    let originalValue: Double?
    let originalCategory: String?
    switch definition.kind {
    case .quantity:
      guard let quantitySample = sample as? HKQuantitySample,
            definition.platformUnit != nil,
            let hkUnit = Self.unit(for: definition.identifier) else {
        throw HealthKitMappingError.unsupportedType(definition.identifier)
      }
      originalValue = quantitySample.quantity.doubleValue(for: hkUnit)
      originalCategory = nil
    case .category:
      guard let categorySample = sample as? HKCategorySample else {
        throw HealthKitMappingError.unsupportedType(definition.identifier)
      }
      originalValue = nil
      originalCategory = Self.sleepCategoryName(categorySample.value)
    }
    return HealthKitTransportSample(
      healthKitSampleUUID: sample.uuid,
      healthKitType: healthKitType,
      startAt: Self.iso8601(sample.startDate),
      endAt: Self.iso8601(sample.endDate),
      originalValue: originalValue,
      originalUnit: definition.platformUnit,
      originalCategory: originalCategory,
      sourceRevision: revision,
      device: device,
      metadata: metadata,
      queryBatchID: batchID,
      recordedAt: recordedAt
    )
  }

  private static func objectType(for definition: HealthKitReadTypeDefinition) -> HKSampleType? {
    switch definition.identifier {
    case "HKQuantityTypeIdentifierHeartRate": return HKQuantityType(.heartRate)
    case "HKQuantityTypeIdentifierHeartRateVariabilitySDNN": return HKQuantityType(.heartRateVariabilitySDNN)
    case "HKQuantityTypeIdentifierOxygenSaturation": return HKQuantityType(.oxygenSaturation)
    case "HKQuantityTypeIdentifierRestingHeartRate": return HKQuantityType(.restingHeartRate)
    case "HKQuantityTypeIdentifierRespiratoryRate": return HKQuantityType(.respiratoryRate)
    case "HKQuantityTypeIdentifierAppleSleepingWristTemperature": return HKQuantityType(.appleSleepingWristTemperature)
    case "HKQuantityTypeIdentifierStepCount": return HKQuantityType(.stepCount)
    case "HKQuantityTypeIdentifierActiveEnergyBurned": return HKQuantityType(.activeEnergyBurned)
    case "HKQuantityTypeIdentifierAppleExerciseTime": return HKQuantityType(.appleExerciseTime)
    case "HKQuantityTypeIdentifierEnvironmentalAudioExposure": return HKQuantityType(.environmentalAudioExposure)
    case "HKQuantityTypeIdentifierBloodPressureSystolic": return HKQuantityType(.bloodPressureSystolic)
    case "HKQuantityTypeIdentifierBloodPressureDiastolic": return HKQuantityType(.bloodPressureDiastolic)
    case "HKQuantityTypeIdentifierBloodGlucose": return HKQuantityType(.bloodGlucose)
    case "HKQuantityTypeIdentifierVO2Max": return HKQuantityType(.vo2Max)
    case "HKQuantityTypeIdentifierWalkingHeartRateAverage": return HKQuantityType(.walkingHeartRateAverage)
    case "HKQuantityTypeIdentifierHeadphoneAudioExposure": return HKQuantityType(.headphoneAudioExposure)
    case "HKQuantityTypeIdentifierAppleWalkingSteadiness": return HKQuantityType(.appleWalkingSteadiness)
    case "HKQuantityTypeIdentifierAtrialFibrillationBurden": return HKQuantityType(.atrialFibrillationBurden)
    case "HKQuantityTypeIdentifierWalkingSpeed": return HKQuantityType(.walkingSpeed)
    case "HKQuantityTypeIdentifierWalkingStepLength": return HKQuantityType(.walkingStepLength)
    case "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage": return HKQuantityType(.walkingDoubleSupportPercentage)
    case "HKQuantityTypeIdentifierWalkingAsymmetryPercentage": return HKQuantityType(.walkingAsymmetryPercentage)
    case "HKQuantityTypeIdentifierTimeInDaylight": return HKQuantityType(.timeInDaylight)
    case "HKQuantityTypeIdentifierPhysicalEffort": return HKQuantityType(.physicalEffort)
    case "HKQuantityTypeIdentifierBodyMass": return HKQuantityType(.bodyMass)
    case "HKCategoryTypeIdentifierSleepAnalysis": return HKCategoryType(.sleepAnalysis)
    default: return nil
    }
  }

  private static func unit(for identifier: String) -> HKUnit? {
    switch identifier {
    case "HKQuantityTypeIdentifierHeartRate", "HKQuantityTypeIdentifierRestingHeartRate", "HKQuantityTypeIdentifierWalkingHeartRateAverage", "HKQuantityTypeIdentifierRespiratoryRate": return .count().unitDivided(by: .minute())
    case "HKQuantityTypeIdentifierHeartRateVariabilitySDNN": return .secondUnit(with: .milli)
    case "HKQuantityTypeIdentifierOxygenSaturation", "HKQuantityTypeIdentifierAppleWalkingSteadiness", "HKQuantityTypeIdentifierAtrialFibrillationBurden", "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage", "HKQuantityTypeIdentifierWalkingAsymmetryPercentage": return .percent()
    case "HKQuantityTypeIdentifierAppleSleepingWristTemperature": return .degreeCelsius()
    case "HKQuantityTypeIdentifierStepCount": return .count()
    case "HKQuantityTypeIdentifierActiveEnergyBurned": return .kilocalorie()
    case "HKQuantityTypeIdentifierAppleExerciseTime", "HKQuantityTypeIdentifierTimeInDaylight": return .minute()
    case "HKQuantityTypeIdentifierEnvironmentalAudioExposure", "HKQuantityTypeIdentifierHeadphoneAudioExposure": return .decibelAWeightedSoundPressureLevel()
    case "HKQuantityTypeIdentifierBloodPressureSystolic", "HKQuantityTypeIdentifierBloodPressureDiastolic": return .millimeterOfMercury()
    case "HKQuantityTypeIdentifierBloodGlucose": return .gramUnit(with: .milli).unitDivided(by: .literUnit(with: .deci))
    case "HKQuantityTypeIdentifierVO2Max": return HKUnit(from: "mL/kg*min")
    case "HKQuantityTypeIdentifierWalkingSpeed": return .meter().unitDivided(by: .second())
    case "HKQuantityTypeIdentifierWalkingStepLength": return .meter()
    case "HKQuantityTypeIdentifierPhysicalEffort": return HKUnit(from: "kcal/kg*hr")
    case "HKQuantityTypeIdentifierBodyMass": return .gramUnit(with: .kilo)
    default: return nil
    }
  }

  private static func sleepCategoryName(_ value: Int) -> String {
    switch value {
    case HKCategoryValueSleepAnalysis.awake.rawValue: return "HKCategoryValueSleepAnalysisAwake"
    case HKCategoryValueSleepAnalysis.asleepUnspecified.rawValue: return "HKCategoryValueSleepAnalysisAsleepUnspecified"
    case HKCategoryValueSleepAnalysis.asleepCore.rawValue: return "HKCategoryValueSleepAnalysisAsleepCore"
    case HKCategoryValueSleepAnalysis.asleepDeep.rawValue: return "HKCategoryValueSleepAnalysisAsleepDeep"
    case HKCategoryValueSleepAnalysis.asleepREM.rawValue: return "HKCategoryValueSleepAnalysisAsleepREM"
    default: return "HKCategoryValueSleepAnalysisRawValue\(value)"
    }
  }

  private static func encodeAnchor(_ anchor: HKQueryAnchor?) throws -> Data? {
    guard let anchor else { return nil }
    return try NSKeyedArchiver.archivedData(withRootObject: anchor, requiringSecureCoding: true)
  }

  private static func decodeAnchor(_ data: Data?) throws -> HKQueryAnchor? {
    guard let data else { return nil }
    return try NSKeyedUnarchiver.unarchivedObject(ofClass: HKQueryAnchor.self, from: data)
  }

  private static func iso8601(_ date: Date) -> String {
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    return formatter.string(from: date)
  }
}
#endif
