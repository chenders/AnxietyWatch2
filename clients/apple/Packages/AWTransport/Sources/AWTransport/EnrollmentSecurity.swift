import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

/// Errors are intentionally named for the state that blocked enrollment. Secret material is never
/// included in an error description.
public enum EnrollmentError: Error, Equatable, Sendable {
  case invalidOrigin(String)
  case invalidArtifact(String)
  case confirmationRequired
  case identityMismatch(String)
  case artifactExpired(String)
  case secureEnclaveUnavailable
  case keychainFailure(String)
  case serverRejected(String)
  case apiVersionIncompatible(String)
  case responseTooLarge(String)
  case redirectRejected(String)
}

/// The only origin accepted by production enrollment: an HTTPS DNS origin on port 443.
public struct ClientOrigin: Codable, Equatable, Hashable, Sendable {
  public let url: URL

  public init(_ url: URL) throws {
    guard url.scheme?.lowercased() == "https" else {
      throw EnrollmentError.invalidOrigin("scheme must be https; received \(url.scheme ?? "missing")")
    }
    guard url.user == nil else {
      throw EnrollmentError.invalidOrigin("embedded user information is not allowed")
    }
    guard let host = url.host?.lowercased(), !host.isEmpty else {
      throw EnrollmentError.invalidOrigin("DNS hostname is missing")
    }
    guard url.port == nil || url.port == 443 else {
      throw EnrollmentError.invalidOrigin("port must be 443; received \(url.port ?? -1)")
    }
    guard url.path.isEmpty || url.path == "/" else {
      throw EnrollmentError.invalidOrigin("origin path must be empty or /; received \(url.path)")
    }
    guard url.query == nil, url.fragment == nil else {
      throw EnrollmentError.invalidOrigin("query and fragment are not allowed in an origin")
    }
    guard !host.contains(":"), !isIPv4Literal(host), host != "localhost" else {
      throw EnrollmentError.invalidOrigin("host must be a public DNS name; received \(host)")
    }
    guard host.unicodeScalars.allSatisfy({ $0.value < 128 }) else {
      throw EnrollmentError.invalidOrigin("host must use ASCII DNS labels; received \(host)")
    }

    let labels = host.split(separator: ".", omittingEmptySubsequences: false)
    guard labels.count >= 2, labels.allSatisfy(isValidDNSLabel) else {
      throw EnrollmentError.invalidOrigin("host must contain valid DNS labels; received \(host)")
    }

    self.url = URL(string: "https://\(host)")!
  }

  public init(string: String) throws {
    guard let url = URL(string: string) else {
      throw EnrollmentError.invalidOrigin("URL could not be parsed")
    }
    try self.init(url)
  }

  public var normalizedString: String { url.absoluteString }

  public func encode(to encoder: Encoder) throws {
    var container = encoder.singleValueContainer()
    try container.encode(normalizedString)
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.singleValueContainer()
    try self.init(string: container.decode(String.self))
  }
}

private func isIPv4Literal(_ host: String) -> Bool {
  let components = host.split(separator: ".", omittingEmptySubsequences: false)
  guard components.count == 4 else { return false }
  return components.allSatisfy { component in
    guard !component.isEmpty, component.count <= 3, component.allSatisfy(\.isNumber),
      let value = Int(component)
    else { return false }
    return value <= 255
  }
}

private func isValidDNSLabel(_ label: Substring) -> Bool {
  guard !label.isEmpty, label.count <= 63,
    label.first != "-", label.last != "-"
  else { return false }
  return label.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" }
}

public struct EnrollmentArtifact: Codable, Equatable, Sendable {
  public let apiMajor: UInt
  public let origin: ClientOrigin
  public let serverInstanceID: UUID
  public let enrollmentID: UUID
  public let token: String
  public let serverDisplayLabel: String
  public let scopes: [String]
  public let expiresAt: Date

