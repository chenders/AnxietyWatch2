import AWDomain
import SwiftUI
import WatchConnectivity
import WatchKit

actor WatchRelayStore {
  private let fileURL: URL
  private var queue: WatchRelayQueue

  init(fileManager: FileManager = .default) throws {
    guard let support = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
      throw WatchRelayError.storageUnavailable
    }
    let directory = support.appendingPathComponent("AnxietyWatch", isDirectory: true)
    try fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
    fileURL = directory.appendingPathComponent("watch-relay.json", isDirectory: false)
    if fileManager.fileExists(atPath: fileURL.path) {
      do {
        queue = try JSONDecoder().decode(WatchRelayQueue.self, from: Data(contentsOf: fileURL))
      } catch {
        throw WatchRelayError.storageUnreadable
      }
    } else {
      queue = WatchRelayQueue()
    }
    try Self.protect(fileURL)
  }

  func enqueue(_ item: WatchQuickLog) throws {
    try queue.enqueue(item)
    try persist()
  }

  func pending() -> [WatchQuickLog] { queue.items }

  func acknowledge(id: UUID) throws {
    queue.acknowledge(id: id)
    try persist()
  }

  private func persist() throws {
    do {
      try JSONEncoder().encode(queue).write(to: fileURL, options: .atomic)
      try Self.protect(fileURL)
    } catch let error as WatchRelayError {
      throw error
    } catch {
      throw WatchRelayError.storageCommitFailed
    }
  }

  private static func protect(_ url: URL) throws {
    guard FileManager.default.fileExists(atPath: url.path) else { return }
#if os(watchOS)
    do {
      try FileManager.default.setAttributes(
        [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
        ofItemAtPath: url.path
      )
    } catch {
      throw WatchRelayError.storageProtectionFailed
    }
#endif
    do {
      var values = URLResourceValues()
      values.isExcludedFromBackup = true
      var protectedURL = url
      try protectedURL.setResourceValues(values)
    } catch {
      throw WatchRelayError.storageProtectionFailed
    }
  }
}

enum WatchRelayError: Error, Equatable {
  case storageUnavailable
  case storageUnreadable
  case storageCommitFailed
  case storageProtectionFailed
}

@MainActor
final class WatchRelayModel: NSObject, ObservableObject, WCSessionDelegate {
  @Published private(set) var pendingCount = 0
  @Published private(set) var status = "Preparing durable iPhone relay…"

  private let store: WatchRelayStore?
  private let session: WCSession?

  override init() {
    do {
      store = try WatchRelayStore()
    } catch {
      store = nil
    }
    session = WCSession.isSupported() ? WCSession.default : nil
    super.init()
    session?.delegate = self
    session?.activate()
    Task { await refresh() }
  }

  func capture(level: UInt8) {
    guard let store else {
      status = "Quick Log blocked: durable watch storage is unavailable."
      return
    }
    Task { @MainActor in
      do {
        let item = try WatchQuickLog.capture(capturedAt: Date(), anxietyRawValue: level)
        try await store.enqueue(item)
        await refresh()
        send(item)
        WKInterfaceDevice.current().play(.success)
      } catch DomainCommandError.watchRelayQueueFull(let count) {
        status = "Quick Log blocked: Watch queue is full at \(count) items; connect the iPhone to relay entries."
      } catch {
        status = "Quick Log blocked: capture could not be durably queued."
      }
    }
  }

  private func refresh() async {
    guard let store else {
      pendingCount = 0
      return
    }
    pendingCount = await store.pending().count
  }

  private func sendPending() async {
    guard let store else { return }
    for item in await store.pending() {
      send(item)
    }
  }

  private func send(_ item: WatchQuickLog) {
    guard let session, session.activationState == .activated else {
      status = "Queued on Watch; iPhone relay is not activated."
      return
    }
    let payload: [String: Any] = [
      "type": "watch_quick_log",
      "id": item.id.uuidString.lowercased(),
      "captured_at": item.capturedAt,
      "anxiety": Int(item.anxiety.rawValue)
    ]
    if session.isReachable {
      session.sendMessage(
        payload,
        replyHandler: { [weak self] reply in
          guard reply["accepted"] as? Bool == true,
            let rawID = reply["id"] as? String
          else { return }
          Task { @MainActor in
            self?.acknowledgeFromPhone(rawID)
          }
        },
        errorHandler: { [weak self] _ in
          session.transferUserInfo(payload)
          Task { @MainActor in
            self?.status = "Queued on Watch; waiting for iPhone relay."
          }
        }
      )
    } else {
      session.transferUserInfo(payload)
      status = "Queued on Watch; waiting for iPhone relay."
    }
  }

  private func acknowledgeFromPhone(_ rawID: String) {
    guard let id = UUID(uuidString: rawID) else { return }
    Task { @MainActor in
      guard let store else { return }
      do {
        try await store.acknowledge(id: id)
        await refresh()
        status = "Queued on iPhone; server acknowledgement is pending."
      } catch {
        status = "Relay acknowledgement blocked: watch queue was preserved."
      }
    }
  }

  nonisolated func session(
    _ session: WCSession,
    activationDidCompleteWith activationState: WCSessionActivationState,
    error: (any Error)?
  ) {
    Task { @MainActor in
      if error != nil || activationState != .activated {
        self.status = "Queued on Watch; iPhone relay is unavailable."
      } else {
        await self.sendPending()
      }
    }
  }

  nonisolated func session(_ session: WCSession, didReceiveUserInfo userInfo: [String: Any] = [:]) {
    guard let rawID = userInfo["id"] as? String else { return }
    Task { @MainActor in
      self.acknowledgeFromPhone(rawID)
    }
  }
}

/// The watch keeps capture to one tap, then gives the durable relay ownership of delivery.
struct QuickLogSkeletonView: View {
  @StateObject private var relay = WatchRelayModel()
  private let ratings = (1...10).compactMap { AnxietyRating(rawValue: UInt8($0)) }

  var body: some View {
    ScrollView {
      VStack(spacing: 8) {
        Text("Quick Log")
          .font(.headline)
        LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())]) {
          ForEach(ratings, id: \.rawValue) { rating in
            Button {
              relay.capture(level: rating.rawValue)
            } label: {
              Text("\(rating.rawValue)")
                .font(.title3.bold())
                .frame(minWidth: 44, minHeight: 44)
                .frame(maxWidth: .infinity)
                .background(Circle().fill(severityColor(rating.rawValue)))
                .foregroundStyle(rating.rawValue >= 7 ? .white : .black)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Anxiety \(rating.rawValue) of 10, \(rating.band.rawValue)")
          }
        }
        Text(relay.status)
          .font(.footnote)
          .foregroundStyle(.secondary)
        if relay.pendingCount > 0 {
          Text("\(relay.pendingCount) queued on Watch")
            .font(.caption)
            .accessibilityLabel("\(relay.pendingCount) quick log entries queued on Watch")
        }
      }
    }
  }

  private func severityColor(_ level: UInt8) -> Color {
    switch level {
    case 1...2: return .green
    case 3...4: return .yellow
    case 5...6: return .orange
    case 7...8: return .red
    default: return Color(red: 0.6, green: 0.0, blue: 0.0)
    }
  }
}
