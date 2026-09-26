import Foundation

/// Independent facts for one HealthKit type. None of these fields claims that Apple exposed a read
/// grant; the only evidence of a sample is an actual query result.
public struct HealthKitTypeFacts: Codable, Equatable, Sendable {
  public let healthKitType: String
  public var configured: Bool
  public var authorizationRequestOutcome: String?
  public var observerRegistrationOutcome: String?
  public var backgroundDeliveryOutcome: String?
  public var lastQueryOutcome: String?
  public var lastQueryAt: Date?
  public var lastMatchingSampleAt: Date?
  public var lastPiAckAt: Date?

  public init(healthKitType: String, configured: Bool = true) {
    self.healthKitType = healthKitType
    self.configured = configured
    authorizationRequestOutcome = nil
    observerRegistrationOutcome = nil
    backgroundDeliveryOutcome = nil
    lastQueryOutcome = nil
    lastQueryAt = nil
    lastMatchingSampleAt = nil
    lastPiAckAt = nil
  }
}

/// Persisted per-type facts used by the Health access screen and the measurement-free status beacon.
public protocol HealthKitFactsStore: Sendable {
  func loadHealthKitFacts() async throws -> [HealthKitTypeFacts]
  func saveHealthKitFacts(_ facts: [HealthKitTypeFacts]) async throws
}

/// In-memory fixture store for tests and previews. The app supplies a Data-Protected file-backed
/// implementation; no HealthKit value is stored in this state.
public actor InMemoryHealthKitFactsStore: HealthKitFactsStore {
  private var values: [HealthKitTypeFacts]

  public init(_ values: [HealthKitTypeFacts] = []) {
    self.values = values
  }

  public func loadHealthKitFacts() async throws -> [HealthKitTypeFacts] { values }

  public func saveHealthKitFacts(_ facts: [HealthKitTypeFacts]) async throws {
    values = facts
  }
}

/// A small operational-facts store. It contains timestamps and named outcomes only, never HealthKit
/// values or journal content. Production may replace it with an app-group store without changing
/// the acquisition contract.
public final class UserDefaultsHealthKitFactsStore: HealthKitFactsStore, @unchecked Sendable {
  private let defaults: UserDefaults
  private let key: String

  public init(defaults: UserDefaults = .standard, key: String = "healthkit.operational-facts.v1") {
    self.defaults = defaults
    self.key = key
  }

  public func loadHealthKitFacts() async throws -> [HealthKitTypeFacts] {
    guard let data = defaults.data(forKey: key) else { return [] }
    return try JSONDecoder().decode([HealthKitTypeFacts].self, from: data)
  }

  public func saveHealthKitFacts(_ facts: [HealthKitTypeFacts]) async throws {
    defaults.set(try JSONEncoder().encode(facts), forKey: key)
  }
}

#if canImport(HealthKit)
import HealthKit

