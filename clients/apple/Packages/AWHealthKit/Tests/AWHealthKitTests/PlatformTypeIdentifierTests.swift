import Testing

@testable import AWHealthKit

@Test("empty platform identifiers are rejected rather than guessed")
func emptyIdentifierIsRejected() {
  #expect(PlatformTypeIdentifier(rawValue: "") == nil)
  #expect(
    PlatformTypeIdentifier(rawValue: "HKQuantityTypeIdentifierHeartRate")?.rawValue
      == "HKQuantityTypeIdentifierHeartRate")
}
