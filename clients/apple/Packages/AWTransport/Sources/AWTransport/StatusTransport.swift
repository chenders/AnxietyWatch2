import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

/// Sends operational phone evidence through the same authenticated request profile as writes.
public protocol ClientStatusTransport: Sendable {
  func send(
    beacon: HealthBeaconRequest,
    credential: StoredDeviceCredential
  ) async throws -> HealthBeaconResponse
}

#if canImport(CryptoKit)
/// URLSession transport for the authenticated phone status endpoint.
public final class URLSessionClientStatusTransport: ClientStatusTransport, @unchecked Sendable {
  private let origin: ClientOrigin
  private let authenticator: RequestAuthenticator
  private let session: URLSession

  public init(origin: ClientOrigin, authenticator: RequestAuthenticator) {
    self.origin = origin
    self.authenticator = authenticator
    self.session = URLSession(configuration: .ephemeral)
  }

  public func send(
    beacon: HealthBeaconRequest,
    credential: StoredDeviceCredential
  ) async throws -> HealthBeaconResponse {
    var url = origin.url
    url.append(path: "api/v1/client/status")
    var request = URLRequest(url: url)
    request.httpMethod = "POST"
    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
    let body = try JSONEncoder().encode(beacon)
    request.httpBody = body
    for (field, value) in try authenticator.headers(
      body: body,
      credential: credential,
      now: Date(),
      method: "POST",
      path: "/api/v1/client/status"
    ) {
      request.setValue(value, forHTTPHeaderField: field)
    }
    let (data, response) = try await session.data(for: request)
    guard data.count <= 256 * 1024 else {
      throw TransportFailure.responseTooLarge("status response was \(data.count) bytes")
    }
    guard let http = response as? HTTPURLResponse else {
      throw TransportFailure.network("status response was not HTTP")
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
      return try JSONDecoder().decode(HealthBeaconResponse.self, from: data)
    } catch {
      throw TransportFailure.network("status response did not match the client contract")
    }
  }
}
#endif
