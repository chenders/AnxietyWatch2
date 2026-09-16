import Foundation

public enum OutboxError: Error, Equatable, Sendable {
  case invalidItem(String)
  case persistenceFailure(String)
  case resetBlocked(String)
  case drainAlreadyOwned(String)
  case itemNotFound(String)
}

/// Credential-neutral content waiting for delivery. It never contains a signature, key, or receipt.
public struct OutboxItem: Codable, Equatable, Sendable {
  public let id: UUID
  public let recordClass: String
  public let schemaVersion: UInt
  public let payload: Data
  public let capturedAt: Date
  public private(set) var state: OutboxState
  public private(set) var attemptCount: Int
  public private(set) var nextEligibleAttemptAt: Date?
  public private(set) var lastFailure: OutboxFailure?

  public init(
    id: UUID,
    recordClass: String,
    schemaVersion: UInt,
    payload: Data,
    capturedAt: Date,
    state: OutboxState = .pending,
    attemptCount: Int = 0,
    nextEligibleAttemptAt: Date? = nil,
    lastFailure: OutboxFailure? = nil
  ) throws {
    guard !recordClass.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw OutboxError.invalidItem("record class is missing")
    }
    guard schemaVersion > 0 else { throw OutboxError.invalidItem("schema version must be positive") }
    guard !payload.isEmpty else { throw OutboxError.invalidItem("payload is empty") }
    guard attemptCount >= 0 else { throw OutboxError.invalidItem("attempt count is negative") }
    self.id = id
    self.recordClass = recordClass
    self.schemaVersion = schemaVersion
    self.payload = payload
    self.capturedAt = capturedAt
    self.state = state
    self.attemptCount = attemptCount
    self.nextEligibleAttemptAt = nextEligibleAttemptAt
    self.lastFailure = lastFailure
  }

  mutating func markSending() {
    state = .pending
  }

  mutating func recordRetry(failure: OutboxFailure, nextAttemptAt: Date) {
    attemptCount += 1
    state = .retryable
    lastFailure = failure
    nextEligibleAttemptAt = nextAttemptAt
  }

  mutating func recordRejected(failure: OutboxFailure) {
    attemptCount += 1
    state = .rejected
    lastFailure = failure
    nextEligibleAttemptAt = nil
  }

  mutating func recordConflict(failure: OutboxFailure) {
    attemptCount += 1
    state = .conflict
    lastFailure = failure
    nextEligibleAttemptAt = nil
  }
}

public struct OutboxFailure: Codable, Equatable, Sendable {
  public let code: String
  public let reason: String
  public let occurredAt: Date

  public init(code: String, reason: String, occurredAt: Date) throws {
    guard !code.isEmpty else { throw OutboxError.invalidItem("failure code is missing") }
    guard !reason.isEmpty else { throw OutboxError.invalidItem("failure reason is missing") }
    self.code = code
    self.reason = reason
    self.occurredAt = occurredAt
  }
}

public struct OutboxReceipt: Codable, Equatable, Sendable {
  public let itemID: UUID
  public let outcome: String
  public let acknowledgedAt: Date
  public let serverAttemptID: String?

  public init(itemID: UUID, outcome: String, acknowledgedAt: Date, serverAttemptID: String?) throws {
    guard !outcome.isEmpty else { throw OutboxError.invalidItem("receipt outcome is missing") }
    self.itemID = itemID
    self.outcome = outcome
    self.acknowledgedAt = acknowledgedAt
    self.serverAttemptID = serverAttemptID
  }
}

public struct OutboxSnapshot: Equatable, Sendable {
  public let pendingCount: Int
  public let oldestPendingAt: Date?
  public let receiptCount: Int

  public init(pendingCount: Int, oldestPendingAt: Date?, receiptCount: Int) {
    self.pendingCount = pendingCount
    self.oldestPendingAt = oldestPendingAt
    self.receiptCount = receiptCount
  }
}

