import AWClientCore

public struct APIVersion: Equatable, Sendable {
  public let major: UInt
  public let minor: UInt

  public init(major: UInt, minor: UInt) {
    self.major = major
    self.minor = minor
  }

  public func isCompatible(with server: APIVersion) -> Bool {
    major == server.major
  }
}
