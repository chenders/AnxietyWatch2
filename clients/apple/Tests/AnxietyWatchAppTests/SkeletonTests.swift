import AWDomain
import AWTransport
import XCTest

private func fixturePrivateIPv4Host() -> String {
  [192, 168, 1, 42].map(String.init).joined(separator: ".")
}

final class SkeletonTests: XCTestCase {
  func testJournalRatingRejectsOutOfRangeValues() {
    XCTAssertNil(AnxietyRating(rawValue: 0))
    XCTAssertNil(AnxietyRating(rawValue: 11))
    XCTAssertEqual(AnxietyRating(rawValue: 10)?.rawValue, 10)
  }
}

final class EnrollmentUITests: XCTestCase {
  func testFirstDraftEnrollmentAcceptsPrivateLanHttpOnly() throws {
    XCTAssertNoThrow(try ClientOrigin(string: "http://\(fixturePrivateIPv4Host()):8480"))
    XCTAssertThrowsError(try ClientOrigin(string: "http://client.example.com"))
    XCTAssertThrowsError(try ClientOrigin(string: "http://8.8.8.8:8480"))
  }
}
