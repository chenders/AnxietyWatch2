/// Generation-one's retained 1...10 anxiety scale. Invalid values are rejected, never clamped.
public struct AnxietyRating: RawRepresentable, Codable, Comparable, Sendable {
  public let rawValue: UInt8

  public init?(rawValue: UInt8) {
    guard (1...10).contains(rawValue) else { return nil }
    self.rawValue = rawValue
  }

  public static func < (lhs: AnxietyRating, rhs: AnxietyRating) -> Bool {
    lhs.rawValue < rhs.rawValue
  }
}
