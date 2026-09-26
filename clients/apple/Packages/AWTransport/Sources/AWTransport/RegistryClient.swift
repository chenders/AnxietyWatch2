import AWHealthKit
import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

/// The authenticated registry-discovery response.
public struct RegistryDiscoveryResponse: Codable, Equatable, Sendable {
  public let apiVersion: String
  public let registrySHA256: String
  public let document: RegistryDocument
  public let serverTime: String

  enum CodingKeys: String, CodingKey {
    case apiVersion = "api_version"
    case registrySHA256 = "registry_sha256"
    case document
    case serverTime = "server_time"
  }
}

/// Fetches the server-owned registry using the enrolled device credential.
public protocol RegistryReading: Sendable {
  func readRegistry(credential: StoredDeviceCredential) async throws -> RegistryDiscoveryResponse
}

/// The result always distinguishes a fresh registry, a stale cached registry, and no registry.
public enum RegistryReadState: Equatable, Sendable {
  case live(document: RegistryDocument)
  case stale(document: RegistryDocument, reason: String)
  case unknown(reason: String)
}

/// Fetches and validates the registry while retaining a last-known-good cache.
public actor RegistryReadService {
  private let cache: RegistryCache
  private let transport: any RegistryReading

  public init(cache: RegistryCache, transport: any RegistryReading) {
    self.cache = cache
    self.transport = transport
  }

  public func read(credential: StoredDeviceCredential) async -> RegistryReadState {
    do {
      let response = try await transport.readRegistry(credential: credential)
      guard response.apiVersion.split(separator: ".").first.flatMap({ UInt($0) }) == 1 else {
        throw RegistryError.invalidAPIVersion(response.apiVersion)
      }
      try await cache.put(
        response.document,
        apiVersion: response.apiVersion,
        fetchedAt: Date()
      )
      return .live(document: response.document)
    } catch {
      guard let cached = await cache.get() else {
        return .unknown(reason: "registry unavailable: fetch or validation failed")
      }
      return .stale(document: cached.document, reason: "registry refresh failed")
    }
  }
}

#if canImport(CryptoKit)

/// URLSession implementation of authenticated registry discovery.
public final class URLSessionRegistryTransport: RegistryReading, @unchecked Sendable {
  private let origin: ClientOrigin
  private let authenticator: RequestAuthenticator
  private let session: URLSession

  public init(origin: ClientOrigin, authenticator: RequestAuthenticator) {
    self.origin = origin
    self.authenticator = authenticator
    self.session = URLSession(configuration: .ephemeral)
  }

  public func readRegistry(credential: StoredDeviceCredential) async throws -> RegistryDiscoveryResponse {
    guard var components = URLComponents(url: origin.url, resolvingAgainstBaseURL: false) else {
      throw TransportFailure.network("registry URL could not be formed")
    }
    components.path = "/api/v1/client/registry"
    guard let url = components.url else {
      throw TransportFailure.network("registry URL could not be formed")
    }
    var request = URLRequest(url: url)
    request.httpMethod = "GET"
    request.setValue("application/json", forHTTPHeaderField: "Accept")
    for (field, value) in try authenticator.headers(
      body: Data(), credential: credential, now: Date(), method: "GET", path: components.percentEncodedPath
    ) {
      request.setValue(value, forHTTPHeaderField: field)
    }
    do {
      let (data, response) = try await session.data(for: request)
      guard data.count <= 2 * 1024 * 1024 else {
        throw TransportFailure.responseTooLarge("registry response was \(data.count) bytes")
      }
      guard let http = response as? HTTPURLResponse else {
        throw TransportFailure.network("registry response was not HTTP")
      }
      guard (200..<300).contains(http.statusCode) else {
        throw TransportFailure.http(
          status: http.statusCode,
          retryAfter: http.value(forHTTPHeaderField: "Retry-After").flatMap(TimeInterval.init)
        )
      }
      return try RegistryDiscoveryResponseDecoder.decode(data)
    } catch let failure as TransportFailure {
      throw failure
    } catch {
      throw TransportFailure.network("registry request failed: \(error.localizedDescription)")
    }
  }
}

private enum RegistryDiscoveryResponseDecoder {
  static func decode(_ data: Data) throws -> RegistryDiscoveryResponse {
    let response = try JSONDecoder().decode(RegistryDiscoveryResponse.self, from: data)
    try response.document.validate()
    return response
  }
}
#endif
