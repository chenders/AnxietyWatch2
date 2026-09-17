import AWDomain
import AWOutbox
import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public enum TransportFailure: Error, Equatable, Sendable {
  case network(String)
  case http(status: Int, retryAfter: TimeInterval?)
  case server(code: String, retryable: Bool, retryAfter: TimeInterval? = nil)
  case credentialExpired(String)
  case credentialRevoked(String)
  case incompatibleVersion(String)
  case responseTooLarge(String)

  public var isRetryable: Bool {
    switch self {
    case .network, .http, .server(_, true, _): true
    case .server(_, false, _), .credentialExpired, .credentialRevoked, .incompatibleVersion, .responseTooLarge: false
    }
  }

  public var code: String {
    switch self {
    case .network: "network_failure"
    case .http(let status, _): "http_\(status)"
    case .server(let code, _, _): code
    case .credentialExpired: "credential_expired"
    case .credentialRevoked: "credential_revoked"
    case .incompatibleVersion: "api_version_incompatible"
    case .responseTooLarge: "response_too_large"
    }
  }

  public var reason: String {
    switch self {
    case .network(let reason): reason
    case .http(let status, _): "HTTP status \(status)"
    case .server(let code, _, _): "server rejected request with \(code)"
    case .credentialExpired(let value): "credential expired: \(value)"
    case .credentialRevoked(let value): "credential revoked: \(value)"
    case .incompatibleVersion(let value): "API version blocked: \(value)"
    case .responseTooLarge(let value): value
    }
  }

  public var retryAfter: TimeInterval? {
    switch self {
    case .http(_, let retryAfter), .server(_, _, let retryAfter): return retryAfter
    default: return nil
    }
  }
}

public protocol ClientItemTransport: Sendable {
  func send(
    batch: ClientWriteBatch,
    credential: StoredDeviceCredential
  ) async throws -> ClientWriteResponse
}

/// Receives terminal and retry evidence for records retained in the protected local mirror.
public protocol DomainSyncStateSink: Sendable {
  func update(recordID: UUID, state: DomainSyncState) async throws
}

public struct SyncConfiguration: Sendable {
  public let batchSize: Int
  public let maximumBackoff: TimeInterval
  public let maximumRetryAfter: TimeInterval
  public let jitter: @Sendable (TimeInterval) -> TimeInterval

  public static let standard = SyncConfiguration(
    uncheckedBatchSize: 25,
    uncheckedMaximumBackoff: 3600,
    uncheckedMaximumRetryAfter: 3600,
    jitter: { base in Double.random(in: 0...(base / 4)) }
  )

  public init(
    batchSize: Int = 25,
    maximumBackoff: TimeInterval = 3600,
    maximumRetryAfter: TimeInterval = 3600,
    jitter: @escaping @Sendable (TimeInterval) -> TimeInterval = { base in
      Double.random(in: 0...(base / 4))
    }
  ) throws {
    guard (1...100).contains(batchSize) else {
      throw EnrollmentError.invalidArtifact("sync batch size must be 1...100; received \(batchSize)")
    }
    guard maximumBackoff > 0 else {
      throw EnrollmentError.invalidArtifact("maximum backoff must be positive; received \(maximumBackoff)")
    }
    guard maximumRetryAfter > 0 else {
      throw EnrollmentError.invalidArtifact("maximum Retry-After must be positive; received \(maximumRetryAfter)")
    }
    self.init(
      uncheckedBatchSize: batchSize,
      uncheckedMaximumBackoff: maximumBackoff,
      uncheckedMaximumRetryAfter: maximumRetryAfter,
      jitter: jitter
    )
  }

  private init(
    uncheckedBatchSize: Int,
    uncheckedMaximumBackoff: TimeInterval,
    uncheckedMaximumRetryAfter: TimeInterval,
    jitter: @escaping @Sendable (TimeInterval) -> TimeInterval
  ) {
    self.batchSize = uncheckedBatchSize
    self.maximumBackoff = uncheckedMaximumBackoff
    self.maximumRetryAfter = uncheckedMaximumRetryAfter
    self.jitter = jitter
  }
}

public struct SyncReport: Equatable, Sendable {
  public let attempted: Int
  public let acknowledged: Int
  public let replayed: Int
  public let rejected: Int
  public let conflicts: Int
  public let retryable: Int
  public let pendingAfter: Int

  public init(
    attempted: Int,
    acknowledged: Int,
    replayed: Int,
    rejected: Int,
    conflicts: Int,
    retryable: Int,
    pendingAfter: Int
  ) {
    self.attempted = attempted
    self.acknowledged = acknowledged
    self.replayed = replayed
    self.rejected = rejected
    self.conflicts = conflicts
    self.retryable = retryable
    self.pendingAfter = pendingAfter
  }
}

