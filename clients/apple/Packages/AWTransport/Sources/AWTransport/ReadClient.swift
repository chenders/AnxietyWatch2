import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public struct ProjectionCacheKey: Hashable, Sendable {
  public let endpoint: String
  public let clientID: UUID
  public let apiMajor: UInt
  public let query: ClientProjectionQuery

  public init(endpoint: String, clientID: UUID, apiMajor: UInt, query: ClientProjectionQuery) {
    self.endpoint = endpoint
    self.clientID = clientID
    self.apiMajor = apiMajor
    self.query = query
  }
}

public struct CachedProjection: Codable, Equatable, Sendable {
  public let page: ClientProjectionPage
  public let fetchedAt: Date
  public let expiresAt: Date
  public let serverGeneratedAt: String
  public let serverObservedAt: String?

  public init(
    page: ClientProjectionPage,
    fetchedAt: Date,
    expiresAt: Date,
    serverGeneratedAt: String,
    serverObservedAt: String?
  ) {
    self.page = page
    self.fetchedAt = fetchedAt
    self.expiresAt = expiresAt
    self.serverGeneratedAt = serverGeneratedAt
    self.serverObservedAt = serverObservedAt
  }
}

public enum ProjectionReadState: Equatable, Sendable {
  case live(page: ClientProjectionPage, age: TimeInterval)
  case stale(page: ClientProjectionPage, age: TimeInterval, reason: String)
  case unknown(reason: String)
}

/// Bounded local cache for server projections. Eviction loses no record because the Pi is the
/// only long-term owner; cache identity includes endpoint, device, API major, and query. When a
/// persistence URL is supplied, entries survive relaunch and receive iOS file protection.
private struct PersistedProjectionCacheEntry: Codable {
  let key: ProjectionCacheKeyCodable
  let value: CachedProjection
}

private struct ProjectionCacheKeyCodable: Codable {
  let endpoint: String
  let clientID: UUID
  let apiMajor: UInt
  let query: ClientProjectionQuery

  init(_ key: ProjectionCacheKey) {
    endpoint = key.endpoint
    clientID = key.clientID
    apiMajor = key.apiMajor
    query = key.query
  }

  var key: ProjectionCacheKey {
    ProjectionCacheKey(endpoint: endpoint, clientID: clientID, apiMajor: apiMajor, query: query)
  }
}

