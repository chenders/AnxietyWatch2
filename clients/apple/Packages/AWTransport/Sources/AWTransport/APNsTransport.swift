import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

/// The APNs token is registered as lowercase hex; it is never logged or shown in the UI.
public struct APNsRegistrationRequest: Codable, Equatable, Sendable {
  public let deviceID: UUID
  public let token: String?
  public let environment: String
  public let appIdentifier: String
  public let appVersion: String
  public let build: String
  public let authorizationStatus: String
  public let alertsEnabled: Bool
  public let soundsEnabled: Bool
  public let badgesEnabled: Bool

  public init(
    deviceID: UUID,
    token: String?,
    environment: String,
    appIdentifier: String,
    appVersion: String,
    build: String,
    authorizationStatus: String,
    alertsEnabled: Bool,
    soundsEnabled: Bool,
    badgesEnabled: Bool
  ) throws {
    guard UUID(uuidString: deviceID.uuidString) != nil else {
      throw APNsError.invalidRegistration("device identity is invalid")
    }
    if let token {
      guard token.count >= 2, token.count <= 512,
        token.allSatisfy({ $0.isHexDigit })
      else {
        throw APNsError.invalidRegistration("APNs token is not hexadecimal")
      }
    }
    guard environment == "sandbox" || environment == "production" else {
      throw APNsError.invalidRegistration("APNs environment is unsupported: \(environment)")
    }
    guard !appIdentifier.isEmpty, !appVersion.isEmpty, !build.isEmpty else {
      throw APNsError.invalidRegistration("app identity is incomplete")
    }
    guard !authorizationStatus.isEmpty else {
      throw APNsError.invalidRegistration("notification authorization status is missing")
    }
    self.deviceID = deviceID
    self.token = token?.lowercased()
    self.environment = environment
    self.appIdentifier = appIdentifier
    self.appVersion = appVersion
    self.build = build
    self.authorizationStatus = authorizationStatus
    self.alertsEnabled = alertsEnabled
    self.soundsEnabled = soundsEnabled
    self.badgesEnabled = badgesEnabled
  }

  enum CodingKeys: String, CodingKey {
    case deviceID = "device_id"
    case token, environment
    case appIdentifier = "app_identifier"
    case appVersion = "app_version"
    case build
    case authorizationStatus = "authorization_status"
    case alertsEnabled = "alerts_enabled"
    case soundsEnabled = "sounds_enabled"
    case badgesEnabled = "badges_enabled"
  }
}

public struct APNsRegistrationResponse: Codable, Equatable, Sendable {
  public let apiVersion: String
  public let status: String
  public let serverTime: String

  enum CodingKeys: String, CodingKey {
    case apiVersion = "api_version"
    case status
    case serverTime = "server_time"
  }
}

public enum APNsMessageOutcome: String, Codable, Equatable, Sendable {
  case displayed
  case acknowledged
}

/// A client outcome never claims APNs provider acceptance or delivery. Those facts are recorded by
/// the Pi/provider ledger; the app can only report display and user acknowledgement.
public struct APNsOutcomeRequest: Codable, Equatable, Sendable {
  public let messageID: UUID
  public let outcome: APNsMessageOutcome
  public let observedAt: String

  public init(messageID: UUID, outcome: APNsMessageOutcome, observedAt: String) throws {
    guard !observedAt.isEmpty else {
      throw APNsError.invalidOutcome("observation time is missing")
    }
    self.messageID = messageID
    self.outcome = outcome
    self.observedAt = observedAt
  }

  enum CodingKeys: String, CodingKey {
    case messageID = "message_id"
    case outcome
    case observedAt = "observed_at"
  }
}

public struct APNsOutcomeResponse: Codable, Equatable, Sendable {
  public let apiVersion: String
  public let messageID: UUID
  public let status: String
  public let serverTime: String

  enum CodingKeys: String, CodingKey {
    case apiVersion = "api_version"
    case messageID = "message_id"
    case status
    case serverTime = "server_time"
  }
}

public enum APNsError: Error, Equatable, Sendable {
  case invalidRegistration(String)
  case invalidMessage(String)
  case invalidOutcome(String)
}

/// The only application data accepted in a remote message. Protected content is fetched after
/// unlock from the Pi; it is never carried in the APNs payload.
public struct APNsMessage: Equatable, Sendable {
  public let messageID: UUID
  public let payloadClass: String
  public let category: String?
  public let deepLink: String?
  public let isTest: Bool

  public init(
    messageID: UUID,
    payloadClass: String,
    category: String?,
    deepLink: String?,
    isTest: Bool
  ) throws {
    guard !payloadClass.isEmpty, payloadClass.count <= 128 else {
      throw APNsError.invalidMessage("payload class is missing or too long")
    }
    if let category, category.count > 128 {
      throw APNsError.invalidMessage("notification category is too long")
    }
    if let deepLink, deepLink.count > 512 {
      throw APNsError.invalidMessage("protected deep link is too long")
    }
    self.messageID = messageID
    self.payloadClass = payloadClass
    self.category = category
    self.deepLink = deepLink
    self.isTest = isTest
  }

