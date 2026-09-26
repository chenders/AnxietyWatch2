import AWDomain
import AWHealthKit
import AWOutbox
import AWTransport
import Combine
import Foundation
import SwiftUI
#if canImport(BackgroundTasks)
import BackgroundTasks
#endif
#if canImport(WatchConnectivity)
import WatchConnectivity
#endif
#if canImport(UserNotifications)
import UserNotifications
#endif
#if canImport(UIKit)
import UIKit
#endif

struct AppProjectionMeasurement: Equatable, Identifiable, Sendable {
  let metric: String
  let label: String
  let unit: String
  let value: Double?
  let observedAt: String?
  let state: String

  var id: String { metric }
}

struct AppJournalMirrorEntry: Codable, Equatable, Identifiable, Sendable {
  let record: DomainRecordPayload
  let syncState: DomainSyncState
  let deleted: Bool

  init(record: DomainRecordPayload, syncState: DomainSyncState, deleted: Bool = false) {
    self.record = record
    self.syncState = syncState
    self.deleted = deleted
  }

  var id: UUID { record.id }

  private enum CodingKeys: String, CodingKey {
    case record, syncState, deleted
  }

  init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    record = try container.decode(DomainRecordPayload.self, forKey: .record)
    syncState = try container.decode(DomainSyncState.self, forKey: .syncState)
    deleted = try container.decodeIfPresent(Bool.self, forKey: .deleted) ?? false
  }
}