public enum SyncError: Error, Equatable, Sendable {
  case credentialUnavailable(String)
  case blocked(TransportFailure)
  case invalidPayload(String)
}

/// A single-owner, trigger-driven drain. It never starts a timer or keeps a socket alive.
public actor SyncEngine {
  private let outbox: any DurableOutbox
  private let credentialStore: any DeviceCredentialStore
  private let transport: any ClientItemTransport
  private let stateSink: (any DomainSyncStateSink)?
  private let configuration: SyncConfiguration
  private let now: @Sendable () -> Date

  public init(
    outbox: any DurableOutbox,
    credentialStore: any DeviceCredentialStore,
    transport: any ClientItemTransport,
    stateSink: (any DomainSyncStateSink)? = nil,
    configuration: SyncConfiguration = .standard,
    now: @escaping @Sendable () -> Date = { Date() }
  ) {
    self.outbox = outbox
    self.credentialStore = credentialStore
    self.transport = transport
    self.stateSink = stateSink
    self.configuration = configuration
    self.now = now
  }

  public func drain() async throws -> SyncReport {
    do {
      try await outbox.acquireDrainOwnership()
    } catch let error as OutboxError {
      throw SyncError.invalidPayload(error.localizedDescription)
    } catch {
      throw SyncError.invalidPayload("drain ownership failed: \(error.localizedDescription)")
    }

    do {
      let report = try await performDrain()
      await outbox.releaseDrainOwnership()
      return report
    } catch {
      await outbox.releaseDrainOwnership()
      throw error
    }
  }

  private func performDrain() async throws -> SyncReport {
    let credential: StoredDeviceCredential
    do {
      guard let loaded = try credentialStore.load() else {
        throw SyncError.credentialUnavailable("no enrolled device credential is stored")
      }
      credential = loaded
    } catch let error as SyncError {
      throw error
    } catch {
      throw SyncError.credentialUnavailable("credential read failed: \(error.localizedDescription)")
    }

    let eligible = try await outbox.pendingItems(limit: configuration.batchSize).filter { item in
      guard let next = item.nextEligibleAttemptAt else { return true }
      return next <= now()
    }
    if eligible.isEmpty {
      let snapshot = try await outbox.snapshot()
      return SyncReport(
        attempted: 0, acknowledged: 0, replayed: 0, rejected: 0, conflicts: 0, retryable: 0,
        pendingAfter: snapshot.pendingCount
      )
    }

    var writableItems: [OutboxItem] = []
    var rejected = 0
    for item in eligible {
      do {
        _ = try JSONDecoder().decode(JSONValue.self, from: item.payload)
        try await outbox.markSending(itemID: item.id)
        writableItems.append(item)
      } catch {
        let failure = try OutboxFailure(
          code: "invalid_payload",
          reason: "item \(item.id) payload is not valid JSON",
          occurredAt: now()
        )
        try await outbox.recordRejected(itemID: item.id, failure: failure)
        try await updateState(for: item.id, state: .unavailable)
        rejected += 1
      }
    }

    guard !writableItems.isEmpty else {
      let snapshot = try await outbox.snapshot()
      return SyncReport(
        attempted: eligible.count, acknowledged: 0, replayed: 0, rejected: rejected,
        conflicts: 0, retryable: 0, pendingAfter: snapshot.pendingCount
      )
    }

    let items = try writableItems.map { item in
      let content = try JSONDecoder().decode(JSONValue.self, from: item.payload)
      return ClientWriteItem(
        id: item.id,
        recordClass: item.recordClass,
        schemaVersion: item.schemaVersion,
        content: content
      )
    }
    let batch = ClientWriteBatch(
      batchID: UUID(),
      recordedAt: iso8601(now()),
      items: items
    )

    let response: ClientWriteResponse
    do {
      response = try await transport.send(batch: batch, credential: credential)
    } catch let failure as TransportFailure {
      try await handleTransportFailure(failure, items: writableItems)
      if !failure.isRetryable { throw SyncError.blocked(failure) }
      let snapshot = try await outbox.snapshot()
      return SyncReport(
        attempted: writableItems.count + rejected, acknowledged: 0, replayed: 0, rejected: rejected,
        conflicts: 0, retryable: writableItems.count, pendingAfter: snapshot.pendingCount
      )
    } catch {
      let failure = TransportFailure.network(error.localizedDescription)
      try await handleTransportFailure(failure, items: writableItems)
      let snapshot = try await outbox.snapshot()
      return SyncReport(
        attempted: writableItems.count + rejected, acknowledged: 0, replayed: 0, rejected: rejected,
        conflicts: 0, retryable: writableItems.count, pendingAfter: snapshot.pendingCount
      )
    }

    var acknowledged = 0
    var replayed = 0
    var conflicts = 0
    var retryable = 0
    var seen = Set<UUID>()
    for acknowledgement in response.acknowledgements {
      guard let item = writableItems.first(where: { $0.id == acknowledgement.itemID }) else { continue }
      seen.insert(item.id)
      switch acknowledgement.status {
      case "created":
        try await updateState(for: item.id, state: .acknowledged)
        try await outbox.recordAcknowledgement(
          itemID: item.id, acknowledgedAt: now(), serverAttemptID: acknowledgement.code
        )
        acknowledged += 1
      case "replayed":
        try await updateState(for: item.id, state: .acknowledged)
        try await outbox.recordAcknowledgement(
          itemID: item.id, acknowledgedAt: now(), serverAttemptID: acknowledgement.code
        )
        acknowledged += 1
        replayed += 1
      case "rejected":
        let failure = try OutboxFailure(
          code: acknowledgement.code ?? "rejected",
          reason: acknowledgement.message,
          occurredAt: now()
        )
        try await outbox.recordRejected(itemID: item.id, failure: failure)
        try await updateState(for: item.id, state: .unavailable)
        rejected += 1
      case "conflict":
        let failure = try OutboxFailure(
          code: acknowledgement.code ?? "conflict",
          reason: acknowledgement.message,
          occurredAt: now()
        )
        try await outbox.recordConflict(itemID: item.id, failure: failure)
        try await updateState(for: item.id, state: .conflict)
        conflicts += 1
      case "retryable":
        let failure = try OutboxFailure(
          code: acknowledgement.code ?? "retryable",
          reason: acknowledgement.message,
          occurredAt: now()
        )
        try await outbox.recordRetry(
          itemID: item.id,
          failure: failure,
          nextAttemptAt: nextAttempt(for: item, retryAfter: nil)
        )
        try await updateState(for: item.id, state: .queued)
        retryable += 1
      default:
        let failure = try OutboxFailure(
          code: "unknown_ack_status",
          reason: "item \(item.id) returned unknown status \(acknowledgement.status)",
          occurredAt: now()
        )
        try await outbox.recordRetry(
          itemID: item.id,
          failure: failure,
          nextAttemptAt: nextAttempt(for: item, retryAfter: nil)
        )
        try await updateState(for: item.id, state: .queued)
        retryable += 1
      }
    }

    for item in writableItems where !seen.contains(item.id) {
      let failure = try OutboxFailure(
        code: "ack_missing",
        reason: "server response omitted item \(item.id); payload preserved",
        occurredAt: now()
      )
      try await outbox.recordRetry(
        itemID: item.id,
        failure: failure,
        nextAttemptAt: nextAttempt(for: item, retryAfter: nil)
      )
      try await updateState(for: item.id, state: .queued)
      retryable += 1
    }

    let snapshot = try await outbox.snapshot()
    return SyncReport(
      attempted: writableItems.count + rejected,
      acknowledged: acknowledged,
      replayed: replayed,
      rejected: rejected,
      conflicts: conflicts,
      retryable: retryable,
      pendingAfter: snapshot.pendingCount
    )
  }

  private func handleTransportFailure(_ failure: TransportFailure, items: [OutboxItem]) async throws {
    if failure.isRetryable {
      for item in items {
        let classified = try OutboxFailure(code: failure.code, reason: failure.reason, occurredAt: now())
        try await outbox.recordRetry(
          itemID: item.id,
          failure: classified,
          nextAttemptAt: nextAttempt(for: item, retryAfter: failure.retryAfter)
        )
        try await updateState(for: item.id, state: .queued)
      }
    } else {
      for item in items {
        let classified = try OutboxFailure(code: failure.code, reason: failure.reason, occurredAt: now())
        try await outbox.recordRejected(itemID: item.id, failure: classified)
        try await updateState(for: item.id, state: .unavailable)
      }
    }
  }

  private func updateState(for recordID: UUID, state: DomainSyncState) async throws {
    try await stateSink?.update(recordID: recordID, state: state)
  }

  private func nextAttempt(for item: OutboxItem, retryAfter: TimeInterval?) -> Date {
    let delay: TimeInterval
    if let retryAfter {
      delay = min(max(0, retryAfter), configuration.maximumRetryAfter)
    } else {
      let exponent = min(item.attemptCount, 10)
      let base = min(pow(2, Double(exponent)), configuration.maximumBackoff)
      let requestedJitter = max(0, configuration.jitter(base))
      delay = min(base + requestedJitter, configuration.maximumBackoff)
    }
    return now().addingTimeInterval(delay)
  }
}