  /// Decodes the APNs user-info dictionary without accepting sensitive content fields. The `aps`
  /// dictionary is intentionally ignored; presentation is decided by iOS and the app delegate.
  public init(userInfo: [AnyHashable: Any]) throws {
    let allowedKeys: Set<String> = ["aps", "message_id", "payload_class", "category", "deep_link", "is_test"]
    for key in userInfo.keys.compactMap({ $0 as? String }) where !allowedKeys.contains(key.lowercased()) {
      throw APNsError.invalidMessage("remote payload contains unsupported field \(key)")
    }
    guard let rawID = userInfo["message_id"] as? String,
      let messageID = UUID(uuidString: rawID),
      let payloadClass = userInfo["payload_class"] as? String
    else {
      throw APNsError.invalidMessage("message identity or payload class is missing")
    }
    try self.init(
      messageID: messageID,
      payloadClass: payloadClass,
      category: userInfo["category"] as? String,
      deepLink: userInfo["deep_link"] as? String,
      isTest: userInfo["is_test"] as? Bool ?? false
    )
  }
}

/// Prevents duplicate display/acknowledgement reports for one message while retaining the two
/// outcomes as independent facts.
public actor APNsOutcomeLedger {
  private var reported: Set<String> = []
  private let maximumEntries: Int

  public init(maximumEntries: Int = 512) throws {
    guard maximumEntries > 0 else {
      throw APNsError.invalidOutcome("outcome ledger capacity must be positive")
    }
    self.maximumEntries = maximumEntries
  }

  public func shouldReport(messageID: UUID, outcome: APNsMessageOutcome) -> Bool {
    let key = "\(messageID.uuidString.lowercased()):\(outcome.rawValue)"
    guard !reported.contains(key) else { return false }
    if reported.count >= maximumEntries {
      reported.removeFirst()
    }
    reported.insert(key)
    return true
  }
}

public protocol APNsNotificationTransport: Sendable {
  func register(
    request: APNsRegistrationRequest,
    credential: StoredDeviceCredential
  ) async throws -> APNsRegistrationResponse

  func report(
    outcome: APNsOutcomeRequest,
    credential: StoredDeviceCredential
  ) async throws -> APNsOutcomeResponse
}

#if canImport(CryptoKit)
/// Authenticated client transport for registration and display/acknowledgement evidence.
public final class URLSessionAPNsTransport: APNsNotificationTransport, @unchecked Sendable {
  private let origin: ClientOrigin
  private let authenticator: RequestAuthenticator
  private let session: URLSession

  public init(origin: ClientOrigin, authenticator: RequestAuthenticator) {
    self.origin = origin
    self.authenticator = authenticator
    self.session = URLSession(configuration: .ephemeral)
  }

  public func register(
    request: APNsRegistrationRequest,
    credential: StoredDeviceCredential
  ) async throws -> APNsRegistrationResponse {
    try await post(
      request,
      response: APNsRegistrationResponse.self,
      path: "/api/v1/client/notifications/apns/registration",
      credential: credential
    )
  }

  public func report(
    outcome: APNsOutcomeRequest,
    credential: StoredDeviceCredential
  ) async throws -> APNsOutcomeResponse {
    try await post(
      outcome,
      response: APNsOutcomeResponse.self,
      path: "/api/v1/client/notifications/apns/outcomes",
      credential: credential
    )
  }

  private func post<Request: Encodable, Response: Decodable>(
    _ body: Request,
    response: Response.Type,
    path: String,
    credential: StoredDeviceCredential
  ) async throws -> Response {
    var url = origin.url
    url.append(path: String(path.dropFirst()))
    var request = URLRequest(url: url)
    request.httpMethod = "POST"
    let bodyData = try JSONEncoder().encode(body)
    request.httpBody = bodyData
    for (field, value) in try authenticator.headers(
      body: bodyData,
      credential: credential,
      now: Date(),
      method: "POST",
      path: path
    ) {
      request.setValue(value, forHTTPHeaderField: field)
    }
    let (data, urlResponse) = try await session.data(for: request)
    guard data.count <= 256 * 1024 else {
      throw TransportFailure.responseTooLarge("APNs response was \(data.count) bytes")
    }
    guard let http = urlResponse as? HTTPURLResponse else {
      throw TransportFailure.network("APNs response was not HTTP")
    }
    guard (200..<300).contains(http.statusCode) else {
      let error = try? JSONDecoder().decode(ClientErrorResponse.self, from: data)
      let retryAfter = http.value(forHTTPHeaderField: "Retry-After").flatMap(TimeInterval.init)
      if let error {
        throw TransportFailure.server(code: error.code, retryable: error.retryable, retryAfter: retryAfter)
      }
      throw TransportFailure.http(status: http.statusCode, retryAfter: retryAfter)
    }
    do {
      return try JSONDecoder().decode(Response.self, from: data)
    } catch {
      throw TransportFailure.network("APNs response did not match the client contract")
    }
  }
}
#endif
