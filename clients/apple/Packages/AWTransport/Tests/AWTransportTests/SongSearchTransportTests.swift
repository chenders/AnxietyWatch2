import AWDomain
import Foundation
import Testing

@testable import AWTransport

@Test("iTunes transport preserves usable metadata and skips incomplete tracks")
func itunesTransportMapsResults() async throws {
  URLProtocolStub.responseData = Data(
    """
    {
      "resultCount": 2,
      "results": [
        {
          "trackName": " Blue Monday ",
          "artistName": "New Order",
          "collectionName": "Substance",
          "trackId": 123,
          "artworkUrl100": "https://example.com/art.jpg",
          "trackViewUrl": "https://example.com/track"
        },
        {
          "artistName": "Missing Title"
        }
      ]
    }
    """.data(using: .utf8)!
  )
  defer { URLProtocolStub.responseData = nil }
  let configuration = URLSessionConfiguration.ephemeral
  configuration.protocolClasses = [URLProtocolStub.self]
  let endpoint = URL(string: "https://itunes.example.test/search")!
  let provider = URLSessionITunesSongSearchProvider(
    session: URLSession(configuration: configuration),
    endpoint: endpoint
  )

  let results = try await provider.search(query: "blue monday")

  #expect(results.count == 1)
  #expect(results.first?.title == "Blue Monday")
  #expect(results.first?.artist == "New Order")
  #expect(results.first?.album == "Substance")
  #expect(results.first?.provider == .itunes)
  #expect(results.first?.providerID == "123")
  #expect(results.first?.identity.artist == "new order")
}

@Test("iTunes transport rejects oversized search queries before network access")
func itunesTransportRejectsOversizedQuery() async {
  let provider = URLSessionITunesSongSearchProvider()
  do {
    _ = try await provider.search(query: String(repeating: "x", count: 201))
    Issue.record("oversized iTunes query unexpectedly succeeded")
  } catch let error as SongSearchError {
    #expect(error == .noUsableResult("iTunes search query exceeds 200 characters; received 201"))
  } catch {
    Issue.record("unexpected error: \(error)")
  }
}

private final class URLProtocolStub: URLProtocol, @unchecked Sendable {
  nonisolated(unsafe) static var responseData: Data?

  override class func canInit(with request: URLRequest) -> Bool { true }

  override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

  override func startLoading() {
    guard let url = request.url, let data = Self.responseData else {
      client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
      return
    }
    let response = HTTPURLResponse(
      url: url,
      statusCode: 200,
      httpVersion: nil,
      headerFields: ["Content-Type": "application/json"]
    )!
    client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
    client?.urlProtocol(self, didLoad: data)
    client?.urlProtocolDidFinishLoading(self)
  }

  override func stopLoading() {}
}
