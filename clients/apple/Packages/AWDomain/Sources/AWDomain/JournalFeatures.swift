import CryptoKit
import Foundation

/// Human-readable bands for the retained 1...10 subjective anxiety scale.
public enum AnxietyBand: String, Codable, Equatable, Sendable {
  case minimal = "Minimal"
  case mild = "Mild"
  case moderate = "Moderate"
  case high = "High"
  case severe = "Severe"

  public init(rating: AnxietyRating) {
    switch rating.rawValue {
    case 1...2: self = .minimal
    case 3...4: self = .mild
    case 5...6: self = .moderate
    case 7...8: self = .high
    default: self = .severe
    }
  }
}

public extension AnxietyRating {
  /// The named presentation band; this does not alter the stored rating.
  var band: AnxietyBand { AnxietyBand(rating: self) }
}

/// A provider-independent identity for a logical song.
public struct CanonicalSongIdentity: Codable, Equatable, Hashable, Sendable {
  public let version: UInt
  public let artist: String
  public let title: String
  public let album: String
  public let songKey: String
  public let compactLookupKey: String

  public init(artist: String, title: String, album: String?) {
    let normalizedArtist = Self.normalize(artist)
    let normalizedTitle = Self.normalize(title)
    let normalizedAlbum = album.map(Self.normalize) ?? ""
    let tuple = [normalizedArtist, normalizedTitle, normalizedAlbum].joined(separator: "|")
    self.version = 1
    self.artist = normalizedArtist
    self.title = normalizedTitle
    self.album = normalizedAlbum
    self.songKey = "v1:\(tuple)"
    self.compactLookupKey = SHA256.hash(data: Data(self.songKey.utf8))
      .map { String(format: "%02x", $0) }
      .joined()
  }

  private static func normalize(_ value: String) -> String {
    value
      .precomposedStringWithCanonicalMapping
      .lowercased()
      .split(whereSeparator: \.isWhitespace)
      .joined(separator: " ")
  }
}

/// Search providers are displayed as evidence, never used as song identity.
public enum SongProvider: String, Codable, Equatable, Sendable {
  case geniusProxy = "genius_proxy"
  case geniusDirect = "genius_direct"
  case itunes
  case manual
  case unknown
}

/// Capability facts used to choose a song-search route without sending journal text.
public struct SongSearchCapabilities: Codable, Equatable, Sendable {
  public let piReachable: Bool
  public let piGeniusAvailable: Bool
  public let directGeniusProvisioned: Bool

  public init(piReachable: Bool, piGeniusAvailable: Bool, directGeniusProvisioned: Bool) {
    self.piReachable = piReachable
    self.piGeniusAvailable = piGeniusAvailable
    self.directGeniusProvisioned = directGeniusProvisioned
  }

  public var preferredProvider: SongProvider {
    if piReachable && piGeniusAvailable { return .geniusProxy }
    if directGeniusProvisioned { return .geniusDirect }
    return .itunes
  }
}

/// A metadata result returned by Genius, iTunes, or an offline manual stub.
public enum SongSearchError: Error, Equatable, Sendable {
  case technicalFailure(String)
  case noUsableResult(String)
}

/// A provider adapter owned by the transport layer; journal text is never passed to it.
public protocol SongSearchProviderClient: Sendable {
  func search(query: String) async throws -> [SongSearchResult]
}

/// Capability-aware search with the required Genius-to-iTunes fallback.
public struct SongSearchCoordinator: Sendable {
  private let geniusProxy: (any SongSearchProviderClient)?
  private let geniusDirect: (any SongSearchProviderClient)?
  private let itunes: any SongSearchProviderClient

  public init(
    geniusProxy: (any SongSearchProviderClient)?,
    geniusDirect: (any SongSearchProviderClient)?,
    itunes: any SongSearchProviderClient
  ) {
    self.geniusProxy = geniusProxy
    self.geniusDirect = geniusDirect
    self.itunes = itunes
  }