actor AppJournalMirrorStore: DomainSyncStateSink {
  private struct Persisted: Codable {
    var entries: [AppJournalMirrorEntry]
  }

  private let fileURL: URL
  private let settingsURL: URL
  private var persisted: Persisted
  private var backupIncluded: Bool

  init(directory: URL, fileName: String = "journal-mirror.json") throws {
    fileURL = directory.appendingPathComponent(fileName, isDirectory: false)
    settingsURL = directory.appendingPathComponent("journal-mirror-settings.json", isDirectory: false)
    backupIncluded = try Self.loadBackupPreference(from: settingsURL)
    do {
      if FileManager.default.fileExists(atPath: fileURL.path) {
        let data = try Data(contentsOf: fileURL)
        let decoder = JSONDecoder()
        persisted = try decoder.decode(Persisted.self, from: data)
      } else {
        persisted = Persisted(entries: [])
      }
      try Self.protect(fileURL, includeInBackup: backupIncluded)
    } catch let error as OutboxError {
      throw error
    } catch {
      throw OutboxError.persistenceFailure("journal mirror load failed: \(error.localizedDescription)")
    }
  }

  func all() -> [AppJournalMirrorEntry] {
    persisted.entries.sorted { $0.record.recordedAt > $1.record.recordedAt }
  }

  func isBackupIncluded() -> Bool { backupIncluded }

  func isBackupChoiceRequired() -> Bool {
    !FileManager.default.fileExists(atPath: settingsURL.path)
  }

  func setBackupIncluded(_ included: Bool) throws {
    let data = try JSONEncoder().encode(included)
    do {
      try data.write(to: settingsURL, options: .atomic)
      try Self.protect(settingsURL, includeInBackup: true)
    } catch let error as OutboxError {
      throw error
    } catch {
      throw OutboxError.persistenceFailure("journal mirror backup preference commit failed")
    }
    backupIncluded = included
    try Self.protect(fileURL, includeInBackup: included)
  }

  func update(recordID: UUID, state: DomainSyncState) throws {
    guard let index = persisted.entries.firstIndex(where: { $0.record.id == recordID }) else { return }
    persisted.entries[index] = AppJournalMirrorEntry(
      record: persisted.entries[index].record,
      syncState: state
    )
    try persist()
  }

  func put(record: DomainRecordPayload, syncState: DomainSyncState) throws {
    if let targetID = record.tombstoneTargetID {
      if let targetIndex = persisted.entries.firstIndex(where: { $0.record.id == targetID }) {
        let target = persisted.entries[targetIndex]
        persisted.entries[targetIndex] = AppJournalMirrorEntry(
          record: target.record,
          syncState: target.syncState,
          deleted: true
        )
      }
      if let tombstoneIndex = persisted.entries.firstIndex(where: { $0.record.id == record.id }) {
        let existing = persisted.entries[tombstoneIndex]
        guard record.disposition(comparedWith: existing.record) != .conflict else {
          throw OutboxError.invalidItem("journal mirror conflict: record \(record.id) has different content")
        }
        persisted.entries[tombstoneIndex] = AppJournalMirrorEntry(
          record: existing.record,
          syncState: syncState,
          deleted: existing.deleted
        )
      } else {
        persisted.entries.append(AppJournalMirrorEntry(record: record, syncState: syncState))
      }
      try persist()
      return
    }

    let alreadyTombstoned = persisted.entries.contains { $0.record.tombstoneTargetID == record.id }
    if let index = persisted.entries.firstIndex(where: { $0.record.id == record.id }) {
      let existing = persisted.entries[index]
      guard record.disposition(comparedWith: existing.record) != .conflict else {
        throw OutboxError.invalidItem("journal mirror conflict: record \(record.id) has different content")
      }
      persisted.entries[index] = AppJournalMirrorEntry(
        record: existing.record,
        syncState: syncState,
        deleted: existing.deleted || alreadyTombstoned
      )
    } else {
      persisted.entries.append(
        AppJournalMirrorEntry(record: record, syncState: syncState, deleted: alreadyTombstoned)
      )
    }
    try persist()
  }

  private static func loadBackupPreference(from url: URL) throws -> Bool {
    guard FileManager.default.fileExists(atPath: url.path) else { return false }
    do {
      return try JSONDecoder().decode(Bool.self, from: Data(contentsOf: url))
    } catch {
      throw OutboxError.persistenceFailure("journal mirror backup preference is unreadable")
    }
  }

  private func persist() throws {
    do {
      let data = try JSONEncoder().encode(persisted)
      try data.write(to: fileURL, options: .atomic)
      try Self.protect(fileURL, includeInBackup: backupIncluded)
    } catch let error as OutboxError {
      throw error
    } catch {
      throw OutboxError.persistenceFailure("journal mirror commit failed: \(error.localizedDescription)")
    }
  }

  private static func protect(_ url: URL, includeInBackup: Bool) throws {
    guard FileManager.default.fileExists(atPath: url.path) else { return }
    #if os(iOS)
    do {
      try FileManager.default.setAttributes(
        [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
        ofItemAtPath: url.path
      )
    } catch {
      throw OutboxError.persistenceFailure("journal mirror Data Protection could not be applied")
    }
    #endif
    do {
      var values = URLResourceValues()
      values.isExcludedFromBackup = !includeInBackup
      var protectedURL = url
      try protectedURL.setResourceValues(values)
    } catch {
      throw OutboxError.persistenceFailure("journal mirror backup exclusion could not be applied")
    }
  }
}

#if canImport(HealthKit)
import HealthKit

private actor AppHealthKitBatchStore: HealthKitBatchStore {
  private let outbox: FileOutboxStore

  init(outbox: FileOutboxStore) {
    self.outbox = outbox
  }

  func anchorData(for typeIdentifier: String) async throws -> Data? {
    await outbox.healthKitAnchor(for: typeIdentifier)
  }

  func commit(
    _ payload: HealthKitEnvelopePayload,
    typeIdentifier: String,
    nextAnchorData: Data?,
    capturedAt: Date
  ) async throws {
    let batchID = payload.samples.first?.queryBatchID ?? payload.deletions.first?.queryBatchID
    let payloadData = try JSONEncoder().encode(payload)
    try await outbox.enqueueHealthKitAndAdvanceAnchor(
      payload: payload.samples.isEmpty && payload.deletions.isEmpty ? nil : payloadData,
      itemID: batchID,
      capturedAt: capturedAt,
      typeIdentifier: typeIdentifier,
      nextAnchorData: nextAnchorData
    )
  }
}
#endif

@MainActor
final class AppClientRuntime: ObservableObject {
  @Published private(set) var isEnrolled = false
  @Published private(set) var syncStatus = "Sync is not initialized."
  @Published private(set) var outboxItemCount = 0
  @Published private(set) var projectionMeasurements: [AppProjectionMeasurement] = []
  @Published private(set) var projectionStatus = "Measurements have not been loaded."
  @Published private(set) var journalBackupIncluded = false
  @Published private(set) var journalBackupChoiceRequired = true
#if canImport(UserNotifications)
  @Published private(set) var notificationStatus = "Notifications are not configured."
  private var apnsTransport: URLSessionAPNsTransport?
  private var apnsOutcomeLedger: APNsOutcomeLedger?
#endif
#if canImport(HealthKit)
  @Published private(set) var healthKitAuthorizationFacts = HealthKitAuthorizationFacts()
  @Published private(set) var healthKitFacts: [HealthKitTypeFacts] = []
#endif
  let songSearchCoordinator: SongSearchCoordinator

  private let credentialStore: KeychainCredentialStore
  private let keyStore: SecureEnclaveKeyStore
  private let initializationError: String?
  private let outbox: FileOutboxStore?
  private let journalMirror: AppJournalMirrorStore?
  private var syncEngine: SyncEngine?
  private var projectionService: ProjectionReadService?
  private var projectionCache: ProjectionCache?
#if canImport(BackgroundTasks)
  private let healthKitDrainTaskIdentifier = "com.groundeffectsoftware.AnxietyWatch.healthkit-drain"
#endif
#if canImport(HealthKit)
  private let healthKitFactsStore: UserDefaultsHealthKitFactsStore
  private let healthKitAuthorizationFactsStore: UserDefaultsHealthKitAuthorizationFactStore
  private var healthKitCoordinator: HealthKitDeliveryCoordinator?
  private var statusTransport: ClientStatusTransport?
  private var registrySHA256 = "unknown"
#endif
#if canImport(WatchConnectivity)
  private let watchRelayReceiver = AppWatchRelayReceiver()
#endif

  init() {
    credentialStore = KeychainCredentialStore()
    keyStore = SecureEnclaveKeyStore()
#if canImport(UserNotifications)
    apnsOutcomeLedger = try? APNsOutcomeLedger()
#endif
#if canImport(HealthKit)
    healthKitFactsStore = UserDefaultsHealthKitFactsStore()
    healthKitAuthorizationFactsStore = UserDefaultsHealthKitAuthorizationFactStore()
#endif
    songSearchCoordinator = SongSearchCoordinator(
      geniusProxy: nil,
      geniusDirect: nil,
      itunes: URLSessionITunesSongSearchProvider()
    )
    var loadedOutbox: FileOutboxStore?
    var loadedJournalMirror: AppJournalMirrorStore?
    var loadError: String?
    do {
      guard let support = FileManager.default.urls(
        for: .applicationSupportDirectory,
        in: .userDomainMask
      ).first else {
        throw OutboxError.persistenceFailure("application support directory is unavailable")
      }
      let directory = support.appendingPathComponent("AnxietyWatch", isDirectory: true)
      projectionCache = try ProjectionCache(
        persistenceURL: directory.appendingPathComponent("projection-cache.json", isDirectory: false)
      )
      let localOutbox = try FileOutboxStore(directory: directory)
      let localJournalMirror = try AppJournalMirrorStore(directory: directory)
      loadedOutbox = localOutbox
      loadedJournalMirror = localJournalMirror
    } catch {
      loadError = error.localizedDescription
    }
    outbox = loadedOutbox
    journalMirror = loadedJournalMirror
    initializationError = loadError
#if canImport(BackgroundTasks)
    BGTaskScheduler.shared.register(forTaskWithIdentifier: healthKitDrainTaskIdentifier, using: nil) { [weak self] task in
      guard let task = task as? BGAppRefreshTask else { return }
      task.expirationHandler = { task.setTaskCompleted(success: false) }
      Task { @MainActor [weak self] in
        await self?.drain()
        task.setTaskCompleted(success: true)
      }
    }
#endif
  }

  func makeJournalCaptureCoordinator() throws -> JournalCaptureCoordinator {
    guard let outbox, let journalMirror else {
      throw OutboxError.persistenceFailure(
        "journal storage is unavailable: \(initializationError ?? "unknown initialization failure")"
      )
    }
    return JournalCaptureCoordinator(
      sink: AppDomainCommandSink(outbox: outbox, journalMirror: journalMirror),
      drainTrigger: AppImmediateDrainTrigger()
    )
  }

  func reload() async {
    guard let outbox, let journalMirror else {
      isEnrolled = false
      syncEngine = nil
      projectionService = nil
#if canImport(UserNotifications)
      apnsTransport = nil
#endif
#if canImport(HealthKit)
      healthKitCoordinator = nil
      statusTransport = nil
#endif
      syncStatus = "Sync unavailable: \(initializationError ?? "journal storage initialization failed")"
      return
    }
    do {
      guard let credential = try credentialStore.load() else {
        isEnrolled = false
        syncEngine = nil
        projectionService = nil
#if canImport(UserNotifications)
        apnsTransport = nil
#endif
#if canImport(HealthKit)
        healthKitCoordinator = nil
        statusTransport = nil
#endif
        syncStatus = "Sync unavailable: this iPhone is not enrolled."
        return
      }
      let authenticator = RequestAuthenticator(keyStore: keyStore, origin: credential.origin)
#if canImport(UserNotifications)
      apnsTransport = URLSessionAPNsTransport(origin: credential.origin, authenticator: authenticator)
#endif
#if canImport(HealthKit)
      if let previousHealthKitCoordinator = healthKitCoordinator {
        await previousHealthKitCoordinator.stopObservers()
      }
#endif
      let writeTransport = URLSessionClientItemTransport(
        origin: credential.origin,
        authenticator: authenticator
      )
      syncEngine = SyncEngine(
        outbox: outbox,
        credentialStore: credentialStore,
        transport: writeTransport,
        stateSink: journalMirror
      )
      guard let cache = projectionCache else {
        throw OutboxError.persistenceFailure("projection cache initialization failed")
      }
      let readTransport = URLSessionProjectionTransport(
        origin: credential.origin,
        authenticator: authenticator
      )
      projectionService = ProjectionReadService(
        endpoint: credential.origin.normalizedString,
        cache: cache,
        credentialStore: credentialStore,
        transport: readTransport
      )
#if canImport(HealthKit)
      statusTransport = URLSessionClientStatusTransport(origin: credential.origin, authenticator: authenticator)
      do {
        let registryResponse = try await URLSessionRegistryTransport(
          origin: credential.origin,
          authenticator: authenticator
        ).readRegistry(credential: credential)
        registrySHA256 = registryResponse.registrySHA256
        let coordinator = try HealthKitDeliveryCoordinator(
          registry: registryResponse.document,
          batchStore: AppHealthKitBatchStore(outbox: outbox),
          factsStore: healthKitFactsStore,
          authorizationFactsStore: healthKitAuthorizationFactsStore,
          onCommit: { [weak self] _ in
            NotificationCenter.default.post(name: .domainOutboxNeedsDrain, object: nil)
#if canImport(BackgroundTasks)
            Task { @MainActor [weak self] in self?.scheduleHealthKitDrain() }
#endif
          }
        )
        await coordinator.load()
        await coordinator.startObservers()
        healthKitCoordinator = coordinator
        await refreshHealthKitState()
      } catch {
        healthKitCoordinator = nil
        healthKitFacts = []
        healthKitAuthorizationFacts = (try? await healthKitAuthorizationFactsStore.loadAuthorizationFacts()) ?? HealthKitAuthorizationFacts()
      }
#endif
      isEnrolled = true
      await refreshProjectionMeasurements()
#if canImport(UserNotifications)
      await refreshNotificationRegistration()
#endif
      syncStatus = "Enrolled; waiting for a queued write or Sync Now."
    } catch {
      isEnrolled = false
      syncEngine = nil
      projectionService = nil
#if canImport(UserNotifications)
      apnsTransport = nil
#endif
#if canImport(HealthKit)
      healthKitCoordinator = nil
      statusTransport = nil
#endif
      syncStatus = "Sync unavailable: credential setup failed."
    }
  }

  func observeDrainRequests() async {
    await reload()
    await reconcileJournalMirror()
    await refreshOutboxState()
    await refreshJournalBackupSetting()
#if canImport(WatchConnectivity)
    watchRelayReceiver.activate(runtime: self)
#endif
    for await _ in NotificationCenter.default.notifications(named: .domainOutboxNeedsDrain) {
      await drain()
    }
  }

  func refreshOutboxState() async {
    guard let outbox else {
      outboxItemCount = 0
      return
    }
    outboxItemCount = (try? await outbox.snapshot().pendingCount) ?? 0
  }

  /// Fetch the small Today surface from Pi-owned projections. The phone only renders values returned
  /// by the server; it does not aggregate, interpolate, or retain a health archive.
  func refreshProjectionMeasurements() async {
    let definitions = [
      (metric: "hr", label: "Heart rate", unit: "bpm"),
      (metric: "spo2", label: "Oxygen saturation", unit: "%"),
      (metric: "respiratory_rate", label: "Respiratory rate", unit: "breaths/min"),
      (metric: "resting_hr", label: "Resting heart rate", unit: "bpm"),
    ]
    guard isEnrolled, let projectionService else {
      projectionMeasurements = []
      projectionStatus = isEnrolled
        ? "Measurements unavailable: projection transport is not initialized."
        : "Measurements unavailable: this iPhone is not enrolled."
      return
    }

    var loaded: [AppProjectionMeasurement] = []
    for definition in definitions {
      let query = ClientProjectionQuery(
        metric: definition.metric,
        windowStart: nil,
        windowEnd: nil,
        cursor: nil
      )
      let state = await projectionService.read(query: query)
      switch state {
      case let .live(page, _):
        let point = page.points.last(where: { $0.value != nil })
        loaded.append(
          AppProjectionMeasurement(
            metric: definition.metric,
            label: definition.label,
            unit: definition.unit,
            value: point?.value,
            observedAt: page.observedAt ?? point?.timestamp,
            state: point == nil ? "not measured" : "live"
          )
        )
      case let .stale(page, _, reason):
        let point = page.points.last(where: { $0.value != nil })
        loaded.append(
          AppProjectionMeasurement(
            metric: definition.metric,
            label: definition.label,
            unit: definition.unit,
            value: point?.value,
            observedAt: page.observedAt ?? point?.timestamp,
            state: point == nil ? "not measured" : "stale: \(reason)"
          )
        )
      case let .unknown(reason):
        loaded.append(
          AppProjectionMeasurement(
            metric: definition.metric,
            label: definition.label,
            unit: definition.unit,
            value: nil,
            observedAt: nil,
            state: "not measured: \(reason)"
          )
        )
      }
    }
    projectionMeasurements = loaded
    projectionStatus = loaded.contains(where: { $0.value != nil })
      ? "Pi projections refreshed."
      : "No requested measurements have been recorded."
  }

#if canImport(BackgroundTasks)
  func scheduleHealthKitDrain() {
    let request = BGAppRefreshTaskRequest(identifier: healthKitDrainTaskIdentifier)
    request.earliestBeginDate = Date(timeIntervalSinceNow: 1)
    try? BGTaskScheduler.shared.submit(request)
  }
#endif

#if canImport(UserNotifications)
  /// Presents the one-time rationale in Settings, then registers authorization facts and the APNs
  /// token through the authenticated Pi API. No protected content is placed in the push payload.
  /// Remove the enrolled credential and its Secure Enclave signing key without deleting local journal data.
  func unenroll() async {
    do {
      if let credential = try credentialStore.load() {
        try keyStore.deleteKey(credential.keyReference)
      }
      try credentialStore.delete()
      await reload()
    } catch {
      syncStatus = "Unenrollment failed: secure credential cleanup did not complete."
    }
  }

  func requestNotifications() async {
    guard isEnrolled else {
      notificationStatus = "Notification setup blocked: this iPhone is not enrolled."
      return
    }
    let center = UNUserNotificationCenter.current()
    let granted: Bool
    do {
      granted = try await center.requestAuthorization(options: [.alert, .sound, .badge])
    } catch {
      notificationStatus = "Notification setup failed: authorization request could not complete."
      return
    }
    await registerAPNsFacts()
    #if canImport(UIKit)
    UIApplication.shared.registerForRemoteNotifications()
    #endif
    notificationStatus = granted
      ? "Notification permission requested; waiting for APNs registration."
      : "Notifications are denied; operator messages will not be displayed."
  }

  func handleAPNsToken(_ tokenData: Data) async {
    await registerAPNsFacts(token: tokenData.map { String(format: "%02x", $0) }.joined())
  }

  private func refreshNotificationRegistration() async {
    let settings = await UNUserNotificationCenter.current().notificationSettings()
    guard settings.authorizationStatus == .authorized
      || settings.authorizationStatus == .provisional
      || settings.authorizationStatus == .ephemeral
    else { return }
    await registerAPNsFacts()
    #if canImport(UIKit)
    UIApplication.shared.registerForRemoteNotifications()
    #endif
  }

  func handleAPNsRegistrationFailure() async {
    await registerAPNsFacts()
    notificationStatus = "APNs registration failed: the token is unavailable; authorization is not delivery evidence."
  }

  func recordAPNsOutcome(for message: APNsMessage, outcome: APNsMessageOutcome) async {
    guard let ledger = apnsOutcomeLedger,
      await ledger.shouldReport(messageID: message.messageID, outcome: outcome)
    else { return }
    guard let transport = apnsTransport else {
      notificationStatus = "Notification outcome blocked: authenticated Pi transport is unavailable."
      return
    }
    guard let credential = try? credentialStore.load() else {
      notificationStatus = "Notification outcome blocked: enrolled credential is unavailable."
      return
    }
    let formatter = ISO8601DateFormatter()
    do {
      let request = try APNsOutcomeRequest(
        messageID: message.messageID,
        outcome: outcome,
        observedAt: formatter.string(from: Date())
      )
      _ = try await transport.report(outcome: request, credential: credential)
      await ledger.markReported(messageID: message.messageID, outcome: outcome)
      notificationStatus = outcome == .displayed
        ? "Operator message displayed; awaiting acknowledgement if selected."
        : "Operator message acknowledgement recorded."
    } catch {
      notificationStatus = "Notification outcome retryable: Pi acknowledgement could not be recorded."
    }
  }

  private func registerAPNsFacts(token: String? = nil) async {
    guard let transport = apnsTransport else {
      notificationStatus = "Notification setup blocked: authenticated Pi transport is unavailable."
      return
    }
    guard let credential = try? credentialStore.load() else {
      notificationStatus = "Notification setup blocked: enrolled credential is unavailable."
      return
    }
    let settings = await UNUserNotificationCenter.current().notificationSettings()
    let status: String
    switch settings.authorizationStatus {
    case .authorized: status = "authorized"
    case .denied: status = "denied"
    case .provisional: status = "provisional"
    case .ephemeral: status = "ephemeral"
    case .notDetermined: status = "not_determined"
    @unknown default: status = "unknown"
    }
    let info = Bundle.main.infoDictionary ?? [:]
    let appVersion = info["CFBundleShortVersionString"] as? String ?? "unknown"
    let build = info["CFBundleVersion"] as? String ?? "unknown"
    #if DEBUG
    let environment = "sandbox"
    #else
    let environment = "production"
    #endif
    do {
      let request = try APNsRegistrationRequest(
        deviceID: credential.clientID,
        token: token,
        environment: environment,
        appIdentifier: Bundle.main.bundleIdentifier ?? "unknown",
        appVersion: appVersion,
        build: build,
        authorizationStatus: status,
        alertsEnabled: settings.alertSetting == .enabled,
        soundsEnabled: settings.soundSetting == .enabled,
        badgesEnabled: settings.badgeSetting == .enabled
      )
      let response = try await transport.register(request: request, credential: credential)
      notificationStatus = response.status == "accepted"
        ? "Notification registration acknowledged by Pi; APNs delivery remains unknown."
        : "Notification registration state: \(response.status)."
    } catch {
      notificationStatus = "Notification registration failed: \(String(describing: error))"
    }
  }
#endif

#if canImport(HealthKit)
  func refreshHealthKitState() async {
    guard let coordinator = healthKitCoordinator else {
      healthKitFacts = []
      healthKitAuthorizationFacts = (try? await healthKitAuthorizationFactsStore.loadAuthorizationFacts()) ?? HealthKitAuthorizationFacts()
      return
    }
    healthKitFacts = await coordinator.facts()
    healthKitAuthorizationFacts = await coordinator.authorizationFacts()
  }

  func requestHealthKitAccess() async {
    guard let coordinator = healthKitCoordinator else {
      syncStatus = "HealthKit unavailable: the Pi registry is not available."
      return
    }
    let authorizationFacts = await coordinator.requestAuthorization()
    healthKitAuthorizationFacts = authorizationFacts
    await coordinator.startObservers()
    await coordinator.syncAll()
    await refreshHealthKitState()
    await drain()
  }

  func sendStatusBeacon() async {
    guard let statusTransport else { return }
    let loadedCredential: StoredDeviceCredential?
    do {
      loadedCredential = try credentialStore.load()
    } catch {
      return
    }
    guard let credential = loadedCredential else { return }
    guard let outbox else { return }
    let snapshot = try? await outbox.snapshot()
    let formatter = ISO8601DateFormatter()
    let assertedAt = Date()
    let factChangeDates = healthKitFacts.flatMap { [$0.lastQueryAt, $0.lastMatchingSampleAt, $0.lastPiAckAt].compactMap { $0 } }
      + [healthKitAuthorizationFacts.attemptedAt].compactMap { $0 }
    let changedAt = factChangeDates.max() ?? assertedAt
    let beacon = HealthBeaconRequest(
      beaconID: UUID(),
      assertedAt: formatter.string(from: assertedAt),
      changedAt: formatter.string(from: min(changedAt, assertedAt)),
      appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "unknown",
      registrySHA256: registrySHA256,
      outboxCount: UInt64(snapshot?.pendingCount ?? 0),
      oldestOutboxAgeS: snapshot?.oldestPendingAt.map { UInt64(max(0, Date().timeIntervalSince($0))) },
      lastServerAckAt: nil,
      types: healthKitFacts.map { facts in
        let formatter = ISO8601DateFormatter()
        return HealthTypeFact(
          healthKitType: facts.healthKitType,
          configured: facts.configured,
          authorizationRequestOutcome: facts.authorizationRequestOutcome,
          observerRegistrationOutcome: facts.observerRegistrationOutcome,
          backgroundDeliveryOutcome: facts.backgroundDeliveryOutcome,
          lastQueryOutcome: facts.lastQueryOutcome,
          lastQueryAt: facts.lastQueryAt.map(formatter.string),
          lastMatchingSampleAt: facts.lastMatchingSampleAt.map(formatter.string),
          lastPiAckAt: facts.lastPiAckAt.map(formatter.string)
        )
      }
    )
    _ = try? await statusTransport.send(beacon: beacon, credential: credential)
  }
#endif

  func reconcileJournalMirror() async {
    guard let outbox, let journalMirror else { return }
    do {
      for item in try await outbox.allItems()
      where item.recordClass == "journal_entry" || item.recordClass == "tombstone" {
        let envelope = try JSONDecoder().decode(DomainEnvelope.self, from: item.payload)
        let state: DomainSyncState
        switch item.state {
        case .pending, .retryable: state = .queued
        case .rejected: state = .unavailable
        case .conflict: state = .conflict
        case .acknowledged: state = .acknowledged
        }
        try await journalMirror.put(record: envelope.record, syncState: state)
      }
    } catch {
      syncStatus = "Journal mirror reconciliation blocked: \(error)"
    }
  }

  func setWatchRelayStatus(_ status: String) {
    syncStatus = status
  }

#if canImport(HealthKit)
  private func healthKitType(for item: OutboxItem) -> String? {
    guard item.recordClass == "platform_sample_batch",
      let payload = try? JSONDecoder().decode(HealthKitEnvelopePayload.self, from: item.payload)
    else { return nil }
    return payload.samples.first?.healthKitType.rawValue ?? payload.deletions.first?.healthKitType.rawValue
  }
#endif

  private func watchRelayIDs(in items: [OutboxItem]) -> Set<UUID> {
    Set(items.compactMap { item in
      guard item.recordClass == "journal_entry",
        let envelope = try? JSONDecoder().decode(DomainEnvelope.self, from: item.payload),
        case .object(let content) = envelope.record.content,
        case .bool(true) = content["watch_relay"]
      else { return nil }
      return envelope.record.id
    })
  }

#if canImport(WatchConnectivity)
  private func notifyWatchRelayServerAcknowledgements(
    for ids: Set<UUID>,
    outbox: FileOutboxStore
  ) async {
    guard let remaining = try? await outbox.allItems() else { return }
    let remainingIDs = watchRelayIDs(in: remaining)
    for id in ids where !remainingIDs.contains(id) {
      watchRelayReceiver.notifyServerAcknowledged(id: id)
    }
  }
#endif

  func acceptWatchQuickLog(_ item: WatchQuickLog) async throws {
    guard let outbox, let journalMirror else {
      throw OutboxError.persistenceFailure("watch relay blocked: journal storage is unavailable")
    }
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    guard let capturedAt = formatter.date(from: item.capturedAt) else {
      throw OutboxError.invalidItem("watch relay blocked: captured time is not valid UTC")
    }
    let record = try item.record()
    let payload = try JSONEncoder().encode(DomainEnvelope(record: record))
    if let existing = try await outbox.allItems().first(where: { $0.id == item.id }) {
      guard existing.recordClass == record.kind,
        existing.schemaVersion == record.schemaVersion,
        let envelope = try? JSONDecoder().decode(DomainEnvelope.self, from: existing.payload),
        envelope.record.disposition(comparedWith: record) == .replay
      else {
        throw OutboxError.invalidItem("watch relay blocked: record \(item.id) conflicts with queued content")
      }
      return
    }
    let outboxItem = try OutboxItem(
      id: record.id,
      recordClass: record.kind,
      schemaVersion: record.schemaVersion,
      payload: payload,
      capturedAt: capturedAt
    )
    try await outbox.enqueue(outboxItem)
    try await journalMirror.put(record: record, syncState: .queued)
    NotificationCenter.default.post(name: .domainOutboxNeedsDrain, object: nil)
  }

  func journalMirrorEntries() async throws -> [AppJournalMirrorEntry] {
    guard let journalMirror else {
      throw OutboxError.persistenceFailure(
        "journal mirror is unavailable: \(initializationError ?? "unknown initialization failure")"
      )
    }
    return await journalMirror.all()
  }

  func refreshJournalBackupSetting() async {
    guard let journalMirror else {
      journalBackupIncluded = false
      journalBackupChoiceRequired = true
      return
    }
    journalBackupIncluded = await journalMirror.isBackupIncluded()
    journalBackupChoiceRequired = await journalMirror.isBackupChoiceRequired()
  }

  func setJournalBackupIncluded(_ included: Bool) async {
    guard let journalMirror else {
      syncStatus = "Journal backup setting blocked: mirror storage is unavailable."
      return
    }
    do {
      try await journalMirror.setBackupIncluded(included)
      journalBackupIncluded = included
      journalBackupChoiceRequired = false
    } catch {
      syncStatus = "Journal backup setting blocked: \(error)"
    }
  }

  func drain() async {
    guard let syncEngine, let outbox else {
      syncStatus = isEnrolled
        ? "Sync blocked: the enrolled transport is not initialized."
        : "Sync unavailable: this iPhone is not enrolled."
      return
    }
    let itemsBeforeDrain = (try? await outbox.allItems()) ?? []
    let watchRelayIDsBeforeDrain = watchRelayIDs(in: itemsBeforeDrain)
#if canImport(HealthKit)
    let healthKitBatchesBeforeDrain = Dictionary(uniqueKeysWithValues: itemsBeforeDrain.compactMap { item in
      healthKitType(for: item).map { (item.id, $0) }
    })
#endif
    do {
      let report = try await syncEngine.drain()
#if canImport(HealthKit)
      if let coordinator = healthKitCoordinator {
        let remainingIDs = Set(((try? await outbox.allItems()) ?? []).map(\.id))
        for (itemID, typeIdentifier) in healthKitBatchesBeforeDrain where !remainingIDs.contains(itemID) {
          await coordinator.recordPiAcknowledgement(for: typeIdentifier, at: Date())
        }
        await refreshHealthKitState()
      }
#endif
#if canImport(WatchConnectivity)
      await notifyWatchRelayServerAcknowledgements(for: watchRelayIDsBeforeDrain, outbox: outbox)
#endif
      outboxItemCount = report.pendingAfter
      await refreshProjectionMeasurements()
      syncStatus = "Sync complete: \(report.acknowledged) acknowledged, "
        + "\(report.replayed) replayed, \(report.rejected) rejected, "
        + "\(report.conflicts) conflicts, \(report.retryable) retryable; "
        + "\(report.pendingAfter) pending."
#if canImport(HealthKit)
      await sendStatusBeacon()
#endif
    } catch let error as SyncError {
      await refreshOutboxState()
      if case .drainAlreadyOwned = error {
        syncStatus = "Sync already in progress; queued items remain safe."
      } else {
        syncStatus = "Sync blocked: \(error)"
      }
    } catch {
      await refreshOutboxState()
      syncStatus = "Sync blocked: \(error)"
    }
  }
}

#if canImport(UserNotifications) && canImport(UIKit)
private final class NotificationAppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
  nonisolated(unsafe) weak var runtime: AppClientRuntime?
  private var pendingOutcomes: [(APNsMessage, APNsMessageOutcome)] = []

  @MainActor
  func attach(runtime: AppClientRuntime) {
    self.runtime = runtime
    let pending = pendingOutcomes
    pendingOutcomes.removeAll()
    for (message, outcome) in pending {
      Task { @MainActor in
        await runtime.recordAPNsOutcome(for: message, outcome: outcome)
      }
    }
  }

  @MainActor
  private func recordOutcome(_ message: APNsMessage, _ outcome: APNsMessageOutcome) async {
    guard let runtime else {
      pendingOutcomes.append((message, outcome))
      return
    }
    await runtime.recordAPNsOutcome(for: message, outcome: outcome)
  }

  func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
  ) -> Bool {
    UNUserNotificationCenter.current().delegate = self
    return true
  }

  func application(_ application: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data) {
    Task { @MainActor [weak self] in
      await self?.runtime?.handleAPNsToken(deviceToken)
    }
  }

  func application(_ application: UIApplication, didFailToRegisterForRemoteNotificationsWithError error: any Error) {
    Task { @MainActor [weak self] in
      await self?.runtime?.handleAPNsRegistrationFailure()
    }
  }

  nonisolated func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    willPresent notification: UNNotification,
    withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
  ) {
    guard let message = try? APNsMessage(userInfo: notification.request.content.userInfo) else {
      completionHandler([])
      return
    }
    Task { @MainActor in
      await self.recordOutcome(message, .displayed)
    }
    completionHandler([.banner, .sound, .badge])
  }

  nonisolated func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    didReceive response: UNNotificationResponse,
    withCompletionHandler completionHandler: @escaping () -> Void
  ) {
    guard let message = try? APNsMessage(userInfo: response.notification.request.content.userInfo) else {
      completionHandler()
      return
    }
    // The payload carries only a protected category/deep link. The eventual protected view is
    // fetched by the enrolled client after the device is unlocked.
    Task { @MainActor in
      await self.recordOutcome(message, .acknowledged)
    }
    completionHandler()
  }
}
#endif