/// Coordinates authorization, observer wakeups, bounded anchored reads, and factual per-type
/// evidence. An observer callback only schedules `sync`; it never performs network or unbounded work.
public actor HealthKitDeliveryCoordinator {
  private let definitions: [HealthKitReadTypeDefinition]
  private let queryEngine: HealthKitAnchoredQueryEngine
  private let batchStore: any HealthKitBatchStore
  private let factsStore: any HealthKitFactsStore
  private let authorization: HealthKitAuthorizationCoordinator
  private let observers: HealthKitObserverCoordinator
  private let now: @Sendable () -> Date
  private let onCommit: (@Sendable (String) -> Void)?
  private var factsByType: [String: HealthKitTypeFacts]

  public init(
    registry: RegistryDocument,
    batchStore: any HealthKitBatchStore,
    factsStore: any HealthKitFactsStore,
    authorizationFactsStore: any HealthKitAuthorizationFactStore,
    authorizationClient: any HealthKitAuthorizationRequesting = SystemHealthKitAuthorizationClient(),
    healthStore: HKHealthStore = HKHealthStore(),
    now: @escaping @Sendable () -> Date = { Date() },
    onCommit: (@Sendable (String) -> Void)? = nil
  ) throws {
    let definitions = HealthKitReadPlan.firstRelease.filter {
      registry.source(named: "healthkit")?.provides.contains($0.registryName) == true
    }
    self.definitions = definitions
    queryEngine = try HealthKitAnchoredQueryEngine(healthStore: healthStore, registry: registry)
    self.batchStore = batchStore
    self.factsStore = factsStore
    authorization = HealthKitAuthorizationCoordinator(client: authorizationClient, factsStore: authorizationFactsStore, now: now)
    observers = HealthKitObserverCoordinator(healthStore: healthStore)
    self.now = now
    self.onCommit = onCommit
    factsByType = [:]
  }

  public func load() async {
    let persisted = (try? await factsStore.loadHealthKitFacts()) ?? []
    factsByType = Dictionary(uniqueKeysWithValues: definitions.map { definition in
      let existing = persisted.first { $0.healthKitType == definition.identifier }
      return (definition.identifier, existing ?? HealthKitTypeFacts(healthKitType: definition.identifier))
    })
  }

  public func authorizationFacts() async -> HealthKitAuthorizationFacts {
    await authorization.facts()
  }

  public func requestAuthorization(force: Bool = false) async -> HealthKitAuthorizationFacts {
    let result = await authorization.requestIfNeeded(types: definitions, force: force)
    for identifier in definitions.map(\.identifier) {
      update(identifier) { facts in
        facts.authorizationRequestOutcome = result.outcome
      }
    }
    await persistFacts()
    return result
  }

  public func startObservers() async {
    observers.start(
      definitions: definitions,
      onUpdate: { [weak self] identifier in
        Task { await self?.sync(identifier: identifier) }
      },
      onRegistration: { [weak self] identifier, outcome, detail in
        Task { await self?.recordRegistration(identifier: identifier, outcome: "\(outcome): \(detail)") }
      }
    )
  }

  public func syncAll() async {
    for identifier in definitions.map(\.identifier) {
      await sync(identifier: identifier)
    }
  }

  public func stopObservers() {
    observers.stop()
  }

  public func facts() -> [HealthKitTypeFacts] {
    definitions.compactMap { factsByType[$0.identifier] }
  }

  public func recordPiAcknowledgement(for typeIdentifier: String, at date: Date) async {
    update(typeIdentifier) { facts in facts.lastPiAckAt = date }
    await persistFacts()
  }

  private func sync(identifier: String) async {
    guard let definition = definitions.first(where: { $0.identifier == identifier }) else { return }
    do {
      let page = try await queryEngine.readAndCommit(definition, store: batchStore, now: now())
      update(identifier) { facts in
        facts.lastQueryOutcome = "succeeded"
        facts.lastQueryAt = page.queriedAt
        let sampleDates = page.payload.samples.compactMap { ISO8601DateFormatter().date(from: $0.endAt) }
        facts.lastMatchingSampleAt = sampleDates.max() ?? facts.lastMatchingSampleAt
      }
      onCommit?(identifier)
    } catch {
      update(identifier) { facts in
        facts.lastQueryOutcome = "failed: \(error.localizedDescription)"
        facts.lastQueryAt = now()
      }
    }
    await persistFacts()
  }

  private func recordRegistration(identifier: String, outcome: String) {
    guard definitions.contains(where: { $0.identifier == identifier }) else { return }
    update(identifier) { facts in
      if outcome.hasPrefix("enabled") || outcome.hasPrefix("failed") {
        facts.backgroundDeliveryOutcome = outcome
      } else {
        facts.observerRegistrationOutcome = outcome
      }
    }
    Task { await persistFacts() }
  }

  private func update(_ identifier: String, change: (inout HealthKitTypeFacts) -> Void) {
    guard var facts = factsByType[identifier] else { return }
    change(&facts)
    factsByType[identifier] = facts
  }

  private func persistFacts() async {
    try? await factsStore.saveHealthKitFacts(facts())
  }
}
#endif
