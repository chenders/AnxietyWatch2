import Foundation

/// A registry-backed HealthKit type that the first-release phone bridge can request and upload.
public struct HealthKitReadTypeDefinition: Codable, Equatable, Sendable, Identifiable {
  public let identifier: String
  public let registryName: String
  public let displayName: String
  public let rationale: String
  public let kind: Kind
  public let platformUnit: String?

  public enum Kind: String, Codable, Equatable, Sendable {
    case quantity
    case category
  }

  public init(
    identifier: String,
    registryName: String,
    displayName: String,
    rationale: String,
    kind: Kind,
    platformUnit: String? = nil
  ) {
    self.identifier = identifier
    self.registryName = registryName
    self.displayName = displayName
    self.rationale = rationale
    self.kind = kind
    self.platformUnit = platformUnit
  }

  public var id: String { identifier }
}

/// The retained first-release HealthKit source set. Context-only and unsupported clinical/series
/// types are deliberately absent: they need separate contracts and must not be flattened.
public enum HealthKitReadPlan {
  public static let firstRelease: [HealthKitReadTypeDefinition] = [
    .init(identifier: "HKQuantityTypeIdentifierHeartRate", registryName: "hr", displayName: "Heart rate", rationale: "Upload heart-rate source samples to your Pi for server-side projections.", kind: .quantity, platformUnit: "count/min"),
    .init(identifier: "HKQuantityTypeIdentifierHeartRateVariabilitySDNN", registryName: "hrv_rmssd", displayName: "Heart-rate variability", rationale: "Upload HRV source samples to your Pi without calculating local trends.", kind: .quantity, platformUnit: "ms"),
    .init(identifier: "HKQuantityTypeIdentifierOxygenSaturation", registryName: "spo2", displayName: "Oxygen saturation", rationale: "Upload oxygen-saturation source samples to your Pi with source provenance.", kind: .quantity, platformUnit: "%"),
    .init(identifier: "HKCategoryTypeIdentifierSleepAnalysis", registryName: "sleep_stage", displayName: "Sleep stages", rationale: "Upload each sleep-stage interval to your Pi; the Pi computes sleep projections.", kind: .category),
    .init(identifier: "HKQuantityTypeIdentifierRestingHeartRate", registryName: "resting_hr", displayName: "Resting heart rate", rationale: "Upload HealthKit resting-heart-rate samples to your Pi.", kind: .quantity, platformUnit: "count/min"),
    .init(identifier: "HKQuantityTypeIdentifierRespiratoryRate", registryName: "respiratory_rate", displayName: "Respiratory rate", rationale: "Upload respiratory-rate source samples to your Pi.", kind: .quantity, platformUnit: "count/min"),
    .init(identifier: "HKQuantityTypeIdentifierAppleSleepingWristTemperature", registryName: "wrist_temperature", displayName: "Sleeping wrist temperature", rationale: "Upload sleeping wrist-temperature samples without local baseline calculations.", kind: .quantity, platformUnit: "degC"),
    .init(identifier: "HKQuantityTypeIdentifierStepCount", registryName: "step_count", displayName: "Step count", rationale: "Upload step source intervals; the Pi owns daily projections.", kind: .quantity, platformUnit: "count"),
    .init(identifier: "HKQuantityTypeIdentifierActiveEnergyBurned", registryName: "active_energy", displayName: "Active energy", rationale: "Upload active-energy source intervals to your Pi.", kind: .quantity, platformUnit: "kcal"),
    .init(identifier: "HKQuantityTypeIdentifierAppleExerciseTime", registryName: "exercise_time", displayName: "Exercise time", rationale: "Upload exercise-time source intervals to your Pi.", kind: .quantity, platformUnit: "min"),
    .init(identifier: "HKQuantityTypeIdentifierEnvironmentalAudioExposure", registryName: "environmental_audio", displayName: "Environmental audio exposure", rationale: "Upload environmental audio exposure samples to your Pi.", kind: .quantity, platformUnit: "dBASPL"),
    .init(identifier: "HKQuantityTypeIdentifierBloodPressureSystolic", registryName: "bp_systolic", displayName: "Systolic blood pressure", rationale: "Upload systolic blood-pressure source samples to your Pi.", kind: .quantity, platformUnit: "mmHg"),
    .init(identifier: "HKQuantityTypeIdentifierBloodPressureDiastolic", registryName: "bp_diastolic", displayName: "Diastolic blood pressure", rationale: "Upload diastolic blood-pressure source samples to your Pi.", kind: .quantity, platformUnit: "mmHg"),
    .init(identifier: "HKQuantityTypeIdentifierBloodGlucose", registryName: "blood_glucose", displayName: "Blood glucose", rationale: "Upload blood-glucose source samples with their HealthKit provenance.", kind: .quantity, platformUnit: "mg/dL"),
    .init(identifier: "HKQuantityTypeIdentifierVO2Max", registryName: "vo2_max", displayName: "VO₂ max", rationale: "Upload HealthKit VO₂ max samples; the Pi does not recompute them.", kind: .quantity, platformUnit: "mL/kg*min"),
    .init(identifier: "HKQuantityTypeIdentifierWalkingHeartRateAverage", registryName: "walking_hr", displayName: "Walking heart rate", rationale: "Upload walking heart-rate source samples to your Pi.", kind: .quantity, platformUnit: "count/min"),
    .init(identifier: "HKQuantityTypeIdentifierHeadphoneAudioExposure", registryName: "headphone_audio", displayName: "Headphone audio exposure", rationale: "Upload headphone exposure samples to your Pi.", kind: .quantity, platformUnit: "dBASPL"),
    .init(identifier: "HKQuantityTypeIdentifierAppleWalkingSteadiness", registryName: "walking_steadiness", displayName: "Walking steadiness", rationale: "Upload walking-steadiness source samples to your Pi.", kind: .quantity, platformUnit: "%"),
    .init(identifier: "HKQuantityTypeIdentifierAtrialFibrillationBurden", registryName: "afib_burden", displayName: "Atrial-fibrillation burden", rationale: "Upload HealthKit atrial-fibrillation burden samples without local interpretation.", kind: .quantity, platformUnit: "%"),
    .init(identifier: "HKQuantityTypeIdentifierWalkingSpeed", registryName: "walking_speed", displayName: "Walking speed", rationale: "Upload walking-speed source samples to your Pi.", kind: .quantity, platformUnit: "m/s"),
    .init(identifier: "HKQuantityTypeIdentifierWalkingStepLength", registryName: "walking_step_length", displayName: "Walking step length", rationale: "Upload walking step-length source samples to your Pi.", kind: .quantity, platformUnit: "m"),
    .init(identifier: "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage", registryName: "walking_double_support", displayName: "Walking double-support", rationale: "Upload walking double-support source samples to your Pi.", kind: .quantity, platformUnit: "%"),
    .init(identifier: "HKQuantityTypeIdentifierWalkingAsymmetryPercentage", registryName: "walking_asymmetry", displayName: "Walking asymmetry", rationale: "Upload walking-asymmetry source samples to your Pi.", kind: .quantity, platformUnit: "%"),
    .init(identifier: "HKQuantityTypeIdentifierTimeInDaylight", registryName: "time_in_daylight", displayName: "Time in daylight", rationale: "Upload daylight exposure source intervals to your Pi.", kind: .quantity, platformUnit: "min"),
    .init(identifier: "HKQuantityTypeIdentifierPhysicalEffort", registryName: "physical_effort", displayName: "Physical effort", rationale: "Upload physical-effort source samples so the Pi can distinguish exertion from anxiety.", kind: .quantity, platformUnit: "kcal/kg*hr"),
    .init(identifier: "HKQuantityTypeIdentifierBodyMass", registryName: "body_mass", displayName: "Body mass", rationale: "Upload body-mass source samples to your Pi.", kind: .quantity, platformUnit: "kg"),
  ]

