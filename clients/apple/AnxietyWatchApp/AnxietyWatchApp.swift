import AWDomain
import AWOutbox
import AWTransport
import Combine
import Foundation
import SwiftUI

struct AppJournalMirrorEntry: Codable, Equatable, Identifiable, Sendable {
  let record: DomainRecordPayload
  let syncState: DomainSyncState

  var id: UUID { record.id }
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
    if let index = persisted.entries.firstIndex(where: { $0.record.id == record.id }) {
      let existing = persisted.entries[index]
      guard record.disposition(comparedWith: existing.record) != .conflict else {
        throw OutboxError.invalidItem("journal mirror conflict: record \(record.id) has different content")
      }
      persisted.entries[index] = AppJournalMirrorEntry(record: existing.record, syncState: syncState)
    } else {
      persisted.entries.append(AppJournalMirrorEntry(record: record, syncState: syncState))
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
      for item in try await outbox.allItems() where item.recordClass == "journal_entry" {
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
