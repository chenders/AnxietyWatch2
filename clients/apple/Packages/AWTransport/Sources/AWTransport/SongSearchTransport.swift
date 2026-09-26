import AWDomain
import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

/// iTunes' public catalog response fields used to create provider-independent song identities.
struct ITunesSearchResponse: Decodable {
  let results: [ITunesTrack]
}

struct ITunesTrack: Decodable {
  let trackName: String?
  let artistName: String?
  let collectionName: String?
  let trackID: Int?
  let artworkURL: String?
  let sourceURL: String?

  enum CodingKeys: String, CodingKey {
    case trackName
    case artistName
    case collectionName
    case trackID = "trackId"
    case artworkURL = "artworkUrl100"
    case sourceURL = "trackViewUrl"
  }
}

/// Read-only iTunes fallback used when the Pi Genius capability is unavailable.
public final class URLSessionITunesSongSearchProvider: SongSearchProviderClient, @unchecked Sendable {
  private let session: URLSession
  private let endpoint: URL
  private let lookupEndpoint: URL

  public convenience init() {
    self.init(
      session: URLSession(configuration: .ephemeral),
      endpoint: URL(string: "https://itunes.apple.com/search")
        ?? URL(fileURLWithPath: "/invalid-itunes-search-endpoint")
    )
  }

  init(session: URLSession, endpoint: URL, lookupEndpoint: URL? = nil) {
    self.session = session
    self.endpoint = endpoint
    self.lookupEndpoint = lookupEndpoint ?? endpoint.deletingLastPathComponent().appendingPathComponent("lookup")
  }

  public func search(query: String) async throws -> [SongSearchResult] {
    let normalizedQuery = query.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !normalizedQuery.isEmpty else {
      throw SongSearchError.noUsableResult("iTunes search query is empty")
    }
    guard normalizedQuery.count <= 200 else {
      throw SongSearchError.noUsableResult(
        "iTunes search query exceeds 200 characters; received \(normalizedQuery.count)"
      )
    }
    guard var components = URLComponents(url: endpoint, resolvingAgainstBaseURL: false) else {
      throw SongSearchError.technicalFailure("iTunes search endpoint could not be formed")
    }
    components.queryItems = [
      URLQueryItem(name: "term", value: normalizedQuery),
      URLQueryItem(name: "entity", value: "song"),
      URLQueryItem(name: "limit", value: "25"),
    ]
    guard let url = components.url else {
      throw SongSearchError.technicalFailure("iTunes search URL could not be formed")
    }

    var request = URLRequest(url: url)
    request.httpMethod = "GET"
    request.setValue("application/json", forHTTPHeaderField: "Accept")
    let data: Data
    let response: URLResponse
    do {
      (data, response) = try await session.data(for: request)
    } catch {
      throw SongSearchError.technicalFailure("iTunes search request failed")
    }
    guard data.count <= 512 * 1024 else {
      throw SongSearchError.technicalFailure("iTunes search response exceeded 512 KiB")
    }
    guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
      let status = (response as? HTTPURLResponse)?.statusCode ?? 0
      throw SongSearchError.technicalFailure("iTunes search returned HTTP status \(status)")
    }
    do {
      let decoded = try JSONDecoder().decode(ITunesSearchResponse.self, from: data)
      return decoded.results.compactMap { track in
        guard let title = nonEmpty(track.trackName), let artist = nonEmpty(track.artistName) else {
          return nil
        }
        return SongSearchResult(
          title: title,
          artist: artist,
          album: nonEmpty(track.collectionName),
          provider: .itunes,
          providerID: track.trackID.map(String.init),
          artworkURL: Self.verifiedArtworkURL(track.artworkURL),
          sourceURL: track.sourceURL
        )
      }
    } catch {
      throw SongSearchError.technicalFailure("iTunes search response was invalid")
    }
  }

  /// Look up artwork by the exact iTunes track ID rather than trusting a broad search match.
  public func artworkURL(for result: SongSearchResult) async throws -> String? {
    guard result.provider == .itunes,
      let providerID = result.providerID,
      !providerID.isEmpty,
      providerID.allSatisfy(\.isNumber),
      var components = URLComponents(url: lookupEndpoint, resolvingAgainstBaseURL: false)
    else { return nil }
    components.queryItems = [
      URLQueryItem(name: "id", value: providerID),
      URLQueryItem(name: "entity", value: "song"),
      URLQueryItem(name: "limit", value: "1"),
    ]
    guard let url = components.url else {
      throw SongSearchError.technicalFailure("iTunes artwork lookup URL could not be formed")
    }
    var request = URLRequest(url: url)
    request.httpMethod = "GET"
    request.setValue("application/json", forHTTPHeaderField: "Accept")
    let (data, response) = try await session.data(for: request)
    guard data.count <= 512 * 1024 else {
      throw SongSearchError.technicalFailure("iTunes artwork lookup response exceeded 512 KiB")
    }
    guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
      let status = (response as? HTTPURLResponse)?.statusCode ?? 0
      throw SongSearchError.technicalFailure("iTunes artwork lookup returned HTTP status \(status)")
    }
    do {
      let decoded = try JSONDecoder().decode(ITunesSearchResponse.self, from: data)
      guard let track = decoded.results.first(where: { $0.trackID.map(String.init) == providerID }) else {
        return nil
      }
      return Self.verifiedArtworkURL(track.artworkURL)
    } catch let error as SongSearchError {
      throw error
    } catch {
      throw SongSearchError.technicalFailure("iTunes artwork lookup response was invalid")
    }
  }

  private static func verifiedArtworkURL(_ value: String?) -> String? {
    guard let value else { return nil }
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    guard let url = URL(string: trimmed),
      url.scheme?.lowercased() == "https",
      url.host != nil,
      url.user == nil,
      url.password == nil
    else { return nil }
    return url.absoluteString
  }

  private func nonEmpty(_ value: String?) -> String? {
    guard let value else { return nil }
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmed.isEmpty ? nil : trimmed
  }
}