  public func search(query: String, capabilities: SongSearchCapabilities) async throws -> [SongSearchResult] {
    guard !query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw SongSearchError.noUsableResult("song search query is empty")
    }
    if capabilities.preferredProvider == .geniusProxy, let geniusProxy {
      if let results = try? await geniusProxy.search(query: query), !results.isEmpty {
        return results
      }
    }
    if capabilities.preferredProvider == .geniusDirect, let geniusDirect {
      if let results = try? await geniusDirect.search(query: query), !results.isEmpty {
        return results
      }
    }
    let results = try await itunes.search(query: query)
    guard !results.isEmpty else {
      throw SongSearchError.noUsableResult("iTunes returned no usable song result")
    }
    return results
  }
}

public struct SongSearchResult: Codable, Equatable, Sendable, Identifiable {
  public let id: UUID
  public let title: String
  public let artist: String
  public let album: String?
  public let provider: SongProvider
  public let providerID: String?
  public let artworkURL: String?
  public let sourceURL: String?
  public let identity: CanonicalSongIdentity

  public init(
    id: UUID = UUID(),
    title: String,
    artist: String,
    album: String? = nil,
    provider: SongProvider,
    providerID: String? = nil,
    artworkURL: String? = nil,
    sourceURL: String? = nil
  ) {
    self.id = id
    self.title = title
    self.artist = artist
    self.album = album
    self.provider = provider
    self.providerID = providerID
    self.artworkURL = artworkURL
    self.sourceURL = sourceURL
    self.identity = CanonicalSongIdentity(artist: artist, title: title, album: album)
  }
}

/// Lyrics never collapse unavailable, pending, unknown, and available into one value.
public enum LyricsState: Codable, Equatable, Sendable {
  case unavailable(reason: String)
  case pending
  case unknown(reason: String)
  case available(text: String, provider: SongProvider, retrievedAt: String)
}

/// Metadata retained in the protected journal mirror and reconciled with Pi projections.
public struct SongRecord: Codable, Equatable, Sendable, Identifiable {
  public let id: UUID
  public let title: String
  public let artist: String
  public let album: String?
  public let provider: SongProvider
  public let providerID: String?
  public let artworkURL: String?
  public let sourceURL: String?
  public let identity: CanonicalSongIdentity
  public let lyrics: LyricsState
  public let lastUsedAt: String?
  public let occurrenceCount: Int
  public let syncState: DomainSyncState

  public init(
    id: UUID,
    title: String,
    artist: String,
    album: String?,
    provider: SongProvider,
    providerID: String?,
    artworkURL: String?,
    sourceURL: String?,
    lyrics: LyricsState = .unknown(reason: "lyrics have not been observed"),
    lastUsedAt: String? = nil,
    occurrenceCount: Int = 0,
    syncState: DomainSyncState = .queued
  ) {
    self.id = id
    self.title = title
    self.artist = artist
    self.album = album
    self.provider = provider
    self.providerID = providerID
    self.artworkURL = artworkURL
    self.sourceURL = sourceURL
    self.identity = CanonicalSongIdentity(artist: artist, title: title, album: album)
    self.lyrics = lyrics
    self.lastUsedAt = lastUsedAt
    self.occurrenceCount = occurrenceCount
    self.syncState = syncState
  }

  public init(result: SongSearchResult, syncState: DomainSyncState = .queued) {
    self.init(
      id: result.id,
      title: result.title,
      artist: result.artist,
      album: result.album,
      provider: result.provider,
      providerID: result.providerID,
      artworkURL: result.artworkURL,
      sourceURL: result.sourceURL,
      syncState: syncState
    )
  }
}

/// One journal-to-song link, carrying the journal subject time rather than a mutable foreign row.
public struct SongOccurrence: Codable, Equatable, Sendable, Identifiable {
  public let id: UUID
  public let songID: UUID
  public let journalRecordID: UUID
  public let subjectAt: String
  public let source: String

  public init(
    id: UUID = UUID(),
    songID: UUID,
    journalRecordID: UUID,
    subjectAt: String,
    source: String = "journal"
  ) {
    self.id = id
    self.songID = songID
    self.journalRecordID = journalRecordID
    self.subjectAt = subjectAt
    self.source = source
  }
}

/// States shown beside locally mirrored records and songs.
public enum DomainSyncState: String, Codable, Equatable, Sendable {
  case queued
  case acknowledged
  case conflict
  case unavailable
  case unknown
}

