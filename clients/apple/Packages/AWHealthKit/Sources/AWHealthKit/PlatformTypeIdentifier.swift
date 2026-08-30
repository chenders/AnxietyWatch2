import AWClientCore
import AWTransport
import Foundation

/// A vendor identifier preserved for registry lookup. This is not a generated registry enum.
public struct PlatformTypeIdentifier: RawRepresentable, Codable, Hashable, Sendable {
  public let rawValue: String

  public init?(rawValue: String) {
    guard !rawValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return nil }
    self.rawValue = rawValue
  }
}