#if canImport(CryptoKit)
import CryptoKit

/// Signs the fixed request profile used by the client API. The Secure Enclave key store supplies
/// the signing operation; the private key and signature never enter the outbox.
public struct RequestAuthenticator: Sendable {
  private let keyStore: any DeviceKeyStore
  private let origin: ClientOrigin

  public init(keyStore: any DeviceKeyStore, origin: ClientOrigin) {
    self.keyStore = keyStore
    self.origin = origin
  }

  public func headers(
    body: Data,
    credential: StoredDeviceCredential,
    now: Date,
    method: String = "POST",
    path: String = "/api/v1/client/items"
  ) throws -> [String: String] {
    guard let host = origin.url.host else {
      throw EnrollmentError.invalidOrigin("authenticated request origin has no hostname")
    }
    let authorityHost = host.contains(":") ? "[\(host)]" : host
    let authority = authorityHost + (origin.url.port.map { ":\($0)" } ?? "")
    let requestID = UUID()
    let created = Int(now.timeIntervalSince1970)
    let expires = created + 120
    let digest = Data(SHA256.hash(data: body)).base64EncodedString()
    let contentDigest = "sha-256=:\(digest):"
    let contentType = "application/json"
    let signatureInput =
      "sig1=(\"@method\" \"@authority\" \"@path\" \"content-digest\" \"content-type\" " +
      "\"x-anxietywatch-api-version\" \"x-anxietywatch-client-id\" \"x-anxietywatch-key-id\" " +
      "\"x-anxietywatch-request-id\");created=\(created);expires=\(expires)"
    let signatureParams = signatureInput.dropFirst(5)
    let signatureBase = "\"@method\": \(method)\n\"@authority\": \(authority)\n\"@path\": \(path)\ncontent-digest: \(contentDigest)\ncontent-type: \(contentType)\nx-anxietywatch-api-version: 1\nx-anxietywatch-client-id: \(credential.clientID.uuidString.lowercased())\nx-anxietywatch-key-id: \(credential.keyID.uuidString.lowercased())\nx-anxietywatch-request-id: \(requestID.uuidString.lowercased())\n\"@signature-params\": \(signatureParams)"
    let signature = try keyStore.sign(
      Data(signatureBase.utf8),
      with: credential.keyReference
    ).base64EncodedString()
    return [
      "X-AnxietyWatch-API-Version": "1",
      "X-AnxietyWatch-Client-ID": credential.clientID.uuidString.lowercased(),
      "X-AnxietyWatch-Key-ID": credential.keyID.uuidString.lowercased(),
      "X-AnxietyWatch-Request-ID": requestID.uuidString.lowercased(),
      "Content-Digest": contentDigest,
      "Content-Type": contentType,
      "Signature-Input": signatureInput,
      "Signature": "sig1=:\(signature):",
      "Host": authority,
    ]
  }
}