  public init(
    apiMajor: UInt,
    origin: ClientOrigin,
    serverInstanceID: UUID,
    enrollmentID: UUID,
    token: String,
    serverDisplayLabel: String,
    scopes: [String],
    expiresAt: Date
  ) throws {
    guard apiMajor > 0 else { throw EnrollmentError.invalidArtifact("api major must be positive") }
    guard !token.isEmpty else { throw EnrollmentError.invalidArtifact("one-time token is missing") }
    guard !serverDisplayLabel.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw EnrollmentError.invalidArtifact("server display label is missing")
    }
    guard !scopes.isEmpty else { throw EnrollmentError.invalidArtifact("requested scopes are empty") }
    self.apiMajor = apiMajor
    self.origin = origin
    self.serverInstanceID = serverInstanceID
    self.enrollmentID = enrollmentID
    self.token = token
    self.serverDisplayLabel = serverDisplayLabel
    self.scopes = scopes
    self.expiresAt = expiresAt
  }

  public func validate(at now: Date) throws {
    guard expiresAt > now else {
      throw EnrollmentError.artifactExpired("enrollment artifact expired at \(expiresAt)")
    }
  }
}

public struct EnrollmentConfirmation: Equatable, Sendable {
  public let artifact: EnrollmentArtifact
  public let confirmedAt: Date

  public init(
    artifact: EnrollmentArtifact,
    confirmedOrigin: ClientOrigin,
    confirmedServerInstanceID: UUID,
    accepted: Bool,
    now: Date
  ) throws {
    guard accepted else { throw EnrollmentError.confirmationRequired }
    guard artifact.origin == confirmedOrigin else {
      throw EnrollmentError.identityMismatch("confirmed origin does not match the artifact origin")
    }
    guard artifact.serverInstanceID == confirmedServerInstanceID else {
      throw EnrollmentError.identityMismatch("confirmed server instance does not match the artifact")
    }
    try artifact.validate(at: now)
    self.artifact = artifact
    self.confirmedAt = now
  }
}

public struct DeviceKeyReference: Codable, Equatable, Sendable {
  public let reference: String
  public let publicKeySPKIDERBase64: String

  public init(reference: String, publicKeySPKIDERBase64: String) throws {
    guard !reference.isEmpty else { throw EnrollmentError.keychainFailure("key reference is missing") }
    guard !publicKeySPKIDERBase64.isEmpty else {
      throw EnrollmentError.keychainFailure("public key representation is missing")
    }
    self.reference = reference
    self.publicKeySPKIDERBase64 = publicKeySPKIDERBase64
  }
}

public protocol DeviceKeyStore: Sendable {
  func createKey() throws -> DeviceKeyReference
  func sign(_ message: Data, with reference: DeviceKeyReference) throws -> Data
  func deleteKey(_ reference: DeviceKeyReference) throws
}

public protocol DeviceCredentialStore: Sendable {
  func load() throws -> StoredDeviceCredential?
  func save(_ credential: StoredDeviceCredential) throws
  func delete() throws
}

public struct StoredDeviceCredential: Codable, Equatable, Sendable {
  public let clientID: UUID
  public let keyID: UUID
  public let keyReference: DeviceKeyReference
  public let scopes: [String]
  public let issuedAt: String
  public let expiresAt: String
  public let serverTime: String

  public init(
    clientID: UUID,
    keyID: UUID,
    keyReference: DeviceKeyReference,
    scopes: [String],
    issuedAt: String,
    expiresAt: String,
    serverTime: String
  ) throws {
    guard !scopes.isEmpty else { throw EnrollmentError.invalidArtifact("credential scopes are empty") }
    guard !issuedAt.isEmpty, !expiresAt.isEmpty, !serverTime.isEmpty else {
      throw EnrollmentError.invalidArtifact("credential timestamps are incomplete")
    }
    self.clientID = clientID
    self.keyID = keyID
    self.keyReference = keyReference
    self.scopes = scopes
    self.issuedAt = issuedAt
    self.expiresAt = expiresAt
    self.serverTime = serverTime
  }
}

public protocol EnrollmentClaiming: Sendable {
  func claim(_ request: EnrollmentClaimRequest) async throws -> EnrollmentCredentialResponse
}