  public static func definition(for identifier: String) -> HealthKitReadTypeDefinition? {
    firstRelease.first { $0.identifier == identifier }
  }
}

/// Facts shown to the user about an authorization request. `requested` is not `granted`: HealthKit
/// intentionally hides read authorization status from the app.
public struct HealthKitAuthorizationFacts: Codable, Equatable, Sendable {
  public let attemptedAt: Date?
  public let outcome: String?
  public let requestStatus: String?

  public init(attemptedAt: Date? = nil, outcome: String? = nil, requestStatus: String? = nil) {
    self.attemptedAt = attemptedAt
    self.outcome = outcome
    self.requestStatus = requestStatus
  }
}

public protocol HealthKitAuthorizationFactStore: Sendable {
  func loadAuthorizationFacts() async throws -> HealthKitAuthorizationFacts
  func saveAuthorizationFacts(_ facts: HealthKitAuthorizationFacts) async throws
}

public protocol HealthKitAuthorizationRequesting: Sendable {
  func requestReadAuthorization(for types: [HealthKitReadTypeDefinition]) async throws
  func requestStatus(for types: [HealthKitReadTypeDefinition]) async -> String
}

/// Stores only the authorization request facts that HealthKit permits the app to retain.
public final class UserDefaultsHealthKitAuthorizationFactStore: HealthKitAuthorizationFactStore, @unchecked Sendable {
  private let defaults: UserDefaults
  private let key: String