#if canImport(WatchConnectivity)
private final class WatchRelayReply: @unchecked Sendable {
  private let handler: ([String: Any]) -> Void

  init(_ handler: @escaping ([String: Any]) -> Void) {
    self.handler = handler
  }

  func accept(id: UUID) {
    handler(["accepted": true, "id": id.uuidString.lowercased()])
  }

  func reject() {
    handler(["accepted": false])
  }
}

@MainActor
private final class AppWatchRelayReceiver: NSObject, WCSessionDelegate {
  private weak var runtime: AppClientRuntime?

  func activate(runtime: AppClientRuntime) {
    self.runtime = runtime
    guard WCSession.isSupported() else { return }
    WCSession.default.delegate = self
    WCSession.default.activate()
  }

  func notifyServerAcknowledged(id: UUID) {
    guard WCSession.isSupported() else { return }
    WCSession.default.transferUserInfo([
      "type": "watch_quick_log_ack",
      "id": id.uuidString.lowercased(),
      "status": "server_acknowledged",
      "accepted": true
    ])
  }

  nonisolated func session(
    _ session: WCSession,
    activationDidCompleteWith activationState: WCSessionActivationState,
    error: (any Error)?
  ) {}

  nonisolated func sessionDidBecomeInactive(_ session: WCSession) {}