/// A bounded, in-memory projection helper. Persistence remains the protected mirror/outbox layer.
public struct SongCatalog: Codable, Equatable, Sendable {
  public private(set) var songs: [SongRecord]
  public private(set) var occurrences: [SongOccurrence]

  public init(songs: [SongRecord] = [], occurrences: [SongOccurrence] = []) {
    self.songs = songs
    self.occurrences = occurrences
  }

  /// Adds a result once per canonical identity; provider IDs never deduplicate songs.
  public mutating func add(_ result: SongSearchResult) -> UUID {
    if let existing = songs.first(where: { $0.identity == result.identity }) {
      return existing.id
    }
    let song = SongRecord(result: result)
    songs.append(song)
    return song.id
  }

  public mutating func addOccurrence(_ occurrence: SongOccurrence) {
    guard !occurrences.contains(where: { $0.id == occurrence.id }) else { return }
    guard songs.contains(where: { $0.id == occurrence.songID }) else { return }
    occurrences.append(occurrence)
    guard let index = songs.firstIndex(where: { $0.id == occurrence.songID }) else { return }
    let song = songs[index]
    songs[index] = SongRecord(
      id: song.id,
      title: song.title,
      artist: song.artist,
      album: song.album,
      provider: song.provider,
      providerID: song.providerID,
      artworkURL: song.artworkURL,
      sourceURL: song.sourceURL,
      lyrics: song.lyrics,
      lastUsedAt: occurrence.subjectAt,
      occurrenceCount: song.occurrenceCount + 1,
      syncState: song.syncState
    )
  }

  /// Most recent activity first, then the displayed title for deterministic ties.
  public var sortedSongs: [SongRecord] {
    songs.sorted { left, right in
      switch (left.lastUsedAt, right.lastUsedAt) {
      case let (leftTime?, rightTime?) where leftTime != rightTime:
        return leftTime > rightTime
      case (nil, .some):
        return false
      case (.some, nil):
        return true
      default:
        return left.title.localizedCaseInsensitiveCompare(right.title) == .orderedAscending
      }
    }
  }
}

/// A capture result is returned only after the caller's durable enqueue succeeds.
public struct JournalCaptureReceipt: Equatable, Sendable {
  public let recordID: UUID
  public let state: DomainSyncState

  public init(recordID: UUID, state: DomainSyncState = .queued) {
    self.recordID = recordID
    self.state = state
  }
}

/// The app adapter supplies the durable outbox transaction; this package never owns persistence.
public protocol DomainCommandSink: Sendable {
  func enqueue(record: DomainRecordPayload, capturedAt: Date) async throws
}

/// A foreground/background adapter can start one existing drain after enqueue commits.
public protocol DomainDrainTrigger: Sendable {
  func requestImmediateDrain() async
}

/// Builds and queues journal input before the UI can report success.
public actor JournalCaptureCoordinator {
  private let sink: any DomainCommandSink
  private let drainTrigger: (any DomainDrainTrigger)?

  public init(sink: any DomainCommandSink, drainTrigger: (any DomainDrainTrigger)? = nil) {
    self.sink = sink
    self.drainTrigger = drainTrigger
  }

  public func capture(
    id: UUID = UUID(),
    capturedAt: Date,
    anxietyRawValue: UInt8,
    note: String? = nil,
    tags: [String] = [],
    songID: UUID? = nil,
    song: SongCommand? = nil,
    expressMode: Bool = false
  ) async throws -> JournalCaptureReceipt {
    if let song, songID != song.id {
      throw DomainCommandError.invalidSongReference
    }
    let command = try JournalCommand.capture(
      id: id,
      capturedAt: capturedAt,
      anxietyRawValue: anxietyRawValue,
      note: note,
      tags: tags,
      songID: songID,
      expressMode: expressMode
    )
    if let song {
      try await sink.enqueue(record: try song.record(), capturedAt: capturedAt)
    }
    let record = try command.record()
    try await sink.enqueue(record: record, capturedAt: capturedAt)
    await drainTrigger?.requestImmediateDrain()
    return JournalCaptureReceipt(recordID: record.id)
  }
}