public struct EnrollmentCredentialResponse: Codable, Equatable, Sendable {
  public let apiVersion: String
  public let clientID: UUID
  public let keyID: UUID
  public let scopes: [String]
  public let issuedAt: String
  public let expiresAt: String
  public let serverTime: String

  public init(
    apiVersion: String,
    clientID: UUID,
    keyID: UUID,
    scopes: [String],
    issuedAt: String,
    expiresAt: String,
    serverTime: String
  ) {
    self.apiVersion = apiVersion
    self.clientID = clientID
    self.keyID = keyID
    self.scopes = scopes
    self.issuedAt = issuedAt
    self.expiresAt = expiresAt
    self.serverTime = serverTime
  }

  enum CodingKeys: String, CodingKey {
    case apiVersion = "api_version"
    case clientID = "client_id"
    case keyID = "key_id"
    case scopes
    case issuedAt = "issued_at"
    case expiresAt = "expires_at"
    case serverTime = "server_time"
  }
}

public actor EnrollmentCoordinator {
  private let keyStore: any DeviceKeyStore
  private let credentialStore: any DeviceCredentialStore
  private let transport: any EnrollmentClaiming

  public init(
    keyStore: any DeviceKeyStore,
    credentialStore: any DeviceCredentialStore,
    transport: any EnrollmentClaiming
  ) {
    self.keyStore = keyStore
    self.credentialStore = credentialStore
    self.transport = transport
  }

  public func enroll(
    confirmation: EnrollmentConfirmation,
    deviceLabel: String,
    appVersion: String
  ) async throws -> StoredDeviceCredential {
    guard !deviceLabel.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw EnrollmentError.invalidArtifact("device label is missing")
    }
    guard !appVersion.isEmpty else { throw EnrollmentError.invalidArtifact("app version is missing") }

    let key = try keyStore.createKey()
    do {
      let artifact = confirmation.artifact
      let request = EnrollmentClaimRequest(
        apiMajor: artifact.apiMajor,
        enrollmentID: artifact.enrollmentID,
        token: artifact.token,
        serverInstanceID: artifact.serverInstanceID,
        deviceLabel: deviceLabel,
        publicKeySPKIDERBase64: key.publicKeySPKIDERBase64,
        algorithm: "ecdsa-p256-sha256",
        appVersion: appVersion
      )
      let response = try await transport.claim(request)
      guard apiMajor(from: response.apiVersion) == artifact.apiMajor else {
        throw EnrollmentError.apiVersionIncompatible(
          "server API \(response.apiVersion) does not match requested major \(artifact.apiMajor)"
        )
      }
      let credential = try StoredDeviceCredential(
        clientID: response.clientID,
        keyID: response.keyID,
        keyReference: key,
        scopes: response.scopes,
        issuedAt: response.issuedAt,
        expiresAt: response.expiresAt,
        serverTime: response.serverTime
      )
      try credentialStore.save(credential)
      return credential
    } catch {
      try? keyStore.deleteKey(key)
      throw error
    }
  }

  private func apiMajor(from version: String) -> UInt? {
    guard let major = version.split(separator: ".").first else { return nil }
    return UInt(major)
  }
}

#if canImport(Security) && canImport(CryptoKit)
import CryptoKit
import Security

/// Production key custody. Secure Enclave availability is checked rather than silently falling back
/// to a software key, and the Keychain item is device-bound and usable after first unlock.
public final class SecureEnclaveKeyStore: DeviceKeyStore, @unchecked Sendable {
  public init() {}