  nonisolated func sessionDidDeactivate(_ session: WCSession) {
    session.activate()
  }

  nonisolated func session(_ session: WCSession, didReceiveUserInfo userInfo: [String: Any] = [:]) {
    guard let item = Self.decode(userInfo) else { return }
    Task { @MainActor [weak self] in
      await self?.receive(item, reply: nil)
    }
  }

  nonisolated func session(
    _ session: WCSession,
    didReceiveMessage message: [String: Any],
    replyHandler: @escaping ([String: Any]) -> Void
  ) {
    let reply = WatchRelayReply(replyHandler)
    guard let item = Self.decode(message) else {
      reply.reject()
      return
    }
    Task { @MainActor [weak self] in
      await self?.receive(item, reply: reply)
    }
  }

  nonisolated private static func decode(_ message: [String: Any]) -> WatchQuickLog? {
    guard message["type"] as? String == "watch_quick_log",
      let rawID = message["id"] as? String,
      let id = UUID(uuidString: rawID),
      let capturedAt = message["captured_at"] as? String,
      let rawAnxiety = message["anxiety"] as? Int,
      let anxiety = UInt8(exactly: rawAnxiety)
    else { return nil }
    return try? WatchQuickLog(id: id, capturedAt: capturedAt, anxietyRawValue: anxiety)
  }

