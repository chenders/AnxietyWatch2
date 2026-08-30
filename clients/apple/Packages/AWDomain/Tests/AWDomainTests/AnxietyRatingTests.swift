import Testing

@testable import AWDomain

@Test("ratings outside 1 through 10 are rejected, not clamped")
func invalidRatingsAreRejected() {
  #expect(AnxietyRating(rawValue: 0) == nil)
  #expect(AnxietyRating(rawValue: 11) == nil)
  #expect(AnxietyRating(rawValue: 10)?.rawValue == 10)
}