public protocol DurableOutbox: Sendable {
  func enqueue(_ item: OutboxItem) async throws
  func pendingItems(limit: Int) async throws -> [OutboxItem]
  func allItems() async throws -> [OutboxItem]
  func markSending(itemID: UUID) async throws
  func recordRetry(itemID: UUID, failure: OutboxFailure, nextAttemptAt: Date) async throws
  func recordRejected(itemID: UUID, failure: OutboxFailure) async throws
  func recordConflict(itemID: UUID, failure: OutboxFailure) async throws
  func recordAcknowledgement(itemID: UUID, acknowledgedAt: Date, serverAttemptID: String?) async throws
  func discard(itemID: UUID, confirmed: Bool) async throws
  func snapshot() async throws -> OutboxSnapshot
  func acquireDrainOwnership() async throws
  func releaseDrainOwnership() async
  func reset(allowLoss: Bool) async throws
}

private struct PersistedOutbox: Codable {
  var items: [OutboxItem]
  var receipts: [OutboxReceipt]
}

/// A small, atomic JSON store for temporary queued writes. It is not a history database and stores
/// no server projection. A replacement write is committed through a temp file before the live file
/// is replaced; a terminal acknowledgement removes payload content in the same commit.
public actor FileOutboxStore: DurableOutbox {
  private let fileURL: URL
  private var persisted: PersistedOutbox
  private var drainOwned = false

  public init(directory: URL, fileName: String = "outbox.json") throws {
    self.fileURL = directory.appendingPathComponent(fileName, isDirectory: false)
    do {
      try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
      self.persisted = try Self.load(fileURL: self.fileURL)
      self.persisted.items = self.persisted.items.map { item in
        var recovered = item
        if recovered.state == .pending { return recovered }
        if recovered.state == .retryable || recovered.state == .rejected || recovered.state == .conflict {
          return recovered
        }
        recovered.markSending()
        return recovered
      }
      try Self.applyProtection(to: self.fileURL)
    } catch let error as OutboxError {
      throw error
    } catch {
      throw OutboxError.persistenceFailure("outbox load failed: \(error.localizedDescription)")
    }
  }

  public func enqueue(_ item: OutboxItem) throws {
    guard !persisted.items.contains(where: { $0.id == item.id }) else {
      throw OutboxError.invalidItem("item \(item.id) is already queued")
    }
    persisted.items.append(item)
    try persist()
  }

  public func pendingItems(limit: Int) throws -> [OutboxItem] {
    guard (1...500).contains(limit) else {
      throw OutboxError.invalidItem("pending item limit must be 1...500; received \(limit)")
    }
    return Array(
      persisted.items
        .filter { $0.state == .pending || $0.state == .retryable }
        .sorted { $0.capturedAt < $1.capturedAt }
        .prefix(limit)
    )
  }

  public func allItems() throws -> [OutboxItem] {
    persisted.items.sorted { $0.capturedAt < $1.capturedAt }
  }

  public func markSending(itemID: UUID) throws {
    guard let index = persisted.items.firstIndex(where: { $0.id == itemID }) else {
      throw OutboxError.itemNotFound("cannot mark missing item \(itemID) as sending")
    }
    persisted.items[index].markSending()
    try persist()
  }

  public func recordRetry(itemID: UUID, failure: OutboxFailure, nextAttemptAt: Date) throws {
    guard let index = persisted.items.firstIndex(where: { $0.id == itemID }) else {
      throw OutboxError.itemNotFound("cannot retry missing item \(itemID)")
    }
    persisted.items[index].recordRetry(failure: failure, nextAttemptAt: nextAttemptAt)
    try persist()
  }

  public func recordRejected(itemID: UUID, failure: OutboxFailure) throws {
    guard let index = persisted.items.firstIndex(where: { $0.id == itemID }) else {
      throw OutboxError.itemNotFound("cannot reject missing item \(itemID)")
    }
    persisted.items[index].recordRejected(failure: failure)
    try persist()
  }

  public func recordConflict(itemID: UUID, failure: OutboxFailure) throws {
    guard let index = persisted.items.firstIndex(where: { $0.id == itemID }) else {
      throw OutboxError.itemNotFound("cannot conflict missing item \(itemID)")
    }
    persisted.items[index].recordConflict(failure: failure)
    try persist()
  }

  public func recordAcknowledgement(
    itemID: UUID,
    acknowledgedAt: Date,
    serverAttemptID: String?
  ) throws {
    guard let index = persisted.items.firstIndex(where: { $0.id == itemID }) else {
      throw OutboxError.itemNotFound("cannot acknowledge missing item \(itemID)")
    }
    persisted.items.remove(at: index)
    let receipt = try OutboxReceipt(
      itemID: itemID,
      outcome: "acknowledged",
      acknowledgedAt: acknowledgedAt,
      serverAttemptID: serverAttemptID
    )
    persisted.receipts.append(receipt)
    persisted.receipts = Array(persisted.receipts.suffix(100))
    try persist()
  }

  public func discard(itemID: UUID, confirmed: Bool) throws {
    guard confirmed else {
      throw OutboxError.resetBlocked("discard blocked: explicit confirmation is false for item \(itemID)")
    }
    guard let index = persisted.items.firstIndex(where: { $0.id == itemID }) else {
      throw OutboxError.itemNotFound("cannot discard missing item \(itemID)")
    }
    persisted.items.remove(at: index)
    try persist()
  }

  public func snapshot() throws -> OutboxSnapshot {
    return OutboxSnapshot(
      pendingCount: persisted.items.count,
      oldestPendingAt: persisted.items.map(\.capturedAt).min(),
      receiptCount: persisted.receipts.count
    )
  }

  public func acquireDrainOwnership() throws {
    guard !drainOwned else {
      throw OutboxError.drainAlreadyOwned("drain blocked: this outbox already has an active owner")
    }
    drainOwned = true
  }

  public func releaseDrainOwnership() { drainOwned = false }

  public func reset(allowLoss: Bool) throws {
    guard allowLoss else {
      let oldest = persisted.items.map(\.capturedAt).min().map(String.init(describing:)) ?? "none"
      throw OutboxError.resetBlocked(
        "reset blocked: \(persisted.items.count) unacknowledged items, oldest captured at \(oldest)"
      )
    }
    persisted.items.removeAll()
    try persist()
  }

  private func persist() throws {
    do {
      let encoder = JSONEncoder()
      encoder.dateEncodingStrategy = .iso8601
      let data = try encoder.encode(persisted)
      try data.write(to: fileURL, options: .atomic)
      try Self.applyProtection(to: fileURL)
    } catch let error as OutboxError {
      throw error
    } catch {
      throw OutboxError.persistenceFailure("outbox commit failed: \(error.localizedDescription)")
    }
  }

  private static func load(fileURL: URL) throws -> PersistedOutbox {
    guard FileManager.default.fileExists(atPath: fileURL.path) else {
      return PersistedOutbox(items: [], receipts: [])
    }
    do {
      let decoder = JSONDecoder()
      decoder.dateDecodingStrategy = .iso8601
      return try decoder.decode(PersistedOutbox.self, from: Data(contentsOf: fileURL))
    } catch {
      throw OutboxError.persistenceFailure("outbox file is unreadable: \(error.localizedDescription)")
    }
  }

  private static func applyProtection(to url: URL) throws {
    guard FileManager.default.fileExists(atPath: url.path) else { return }
    #if os(iOS)
    do {
      try FileManager.default.setAttributes(
        [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
        ofItemAtPath: url.path
      )
    } catch {
      throw OutboxError.persistenceFailure("Data Protection could not be applied to \(url.lastPathComponent)")
    }
    #endif
    do {
      var values = URLResourceValues()
      values.isExcludedFromBackup = true
      var protectedURL = url
      try protectedURL.setResourceValues(values)
    } catch {
      throw OutboxError.persistenceFailure("backup exclusion could not be applied to \(url.lastPathComponent)")
    }
  }
}