public actor ProjectionCache {
  private let maximumEntries: Int
  private let persistenceURL: URL?
  private var entries: [ProjectionCacheKey: CachedProjection] = [:]

  public init(maximumEntries: Int = 50, persistenceURL: URL? = nil) throws {
    guard (1...500).contains(maximumEntries) else {
      throw EnrollmentError.invalidArtifact(
        "projection cache size must be 1...500; received \(maximumEntries)"
      )
    }
    self.maximumEntries = maximumEntries
    self.persistenceURL = persistenceURL
    if let persistenceURL,
      FileManager.default.fileExists(atPath: persistenceURL.path)
    {
      let data = try Data(contentsOf: persistenceURL)
      let persisted = try JSONDecoder().decode([PersistedProjectionCacheEntry].self, from: data)
      for item in persisted.sorted(by: { $0.value.fetchedAt < $1.value.fetchedAt }).suffix(maximumEntries) {
        entries[item.key.key] = item.value
      }
    }
  }

  public func put(
    _ page: ClientProjectionPage,
    for key: ProjectionCacheKey,
    fetchedAt: Date,
    expiresAt: Date
  ) throws {
    guard expiresAt > fetchedAt else {
      throw EnrollmentError.invalidArtifact("projection cache expiry must be after fetch time")
    }
    entries[key] = CachedProjection(
      page: page,
      fetchedAt: fetchedAt,
      expiresAt: expiresAt,
      serverGeneratedAt: page.generatedAt,
      serverObservedAt: page.observedAt
    )
    if entries.count > maximumEntries {
      let oldestKey = entries.min { lhs, rhs in lhs.value.fetchedAt < rhs.value.fetchedAt }?.key
      if let oldestKey { entries.removeValue(forKey: oldestKey) }
    }
    try persist()
  }

  private func persist() throws {
    guard let persistenceURL else { return }
    let data = try JSONEncoder().encode(
      entries.map { PersistedProjectionCacheEntry(key: ProjectionCacheKeyCodable($0.key), value: $0.value) }
    )
    try FileManager.default.createDirectory(
      at: persistenceURL.deletingLastPathComponent(),
      withIntermediateDirectories: true
    )
    try data.write(to: persistenceURL, options: .atomic)
#if os(iOS)
    try FileManager.default.setAttributes(
      [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
      ofItemAtPath: persistenceURL.path
    )
#endif
  }

  public func get(_ key: ProjectionCacheKey) -> CachedProjection? { entries[key] }
  public func remove(_ key: ProjectionCacheKey) { entries.removeValue(forKey: key) }
  public func removeAll() { entries.removeAll() }
}

public protocol ProjectionReading: Sendable {
  func read(
    query: ClientProjectionQuery,
    credential: StoredDeviceCredential
  ) async throws -> ClientProjectionPage
}

public actor ProjectionReadService {
  private let endpoint: String
  private let cache: ProjectionCache
  private let credentialStore: any DeviceCredentialStore
  private let transport: any ProjectionReading
  private let now: @Sendable () -> Date

  public init(
    endpoint: String,
    cache: ProjectionCache,
    credentialStore: any DeviceCredentialStore,
    transport: any ProjectionReading,
    now: @escaping @Sendable () -> Date = { Date() }
  ) {
    self.endpoint = endpoint
    self.cache = cache
    self.credentialStore = credentialStore
    self.transport = transport
    self.now = now
  }

  public func read(
    query: ClientProjectionQuery,
    cacheLifetime: TimeInterval = 300
  ) async -> ProjectionReadState {
    guard cacheLifetime > 0 else {
      return .unknown(reason: "read blocked: cache lifetime must be positive; received \(cacheLifetime)")
    }
    let credential: StoredDeviceCredential
    do {
      guard let loaded = try credentialStore.load() else {
        return .unknown(reason: "read unavailable: no enrolled device credential is stored")
      }
      credential = loaded
    } catch {
      return .unknown(reason: "read unavailable: credential read failed")
    }

    let key = ProjectionCacheKey(endpoint: endpoint, clientID: credential.clientID, apiMajor: 1, query: query)
    let current = now()
    do {
      let page = try await transport.read(query: query, credential: credential)
      guard apiMajor(from: page.apiVersion) == 1 else {
        throw TransportFailure.incompatibleVersion(
          "projection response API \(page.apiVersion) does not match client major 1"
        )
      }
      try await cache.put(page, for: key, fetchedAt: current, expiresAt: current.addingTimeInterval(cacheLifetime))
      return .live(page: page, age: 0)
    } catch let failure as TransportFailure {
      return await staleOrUnknown(key: key, now: current, reason: failure.reason)
    } catch {
      return await staleOrUnknown(key: key, now: current, reason: "projection read failed")
    }
  }

  private func staleOrUnknown(
    key: ProjectionCacheKey,
    now: Date,
    reason: String
  ) async -> ProjectionReadState {
    guard let cached = await cache.get(key) else {
      return .unknown(reason: "projection unavailable: \(reason)")
    }
    return .stale(
      page: cached.page,
      age: max(0, now.timeIntervalSince(cached.fetchedAt)),
      reason: reason
    )
  }

  private func apiMajor(from version: String) -> UInt? {
    guard let major = version.split(separator: ".").first else { return nil }
    return UInt(major)
  }
}

#if canImport(CryptoKit)

public final class URLSessionProjectionTransport: ProjectionReading, @unchecked Sendable {
  private let origin: ClientOrigin
  private let authenticator: RequestAuthenticator
  private let session: URLSession

  public init(origin: ClientOrigin, authenticator: RequestAuthenticator) {
    self.origin = origin
    self.authenticator = authenticator
    self.session = URLSession(configuration: .ephemeral)
  }

  public func read(
    query: ClientProjectionQuery,
    credential: StoredDeviceCredential
  ) async throws -> ClientProjectionPage {
    guard var components = URLComponents(url: origin.url, resolvingAgainstBaseURL: false) else {
      throw TransportFailure.network("projection URL could not be formed")
    }
    components.path = "/api/v1/client/projections"
    components.queryItems = [
      URLQueryItem(name: "metric", value: query.metric),
      URLQueryItem(name: "window_start", value: query.windowStart),
      URLQueryItem(name: "window_end", value: query.windowEnd),
      URLQueryItem(name: "cursor", value: query.cursor),
    ].filter { $0.value != nil }
    guard let url = components.url else {
      throw TransportFailure.network("projection query URL could not be formed")
    }
    var request = URLRequest(url: url)
    request.httpMethod = "GET"
    request.setValue("application/json", forHTTPHeaderField: "Accept")
    let path = components.percentEncodedPath
      + (components.percentEncodedQuery.map { "?\($0)" } ?? "")
    for (field, value) in try authenticator.headers(
      body: Data(), credential: credential, now: Date(), method: "GET", path: path
    ) {
      request.setValue(value, forHTTPHeaderField: field)
    }
    do {
      let (data, response) = try await session.data(for: request)
      guard data.count <= 2 * 1024 * 1024 else {
        throw TransportFailure.responseTooLarge("projection response was \(data.count) bytes")
      }
      guard let http = response as? HTTPURLResponse else {
        throw TransportFailure.network("projection response was not HTTP")
      }
      guard (200..<300).contains(http.statusCode) else {
        let error = try? JSONDecoder().decode(ClientErrorResponse.self, from: data)
        if let error {
          throw TransportFailure.server(code: error.code, retryable: error.retryable)
        }
        throw TransportFailure.http(
          status: http.statusCode,
          retryAfter: http.value(forHTTPHeaderField: "Retry-After").flatMap(TimeInterval.init)
        )
      }
      return try JSONDecoder().decode(ClientProjectionPage.self, from: data)
    } catch let failure as TransportFailure {
      throw failure
    } catch {
      throw TransportFailure.network("projection request failed: \(error.localizedDescription)")
    }
  }
}
#endif
