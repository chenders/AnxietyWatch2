import Foundation

/// A server-projected value never substitutes zero for unknown or missing data.
public enum Observation<Value: Equatable & Sendable>: Equatable, Sendable {
  case unknown
  case current(value: Value, observedAt: Date)
  case stale(value: Value, observedAt: Date, asOf: Date)

  public var value: Value? {
    switch self {
    case .unknown:
      nil
    case .current(let value, _), .stale(let value, _, _):
      value
    }
  }

  public var isStale: Bool {
    if case .stale = self { true } else { false }
  }
}

/// Build facts sent with diagnostics and status beacons. No device or person identifier belongs here.
public struct ClientBuild: Equatable, Sendable {
  public let version: String
  public let build: String

  public init(version: String, build: String) {
    self.version = version
    self.build = build
  }
}