  public func createKey() throws -> DeviceKeyReference {
    guard SecureEnclave.isAvailable else { throw EnrollmentError.secureEnclaveUnavailable }
    let privateKey: SecureEnclave.P256.Signing.PrivateKey
    do {
      privateKey = try SecureEnclave.P256.Signing.PrivateKey()
    } catch {
      throw EnrollmentError.keychainFailure("Secure Enclave key creation failed: \(error.localizedDescription)")
    }
    let reference = UUID().uuidString
    let tag = Data(reference.utf8)
    let query: [CFString: Any] = [
      kSecClass: kSecClassKey,
      kSecAttrKeyType: kSecAttrKeyTypeECSECPrimeRandom,
      kSecAttrKeyClass: kSecAttrKeyClassPrivate,
      kSecAttrApplicationTag: tag,
      kSecAttrAccessible: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
      kSecValueData: privateKey.dataRepresentation,
    ]
    let status = SecItemAdd(query as CFDictionary, nil)
    guard status == errSecSuccess else {
      throw EnrollmentError.keychainFailure("Secure Enclave key storage failed with status \(status)")
    }
    return try DeviceKeyReference(
      reference: reference,
      publicKeySPKIDERBase64: privateKey.publicKey.derRepresentation.base64EncodedString()
    )
  }

  public func sign(_ message: Data, with reference: DeviceKeyReference) throws -> Data {
    let key = try load(reference)
    do {
      return try key.signature(for: message).derRepresentation
    } catch {
      throw EnrollmentError.keychainFailure("Secure Enclave signing failed: \(error.localizedDescription)")
    }
  }

  public func deleteKey(_ reference: DeviceKeyReference) throws {
    let status = SecItemDelete(keyQuery(reference) as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else {
      throw EnrollmentError.keychainFailure("key deletion failed with status \(status)")
    }
  }

  private func load(_ reference: DeviceKeyReference) throws -> SecureEnclave.P256.Signing.PrivateKey {
    var result: CFTypeRef?
    var query = keyQuery(reference)
    query[kSecReturnData] = true
    let status = SecItemCopyMatching(query as CFDictionary, &result)
    guard status == errSecSuccess, let data = result as? Data else {
      throw EnrollmentError.keychainFailure("key reference \(reference.reference) was unavailable with status \(status)")
    }
    do {
      return try SecureEnclave.P256.Signing.PrivateKey(dataRepresentation: data)
    } catch {
      throw EnrollmentError.keychainFailure("stored Secure Enclave key could not be loaded")
    }
  }

  private func keyQuery(_ reference: DeviceKeyReference) -> [CFString: Any] {
    [
      kSecClass: kSecClassKey,
      kSecAttrKeyType: kSecAttrKeyTypeECSECPrimeRandom,
      kSecAttrKeyClass: kSecAttrKeyClassPrivate,
      kSecAttrApplicationTag: Data(reference.reference.utf8),
    ]
  }
}

public final class KeychainCredentialStore: DeviceCredentialStore, @unchecked Sendable {
  private let service = "com.groundeffectsoftware.anxietywatch.client-credential"
  private let account = "active"

  public init() {}

  public func load() throws -> StoredDeviceCredential? {
    var result: CFTypeRef?
    var query = baseQuery()
    query[kSecReturnData] = true
    let status = SecItemCopyMatching(query as CFDictionary, &result)
    if status == errSecItemNotFound { return nil }
    guard status == errSecSuccess, let data = result as? Data else {
      throw EnrollmentError.keychainFailure("credential read failed with status \(status)")
    }
    do {
      return try JSONDecoder().decode(StoredDeviceCredential.self, from: data)
    } catch {
      throw EnrollmentError.keychainFailure("stored credential metadata is invalid")
    }
  }

  public func save(_ credential: StoredDeviceCredential) throws {
    let data: Data
    do {
      data = try JSONEncoder().encode(credential)
    } catch {
      throw EnrollmentError.keychainFailure("credential metadata could not be encoded")
    }
    let attributes: [CFString: Any] = [
      kSecValueData: data,
      kSecAttrAccessible: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
    ]
    let updateStatus = SecItemUpdate(baseQuery() as CFDictionary, attributes as CFDictionary)
    if updateStatus == errSecItemNotFound {
      var query = baseQuery()
      query[kSecValueData] = data
      query[kSecAttrAccessible] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
      let addStatus = SecItemAdd(query as CFDictionary, nil)
      guard addStatus == errSecSuccess else {
        throw EnrollmentError.keychainFailure("credential storage failed with status \(addStatus)")
      }
    } else if updateStatus != errSecSuccess {
      throw EnrollmentError.keychainFailure("credential update failed with status \(updateStatus)")
    }
  }