  public init(defaults: UserDefaults = .standard, key: String = "healthkit.authorization-facts.v1") {
    self.defaults = defaults
    self.key = key
  }

  public func loadAuthorizationFacts() async throws -> HealthKitAuthorizationFacts {
    guard let data = defaults.data(forKey: key) else { return HealthKitAuthorizationFacts() }
    return try JSONDecoder().decode(HealthKitAuthorizationFacts.self, from: data)
  }

  public func saveAuthorizationFacts(_ facts: HealthKitAuthorizationFacts) async throws {
    defaults.set(try JSONEncoder().encode(facts), forKey: key)
  }
}

/// Coordinates the inventive but honest access flow: explain each type, offer the system sheet once,
/// and retain the ask/result facts while never claiming that a read grant was observable.
public actor HealthKitAuthorizationCoordinator {
  private let client: any HealthKitAuthorizationRequesting
  private let factsStore: any HealthKitAuthorizationFactStore
  private let now: @Sendable () -> Date

  public init(
    client: any HealthKitAuthorizationRequesting,
    factsStore: any HealthKitAuthorizationFactStore,
    now: @escaping @Sendable () -> Date = { Date() }
  ) {
    self.client = client
    self.factsStore = factsStore
    self.now = now
  }

  public func facts() async -> HealthKitAuthorizationFacts {
    (try? await factsStore.loadAuthorizationFacts()) ?? HealthKitAuthorizationFacts()
  }

  public func requestIfNeeded(
    types: [HealthKitReadTypeDefinition] = HealthKitReadPlan.firstRelease,
    force: Bool = false
  ) async -> HealthKitAuthorizationFacts {
    let existing = await facts()
    guard force || existing.attemptedAt == nil else { return existing }
    let attemptedAt = now()
    do {
      try await client.requestReadAuthorization(for: types)
      let facts = HealthKitAuthorizationFacts(
        attemptedAt: attemptedAt,
        outcome: "requested",
        requestStatus: await client.requestStatus(for: types)
      )
      try? await factsStore.saveAuthorizationFacts(facts)
      return facts
    } catch {
      let facts = HealthKitAuthorizationFacts(
        attemptedAt: attemptedAt,
        outcome: "failed: \(error.localizedDescription)",
        requestStatus: await client.requestStatus(for: types)
      )
      try? await factsStore.saveAuthorizationFacts(facts)
      return facts
    }
  }
}

#if canImport(HealthKit)
import HealthKit

/// System adapter for the read-only HealthKit prompt. The share set is permanently empty.
public final class SystemHealthKitAuthorizationClient: HealthKitAuthorizationRequesting, @unchecked Sendable {
  private let healthStore = HKHealthStore()

  public init() {}

  public func requestReadAuthorization(for types: [HealthKitReadTypeDefinition]) async throws {
    guard HKHealthStore.isHealthDataAvailable() else { return }
    try await healthStore.requestAuthorization(toShare: [], read: Self.objectTypes(for: types))
  }

  public func requestStatus(for types: [HealthKitReadTypeDefinition]) async -> String {
    guard HKHealthStore.isHealthDataAvailable() else { return "unavailable" }
    do {
      let status = try await healthStore.statusForAuthorizationRequest(
        toShare: [], read: Self.objectTypes(for: types)
      )
      switch status {
      case .shouldRequest: return "should_request"
      case .unnecessary: return "unnecessary"
      case .unknown: return "unknown"
      @unknown default: return "unknown"
      }
    } catch {
      return "status_failed"
    }
  }

  private static func objectTypes(for definitions: [HealthKitReadTypeDefinition]) -> Set<HKObjectType> {
    Set(definitions.compactMap { definition in
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
    })
  }
}
#endif