  private func receive(_ item: WatchQuickLog, reply: WatchRelayReply?) async {
    guard let runtime else {
      reply?.reject()
      return
    }
    do {
      try await runtime.acceptWatchQuickLog(item)
      runtime.setWatchRelayStatus("Watch Quick Log queued on iPhone; waiting for Pi acknowledgement.")
      reply?.accept(id: item.id)
      if reply == nil {
        WCSession.default.transferUserInfo([
          "type": "watch_quick_log_ack",
          "id": item.id.uuidString.lowercased(),
          "accepted": true
        ])
      }
    } catch {
      runtime.setWatchRelayStatus("Watch Quick Log blocked: durable iPhone enqueue failed.")
      reply?.reject()
    }
  }
}
#endif

@main
struct AnxietyWatchApp: App {
  @StateObject private var runtime = AppClientRuntime()
#if canImport(UserNotifications) && canImport(UIKit)
  @UIApplicationDelegateAdaptor(NotificationAppDelegate.self) private var notificationDelegate
#endif

  var body: some Scene {
    WindowGroup {
      ClientRootView()
        .environmentObject(runtime)
        .onAppear {
#if canImport(UserNotifications) && canImport(UIKit)
          notificationDelegate.attach(runtime: runtime)
#endif
        }
    }
  }
}
