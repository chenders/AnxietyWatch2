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

@Test("iTunes artwork lookup uses the exact track ID and validates HTTPS artwork")
func itunesTransportLooksUpExactArtwork() async throws {
  let lookupKey = "itunes.example.test/lookup"
  URLProtocolStub.responses[lookupKey] = Data(
    """
    {
      "resultCount": 1,
      "results": [
        {
          "trackName": "Blue Monday",
          "artistName": "New Order",
          "trackId": 123,
          "artworkUrl100": "https://example.com/blue.jpg"
        }
      ]
    }
    """.data(using: .utf8)!
  )
  URLProtocolStub.lastURLs[lookupKey] = nil
  defer {
    URLProtocolStub.responses[lookupKey] = nil
    URLProtocolStub.lastURLs[lookupKey] = nil
  }
  let configuration = URLSessionConfiguration.ephemeral
  configuration.protocolClasses = [URLProtocolStub.self]
  let provider = URLSessionITunesSongSearchProvider(
    session: URLSession(configuration: configuration),
    endpoint: URL(string: "https://itunes.example.test/search")!,
    lookupEndpoint: URL(string: "https://itunes.example.test/lookup")!
  )
  let result = SongSearchResult(
    title: "Blue Monday",
    artist: "New Order",
    provider: .itunes,
    providerID: "123"
  )

  let artwork = try await provider.artworkURL(for: result)

  #expect(artwork == "https://example.com/blue.jpg")
  #expect(URLProtocolStub.lastURLs[lookupKey]?.path == "/lookup")
  #expect(URLProtocolStub.lastURLs[lookupKey]?.query?.contains("id=123") == true)
}

@Test("iTunes transport rejects non-HTTPS artwork")
func itunesTransportRejectsInsecureArtwork() async throws {
  let searchKey = "insecure.example.test/search"
  URLProtocolStub.responses[searchKey] = Data(
    """
    { "resultCount": 1, "results": [{
      "trackName": "Track", "artistName": "Artist", "trackId": 9,
      "artworkUrl100": "http://example.com/insecure.jpg"
    }] }
    """.data(using: .utf8)!
  )
  defer { URLProtocolStub.responses[searchKey] = nil }
  let configuration = URLSessionConfiguration.ephemeral
  configuration.protocolClasses = [URLProtocolStub.self]
  let provider = URLSessionITunesSongSearchProvider(
    session: URLSession(configuration: configuration),
    endpoint: URL(string: "https://insecure.example.test/search")!
  )

  let results = try await provider.search(query: "track")

  #expect(results.first?.artworkURL == nil)
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
  nonisolated(unsafe) static var responses: [String: Data] = [:]
  nonisolated(unsafe) static var lastURLs: [String: URL] = [:]

  override class func canInit(with request: URLRequest) -> Bool { true }

  override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

  override func startLoading() {
    guard let url = request.url else {
      client?.urlProtocol(self, didFailWithError: URLError(.badURL))
      return
    }
    let key = "\(url.host ?? "")\(url.path)"
    Self.lastURLs[key] = url
    guard let data = Self.responses[key] ?? Self.responseData else {
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