  public func delete() throws {
    let status = SecItemDelete(baseQuery() as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else {
      throw EnrollmentError.keychainFailure("credential deletion failed with status \(status)")
    }
  }

  private func baseQuery() -> [CFString: Any] {
    [kSecClass: kSecClassGenericPassword, kSecAttrService: service, kSecAttrAccount: account]
  }
}
#else

/// The production implementation is intentionally unavailable on platforms without Apple Security.
public final class SecureEnclaveKeyStore: DeviceKeyStore, @unchecked Sendable {
  public init() {}
  public func createKey() throws -> DeviceKeyReference { throw EnrollmentError.secureEnclaveUnavailable }
  public func sign(_ message: Data, with reference: DeviceKeyReference) throws -> Data {
    throw EnrollmentError.secureEnclaveUnavailable
  }
  public func deleteKey(_ reference: DeviceKeyReference) throws {}
}

public final class KeychainCredentialStore: DeviceCredentialStore, @unchecked Sendable {
  public init() {}
  public func load() throws -> StoredDeviceCredential? { throw EnrollmentError.keychainFailure("Apple Keychain is unavailable") }
  public func save(_ credential: StoredDeviceCredential) throws {
    throw EnrollmentError.keychainFailure("Apple Keychain is unavailable")
  }
  public func delete() throws {}
}
#endif

private final class RedirectRejectingDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
  func urlSession(
    _ session: URLSession,
    task: URLSessionTask,
    willPerformHTTPRedirection response: HTTPURLResponse,
    newRequest request: URLRequest,
    completionHandler: @escaping (URLRequest?) -> Void
  ) {
    completionHandler(nil)
  }
}

/// TLS is delegated to URLSession's system trust and hostname validation. This class deliberately
/// provides no trust override and rejects enrollment redirects.
public final class URLSessionEnrollmentTransport: EnrollmentClaiming, @unchecked Sendable {
  private let origin: ClientOrigin
  private let session: URLSession

  public init(origin: ClientOrigin) {
    self.origin = origin
    self.session = URLSession(
      configuration: .ephemeral,
      delegate: RedirectRejectingDelegate(),
      delegateQueue: nil
    )
  }

  public func claim(_ request: EnrollmentClaimRequest) async throws -> EnrollmentCredentialResponse {
    var url = origin.url
    url.append(path: "api/v1/client/enroll")
    var urlRequest = URLRequest(url: url)
    urlRequest.httpMethod = "POST"
    urlRequest.setValue("application/json", forHTTPHeaderField: "Content-Type")
    urlRequest.setValue("1", forHTTPHeaderField: "X-AnxietyWatch-API-Version")
    do {
      urlRequest.httpBody = try JSONEncoder().encode(request)
    } catch {
      throw EnrollmentError.serverRejected("enrollment request could not be encoded")
    }

    let (data, response): (Data, URLResponse)
    do {
      (data, response) = try await session.data(for: urlRequest)
    } catch {
      throw EnrollmentError.serverRejected("TLS or network request failed: \(error.localizedDescription)")
    }
    guard data.count <= 128 * 1024 else {
      throw EnrollmentError.responseTooLarge("enrollment response was \(data.count) bytes")
    }
    guard let httpResponse = response as? HTTPURLResponse else {
      throw EnrollmentError.serverRejected("enrollment response was not HTTP")
    }
    guard (200..<300).contains(httpResponse.statusCode) else {
      let errorResponse = try? JSONDecoder().decode(ClientErrorResponse.self, from: data)
      throw EnrollmentError.serverRejected(
        errorResponse.map { "\($0.code): \($0.blocker)" } ?? "HTTP status \(httpResponse.statusCode)"
      )
    }
    do {
      return try JSONDecoder().decode(EnrollmentCredentialResponse.self, from: data)
    } catch {
      throw EnrollmentError.serverRejected("enrollment response did not match the credential contract")
    }
  }
}
