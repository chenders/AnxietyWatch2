import AWDomain
import AWOutbox
import AWTransport
import Combine
import Foundation
import SwiftUI
#if canImport(WatchConnectivity)
import WatchConnectivity
#endif

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

@MainActor
final class AppClientRuntime: ObservableObject {
  @Published private(set) var isEnrolled = false
  @Published private(set) var syncStatus = "Sync is not initialized."
  @Published private(set) var outboxItemCount = 0
  @Published private(set) var journalBackupIncluded = false
  @Published private(set) var journalBackupChoiceRequired = true
  let songSearchCoordinator: SongSearchCoordinator

  private let credentialStore: KeychainCredentialStore
  private let keyStore: SecureEnclaveKeyStore
  private let initializationError: String?
  private let outbox: FileOutboxStore?
  private let journalMirror: AppJournalMirrorStore?
  private var syncEngine: SyncEngine?
  private var projectionService: ProjectionReadService?
#if canImport(WatchConnectivity)
  private let watchRelayReceiver = AppWatchRelayReceiver()
#endif

  init() {
    credentialStore = KeychainCredentialStore()
    keyStore = SecureEnclaveKeyStore()
    songSearchCoordinator = SongSearchCoordinator(
      geniusProxy: nil,
      geniusDirect: nil,
      itunes: URLSessionITunesSongSearchProvider()
    )
    do {
      guard let support = FileManager.default.urls(
        for: .applicationSupportDirectory,
        in: .userDomainMask
      ).first else {
        throw OutboxError.persistenceFailure("application support directory is unavailable")
      }
      let directory = support.appendingPathComponent("AnxietyWatch", isDirectory: true)
      outbox = try FileOutboxStore(directory: directory)
      journalMirror = try AppJournalMirrorStore(directory: directory)
      journalBackupIncluded = false
      journalBackupChoiceRequired = true
      initializationError = nil
    } catch {
      outbox = nil
      journalMirror = nil
      journalBackupIncluded = false
      journalBackupChoiceRequired = true
      initializationError = error.localizedDescription
    }
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
      syncStatus = "Sync unavailable: \(initializationError ?? "journal storage initialization failed")"
      return
    }
    do {
      guard let credential = try credentialStore.load() else {
        isEnrolled = false
        syncEngine = nil
        projectionService = nil
        syncStatus = "Sync unavailable: this iPhone is not enrolled."
        return
      }
      let authenticator = RequestAuthenticator(keyStore: keyStore, origin: credential.origin)
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
      let cache = try ProjectionCache()
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
      isEnrolled = true
      syncStatus = "Enrolled; waiting for a queued write or Sync Now."
    } catch {
      isEnrolled = false
      syncEngine = nil
      projectionService = nil
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
    guard let syncEngine else {
      syncStatus = isEnrolled
        ? "Sync blocked: the enrolled transport is not initialized."
        : "Sync unavailable: this iPhone is not enrolled."
      return
    }
    do {
      let report = try await syncEngine.drain()
      outboxItemCount = report.pendingAfter
      syncStatus = "Sync complete: \(report.acknowledged) acknowledged; \(report.pendingAfter) pending."
    } catch {
      await refreshOutboxState()
      syncStatus = "Sync blocked: \(error)"
    }
  }
}

#if canImport(WatchConnectivity)
@MainActor
private final class AppWatchRelayReceiver: NSObject, WCSessionDelegate {
  private weak var runtime: AppClientRuntime?

  func activate(runtime: AppClientRuntime) {
    self.runtime = runtime
    guard WCSession.isSupported() else { return }
    WCSession.default.delegate = self
    WCSession.default.activate()
  }

  nonisolated func session(
    _ session: WCSession,
    activationDidCompleteWith activationState: WCSessionActivationState,
    error: (any Error)?
  ) {}

  nonisolated func session(_ session: WCSession, didReceiveUserInfo userInfo: [String: Any] = [:]) {
    Task { @MainActor [weak self] in
      await self?.receive(userInfo, reply: nil)
    }
  }

  nonisolated func session(
    _ session: WCSession,
    didReceiveMessage message: [String: Any],
    replyHandler: @escaping ([String: Any]) -> Void
  ) {
    Task { @MainActor [weak self] in
      await self?.receive(message, reply: replyHandler)
    }
  }

  private func receive(
    _ message: [String: Any],
    reply: (([String: Any]) -> Void)?
  ) async {
    guard message["type"] as? String == "watch_quick_log",
      let rawID = message["id"] as? String,
      let id = UUID(uuidString: rawID),
      let capturedAt = message["captured_at"] as? String,
      let rawAnxiety = message["anxiety"] as? Int,
      let anxiety = UInt8(exactly: rawAnxiety),
      let runtime
    else {
      reply?(["accepted": false])
      return
    }
    do {
      let item = try WatchQuickLog(id: id, capturedAt: capturedAt, anxietyRawValue: anxiety)
      try await runtime.acceptWatchQuickLog(item)
      runtime.setWatchRelayStatus("Watch Quick Log queued on iPhone; waiting for Pi acknowledgement.")
      reply?(["accepted": true, "id": id.uuidString.lowercased()])
      if reply == nil {
        WCSession.default.transferUserInfo([
          "type": "watch_quick_log_ack",
          "id": id.uuidString.lowercased(),
          "accepted": true
        ])
      }
    } catch {
      runtime.setWatchRelayStatus("Watch Quick Log blocked: durable iPhone enqueue failed.")
      reply?(["accepted": false])
    }
  }
}
#endif

@main
struct AnxietyWatchApp: App {
  @StateObject private var runtime = AppClientRuntime()

  var body: some Scene {
    WindowGroup {
      ClientRootView()
        .environmentObject(runtime)
    }
  }
}
