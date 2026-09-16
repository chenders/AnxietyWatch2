import Foundation
import Testing

@testable import AWDomain

@Test("severity ratings expose named bands without changing their raw value")
func severityBandsAreNamed() {
  #expect(AnxietyRating(rawValue: 1)?.band == .minimal)
  #expect(AnxietyRating(rawValue: 4)?.band == .mild)
  #expect(AnxietyRating(rawValue: 6)?.band == .moderate)
  #expect(AnxietyRating(rawValue: 8)?.band == .high)
  #expect(AnxietyRating(rawValue: 10)?.band == .severe)
}

@Test("song identity is provider independent and treats missing albums explicitly")
func canonicalSongIdentityIsStable() {
  let genius = CanonicalSongIdentity(artist: "  Björk ", title: "Jóga", album: nil)
  let itunes = CanonicalSongIdentity(artist: "BJÖRK", title: "JÓGA", album: "")

  #expect(genius == itunes)
  #expect(genius.songKey == "v1:björk|jóga|")
  #expect(genius.compactLookupKey.count == 64)
}

@Test("provider selection follows capability priority")
func providerSelectionIsCapabilityAware() {
  #expect(
    SongSearchCapabilities(piReachable: true, piGeniusAvailable: true, directGeniusProvisioned: true)
      .preferredProvider == .geniusProxy
  )
  #expect(
    SongSearchCapabilities(piReachable: false, piGeniusAvailable: true, directGeniusProvisioned: true)
      .preferredProvider == .geniusDirect
  )
  #expect(
    SongSearchCapabilities(piReachable: false, piGeniusAvailable: false, directGeniusProvisioned: false)
      .preferredProvider == .itunes
  )
}

@Test("catalog deduplicates by canonical identity and sorts by recent journal use")
func songCatalogDeduplicatesAndSorts() {
  let firstID = UUID(uuidString: "00000000-0000-7000-8000-000000000201")!
  let secondID = UUID(uuidString: "00000000-0000-7000-8000-000000000202")!
  let first = SongSearchResult(
    id: firstID,
    title: "Track",
    artist: "Artist",
    provider: .geniusProxy,
    providerID: "genius-1"
  )
  let duplicate = SongSearchResult(
    id: secondID,
    title: " TRACK ",
    artist: "artist",
    provider: .itunes,
    providerID: "itunes-2"
  )
  var catalog = SongCatalog()
  let songID = catalog.add(first)
  #expect(catalog.add(duplicate) == songID)
  #expect(catalog.songs.count == 1)

  catalog.addOccurrence(
    SongOccurrence(
      id: UUID(uuidString: "00000000-0000-7000-8000-000000000203")!,
      songID: songID,
      journalRecordID: UUID(uuidString: "00000000-0000-7000-8000-000000000204")!,
      subjectAt: "2025-01-15T12:00:00Z"
    )
  )
  #expect(catalog.sortedSongs.first?.occurrenceCount == 1)
  #expect(catalog.sortedSongs.first?.lastUsedAt == "2025-01-15T12:00:00Z")
}

private struct FixedSearchProvider: SongSearchProviderClient {
  let results: [SongSearchResult]
  let failure: SongSearchError?

  func search(query: String) async throws -> [SongSearchResult] {
    if let failure { throw failure }
    return results
  }
}

@Test("a Genius technical failure falls back to iTunes and preserves provider evidence")
func geniusFailureFallsBackToITunes() async throws {
  let itunesResult = SongSearchResult(
    title: "Fallback Track",
    artist: "Fallback Artist",
    provider: .itunes,
    providerID: "itunes-1"
  )
  let coordinator = SongSearchCoordinator(
    geniusProxy: FixedSearchProvider(results: [], failure: .technicalFailure("Pi unavailable")),
    geniusDirect: nil,
    itunes: FixedSearchProvider(results: [itunesResult], failure: nil)
  )
  let results = try await coordinator.search(
    query: "Fallback Track",
    capabilities: SongSearchCapabilities(
      piReachable: true,
      piGeniusAvailable: true,
      directGeniusProvisioned: false
    )
  )
  #expect(results == [itunesResult])
  #expect(results.first?.provider == .itunes)
}

@Test("lyrics preserve pending, unavailable, unknown, and available states")
func lyricsStatesRemainExplicit() {
  let states: [LyricsState] = [
    .pending,
    .unavailable(reason: "Pi has no configured Genius capability"),
    .unknown(reason: "server has not observed a result"),
    .available(text: "A lyric", provider: .geniusProxy, retrievedAt: "2025-01-15T12:00:00Z")
  ]
  #expect(states.count == 4)
  #expect(states[0] != states[1])
  #expect(states[1] != states[2])
}

private actor RecordingSink: DomainCommandSink {
  private(set) var records: [DomainRecordPayload] = []

  func enqueue(record: DomainRecordPayload, capturedAt: Date) {
    records.append(record)
  }

  func allRecords() -> [DomainRecordPayload] { records }
}

private actor RecordingDrainTrigger: DomainDrainTrigger {
  private(set) var requestCount = 0

  func requestImmediateDrain() {
    requestCount += 1
  }

  func count() -> Int { requestCount }
}

@Test("capture reports queued only after enqueue and requests one immediate drain")
func captureQueuesBeforeReceipt() async throws {
  let sink = RecordingSink()
  let trigger = RecordingDrainTrigger()
  let coordinator = JournalCaptureCoordinator(sink: sink, drainTrigger: trigger)
  let id = UUID(uuidString: "00000000-0000-7000-8000-000000000205")!
  let receipt = try await coordinator.capture(
    id: id,
    capturedAt: Date(timeIntervalSince1970: 1_736_942_400),
    anxietyRawValue: 5,
    note: "offline capture",
    expressMode: true
  )

  #expect(receipt == JournalCaptureReceipt(recordID: id))
  let records = await sink.allRecords()
  #expect(records.count == 1)
  #expect(records[0].content != .null)
  #expect(await trigger.count() == 1)
}
