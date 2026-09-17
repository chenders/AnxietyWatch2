import AWOutbox
import AWTransport
import Combine
import Foundation
import SwiftUI

@MainActor
final class AppClientRuntime: ObservableObject {
  @Published private(set) var isEnrolled = false
  @Published private(set) var syncStatus = "Sync is not initialized."
  @Published private(set) var outboxItemCount = 0
  let songSearchCoordinator: SongSearchCoordinator

  private let credentialStore: KeychainCredentialStore
  private let keyStore: SecureEnclaveKeyStore
  private let initializationError: String?
  private let outbox: FileOutboxStore?
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
      initializationError = nil
    } catch {
      outbox = nil
      initializationError = error.localizedDescription
    }
  }

  func makeJournalCaptureCoordinator() throws -> JournalCaptureCoordinator {
    guard let outbox else {
      throw OutboxError.persistenceFailure(
        "journal storage is unavailable: \(initializationError ?? "unknown initialization failure")"
      )
    }
    return JournalCaptureCoordinator(
      sink: AppDomainCommandSink(outbox: outbox),
      drainTrigger: AppImmediateDrainTrigger()
    )
  }

  func reload() async {
    guard let outbox else {
      isEnrolled = false
      syncEngine = nil
      projectionService = nil
      syncStatus = "Sync unavailable: \(initializationError ?? "outbox initialization failed")"
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
        transport: writeTransport
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
    await refreshOutboxState()
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
