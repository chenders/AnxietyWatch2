import Testing

@testable import AWTransport

@Test("API compatibility is pinned to the major version")
func majorVersionCompatibility() {
  #expect(APIVersion(major: 1, minor: 0).isCompatible(with: APIVersion(major: 1, minor: 9)))
  #expect(!APIVersion(major: 1, minor: 9).isCompatible(with: APIVersion(major: 2, minor: 0)))
}
