import AWDomain
import XCTest

final class SkeletonTests: XCTestCase {
  func testJournalRatingRejectsOutOfRangeValues() {
    XCTAssertNil(AnxietyRating(rawValue: 0))
    XCTAssertNil(AnxietyRating(rawValue: 11))
    XCTAssertEqual(AnxietyRating(rawValue: 10)?.rawValue, 10)
  }
}
