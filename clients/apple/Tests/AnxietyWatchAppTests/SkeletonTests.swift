import AWDomain
import AWTransport
import XCTest

final class SkeletonTests: XCTestCase {
  func testJournalRatingRejectsOutOfRangeValues() {
    XCTAssertNil(AnxietyRating(rawValue: 0))
    XCTAssertNil(AnxietyRating(rawValue: 11))
    XCTAssertEqual(AnxietyRating(rawValue: 10)?.rawValue, 10)
  }
}

final class EnrollmentUITests: XCTestCase {
  func testProductionEnrollmentRejectsCleartextOrigin() {
    XCTAssertThrowsError(try ClientOrigin(string: "http://client.example.com"))
  }
}