public final class URLSessionClientItemTransport: ClientItemTransport, @unchecked Sendable {
  private let origin: ClientOrigin
  private let authenticator: RequestAuthenticator
  private let session: URLSession

  public init(origin: ClientOrigin, authenticator: RequestAuthenticator) {
    self.origin = origin
    self.authenticator = authenticator
    self.session = URLSession(configuration: .ephemeral)
  }

  public func send(
    batch: ClientWriteBatch,
    credential: StoredDeviceCredential
  ) async throws -> ClientWriteResponse {
    var url = origin.url
    url.append(path: "api/v1/client/items")
    var request = URLRequest(url: url)
    request.httpMethod = "POST"
    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
    let body = try JSONEncoder().encode(batch)
    request.httpBody = body
    for (field, value) in try authenticator.headers(
      body: body,
      credential: credential,
      now: Date(),
      method: "POST",
      path: "/api/v1/client/items"
    ) {
      request.setValue(value, forHTTPHeaderField: field)
    }
    let (data, response) = try await session.data(for: request)
    guard data.count <= 2 * 1024 * 1024 else {
      throw TransportFailure.responseTooLarge("write response was \(data.count) bytes")
    }
    guard let http = response as? HTTPURLResponse else {
      throw TransportFailure.network("write response was not HTTP")
    }
    guard (200..<300).contains(http.statusCode) else {
      let error = try? JSONDecoder().decode(ClientErrorResponse.self, from: data)
      let retryAfter = http.value(forHTTPHeaderField: "Retry-After").flatMap(TimeInterval.init)
      if let error {
        throw TransportFailure.server(
          code: error.code,
          retryable: error.retryable,
          retryAfter: retryAfter
        )
      }
      throw TransportFailure.http(status: http.statusCode, retryAfter: retryAfter)
    }
    do {
      return try JSONDecoder().decode(ClientWriteResponse.self, from: data)
    } catch {
      throw TransportFailure.network("write response did not match the client contract")
    }
  }
}
#endif

private func iso8601(_ date: Date) -> String {
  let formatter = ISO8601DateFormatter()
  formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
  return formatter.string(from: date)
}
