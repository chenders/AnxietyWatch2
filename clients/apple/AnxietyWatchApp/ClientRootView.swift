import AWDomain
import AWOutbox
import AWTransport
import SwiftUI

/// Navigation responsibility derives from generation-one `AnxietyWatch/App/ContentView.swift`.
/// Stage 7 keeps subjective records on the phone only as protected outbox/mirror data and sends
/// authoritative history and enrichment work to the Pi.
struct ClientRootView: View {
  var body: some View {
    TabView {
      PlaceholderSurface(
        title: "Today", systemImage: "heart.text.square",
        message: "Pi status is unknown until this device is enrolled."
      )
      .tabItem { Label("Today", systemImage: "heart.text.square") }
      JournalSurface()
        .tabItem { Label("Journal", systemImage: "book") }
      PlaceholderSurface(
        title: "Medications", systemImage: "pills",
        message: "Medication definitions will come from the Pi."
      )
      .tabItem { Label("Medications", systemImage: "pills") }
      PlaceholderSurface(
        title: "History", systemImage: "chart.xyaxis.line",
        message: "History will render Pi projections and explicit gaps."
      )
      .tabItem { Label("History", systemImage: "chart.xyaxis.line") }
      PlaceholderSurface(
        title: "Health", systemImage: "heart.circle", message: "Health access is not configured."
      )
      .tabItem { Label("Health", systemImage: "heart.circle") }
      EnrollmentSurface()
        .tabItem { Label("Settings", systemImage: "gear") }
    }
    .privacySensitive()
  }
}

private extension Notification.Name {
  static let domainOutboxNeedsDrain = Notification.Name("AnxietyWatch.domainOutboxNeedsDrain")
}

private struct AppImmediateDrainTrigger: DomainDrainTrigger {
  func requestImmediateDrain() async {
    NotificationCenter.default.post(name: .domainOutboxNeedsDrain, object: nil)
  }
}

private actor AppDomainCommandSink: DomainCommandSink {
  private let outbox: FileOutboxStore

  init() throws {
    guard let support = FileManager.default.urls(
      for: .applicationSupportDirectory,
      in: .userDomainMask
    ).first else {
      throw OutboxError.persistenceFailure("application support directory is unavailable")
    }
    let directory = support.appendingPathComponent("AnxietyWatch", isDirectory: true)
    outbox = try FileOutboxStore(directory: directory)
  }

  func enqueue(record: DomainRecordPayload, capturedAt: Date) async throws {
    let payload = try JSONEncoder().encode(DomainEnvelope(record: record))
    let item = try OutboxItem(
      id: record.id,
      recordClass: record.kind,
      schemaVersion: record.schemaVersion,
      payload: payload,
      capturedAt: capturedAt
    )
    try await outbox.enqueue(item)
  }
}

private struct JournalListItem: Identifiable, Equatable {
  let command: JournalCommand
  let syncState: DomainSyncState
  var id: UUID { command.id }
}

private struct JournalSurface: View {
  @State private var showingCapture = false
  @State private var selectedSegment = 0
  @State private var entries: [JournalListItem] = []
  @State private var catalog = SongCatalog()
  @State private var sink: AppDomainCommandSink?
  @State private var coordinator: JournalCaptureCoordinator?
  @State private var setupError: String?

  var body: some View {
    NavigationStack {
      VStack(spacing: 0) {
        Picker("Journal or Songs", selection: $selectedSegment) {
          Text("Journal").tag(0)
          Text("Songs").tag(1)
        }
        .pickerStyle(.segmented)
        .padding(.horizontal)
        .padding(.vertical, 8)
        .accessibilityLabel("Journal or Songs")

        if selectedSegment == 0 {
          journalList
        } else {
          songsList
        }
      }
      .navigationTitle(selectedSegment == 0 ? "Journal" : "Songs")
      .toolbar {
        if selectedSegment == 0 {
          ToolbarItem(placement: .primaryAction) {
            Button {
              showingCapture = true
            } label: {
              Label("New journal entry", systemImage: "plus")
            }
          }
        }
      }
      .sheet(isPresented: $showingCapture) {
        JournalCaptureSurface(coordinator: coordinator) { command, song in
          entries.insert(JournalListItem(command: command, syncState: .queued), at: 0)
          if let song {
            let songID = catalog.add(song)
            catalog.addOccurrence(
              SongOccurrence(
                songID: songID,
                journalRecordID: command.id,
                subjectAt: command.subjectAt
              )
            )
          }
        }
      }
      .task {
        guard coordinator == nil else { return }
        do {
          let createdSink = try AppDomainCommandSink()
          sink = createdSink
          coordinator = JournalCaptureCoordinator(
            sink: createdSink,
            drainTrigger: AppImmediateDrainTrigger()
          )
        } catch {
          setupError = "Journal storage unavailable: \(error.localizedDescription)"
        }
      }
      .alert("Journal unavailable", isPresented: setupErrorBinding) {
        Button("OK", role: .cancel) { setupError = nil }
      } message: {
        Text(setupError ?? "Journal storage is unavailable.")
      }
    }
  }

  private var setupErrorBinding: Binding<Bool> {
    Binding(
      get: { setupError != nil },
      set: { if !$0 { setupError = nil } }
    )
  }

  @ViewBuilder
  private var journalList: some View {
    if entries.isEmpty {
      ContentUnavailableView(
        "No Entries Yet",
        systemImage: "book",
        description: Text("Tap New journal entry to record how you feel.")
      )
    } else {
      List(entries) { item in
        NavigationLink {
          JournalEntryDetailSurface(item: item)
        } label: {
          JournalEntryRow(item: item)
        }
      }
    }
  }

  @ViewBuilder
  private var songsList: some View {
    if catalog.sortedSongs.isEmpty {
      ContentUnavailableView(
        "No Songs Yet",
        systemImage: "music.note",
        description: Text("Add a song while recording a journal entry.")
      )
    } else {
      List(catalog.sortedSongs) { song in
        NavigationLink {
          SongDetailSurface(
            song: song,
            occurrences: catalog.occurrences.filter { $0.songID == song.id }
          )
        } label: {
          SongRowSurface(song: song)
        }
      }
    }
  }
}

private struct JournalCaptureSurface: View {
  @Environment(\.dismiss) private var dismiss
  let coordinator: JournalCaptureCoordinator?
  let onSaved: (JournalCommand, SongSearchResult?) -> Void
  @State private var selectedRating: UInt8?
  @State private var note = ""
  @State private var tagText = ""
  @State private var tags: [String] = []
  @State private var expressMode = true
  @State private var selectedSong: SongSearchResult?
  @State private var showingSongPicker = false
  @State private var isSaving = false
  @State private var saveError: String?

  var body: some View {
    NavigationStack {
      Form {
        Section {
          severityPicker
        } header: {
          Text("How are you feeling?")
        } footer: {
          if let selectedRating, let rating = AnxietyRating(rawValue: selectedRating) {
            Text("\(rating.band.rawValue) anxiety")
          } else {
            Text("Choose 1 through 10")
          }
        }

        Section {
          Toggle("Express Mode", isOn: $expressMode)
            .accessibilityHint("When enabled, tapping a severity saves immediately")
        } footer: {
          Text("Express Mode saves the rating immediately; optional details can be added later.")
        }

        if !expressMode {
          Section("Note") {
            TextField("Optional note", text: $note, axis: .vertical)
              .lineLimit(3...6)
          }
          tagsSection
          songSection
        }
      }
      .navigationTitle("New Journal Entry")
      .navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") { dismiss() }
        }
        if !expressMode {
          ToolbarItem(placement: .confirmationAction) {
            Button("Save") { Task { await save() } }
              .disabled(selectedRating == nil || isSaving)
          }
        }
      }
      .sheet(isPresented: $showingSongPicker) {
        SongPickerSurface { result in
          selectedSong = result
          showingSongPicker = false
        }
      }
      .alert("Entry not saved", isPresented: saveErrorBinding) {
        Button("OK", role: .cancel) { saveError = nil }
      } message: {
        Text(saveError ?? "The entry remains unsaved.")
      }
    }
  }

  private var saveErrorBinding: Binding<Bool> {
    Binding(
      get: { saveError != nil },
      set: { if !$0 { saveError = nil } }
    )
  }

  private var severityPicker: some View {
    LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 5), spacing: 8) {
      ForEach(UInt8(1)...UInt8(10), id: \.self) { level in
        Button {
          selectedRating = level
          if expressMode { Task { await save() } }
        } label: {
          Text("\(level)")
            .font(.title3.bold())
            .frame(minWidth: 44, minHeight: 44)
            .frame(maxWidth: .infinity)
            .background(selectedRating == level ? Color.accentColor : Color.secondary.opacity(0.2), in: .circle)
            .foregroundStyle(selectedRating == level ? .white : .primary)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Anxiety \(level), \(AnxietyRating(rawValue: level)?.band.rawValue ?? "unknown")")
      }
    }
    .padding(.vertical, 4)
  }

  @ViewBuilder
  private var tagsSection: some View {
    Section("Tags") {
      HStack {
        TextField("Add tag", text: $tagText)
          .onSubmit { addTag() }
        Button("Add") { addTag() }
          .disabled(tagText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
      }
      if !tags.isEmpty {
        ForEach(tags, id: \.self) { tag in
          Button {
            tags.removeAll { $0 == tag }
          } label: {
            Label(tag, systemImage: "xmark.circle")
          }
          .accessibilityHint("Removes this tag")
        }
      }
    }
  }

  @ViewBuilder
  private var songSection: some View {
    Section("Song") {
      if let selectedSong {
        VStack(alignment: .leading, spacing: 4) {
          Text(selectedSong.title).font(.headline)
          Text("\(selectedSong.artist) · \(selectedSong.provider.rawValue)")
            .font(.caption)
            .foregroundStyle(.secondary)
          Button("Remove song") { self.selectedSong = nil }
        }
      } else {
        Button {
          showingSongPicker = true
        } label: {
          Label("Search or enter a song", systemImage: "music.note")
        }
      }
    }
  }

  private func addTag() {
    let value = tagText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    guard !value.isEmpty, !tags.contains(value) else {
      tagText = ""
      return
    }
    tags.append(value)
    tagText = ""
  }

  private func save() async {
    guard !isSaving, let selectedRating, let coordinator else {
      if coordinator == nil { saveError = "Journal storage is not ready; no data was discarded." }
      return
    }
    isSaving = true
    defer { isSaving = false }
    let capturedAt = Date()
    do {
      let command = try JournalCommand.capture(
        capturedAt: capturedAt,
        anxietyRawValue: selectedRating,
        note: expressMode ? nil : note.nilIfEmpty,
        tags: expressMode ? [] : tags,
        songID: expressMode ? nil : selectedSong?.id,
        expressMode: expressMode
      )
      _ = try await coordinator.capture(
        id: command.id,
        capturedAt: capturedAt,
        anxietyRawValue: selectedRating,
        note: command.note,
        tags: command.tags,
        songID: command.songID,
        expressMode: command.expressMode
      )
      onSaved(command, expressMode ? nil : selectedSong)
      dismiss()
    } catch {
      saveError = "Capture failed and remains unsaved: \(error.localizedDescription)"
    }
  }
}

private extension String {
  var nilIfEmpty: String? { isEmpty ? nil : self }
}

private struct SongPickerSurface: View {
  @Environment(\.dismiss) private var dismiss
  let onSelect: (SongSearchResult) -> Void
  @State private var title = ""
  @State private var artist = ""
  @State private var album = ""

  var body: some View {
    NavigationStack {
      Form {
        Section("Offline song stub") {
          TextField("Title", text: $title)
          TextField("Artist", text: $artist)
          TextField("Album (optional)", text: $album)
        }
        Section {
          Button("Use this song") {
            onSelect(
              SongSearchResult(
                title: title.trimmingCharacters(in: .whitespacesAndNewlines),
                artist: artist.trimmingCharacters(in: .whitespacesAndNewlines),
                album: album.nilIfEmpty,
                provider: .manual
              )
            )
          }
          .disabled(
            title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              || artist.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          )
        } footer: {
          Text("Live results use the capability-aware Pi/Genius/iTunes search contract when available. This manual stub works offline.")
        }
      }
      .navigationTitle("Song")
      .navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") { dismiss() }
        }
      }
    }
  }
}

private struct JournalEntryRow: View {
  let item: JournalListItem

  var body: some View {
    HStack(spacing: 12) {
      Text("\(item.command.anxiety.rawValue)")
        .font(.headline.bold())
        .foregroundStyle(.white)
        .frame(minWidth: 44, minHeight: 44)
        .background(Color.accentColor, in: .circle)
        .accessibilityLabel("Anxiety \(item.command.anxiety.rawValue), \(item.command.anxiety.band.rawValue)")
      VStack(alignment: .leading, spacing: 3) {
        Text(item.command.subjectAt)
          .font(.subheadline.bold())
        Text(item.command.anxiety.band.rawValue)
          .font(.caption)
          .foregroundStyle(.secondary)
        if let note = item.command.note {
          Text(note)
            .font(.caption)
            .foregroundStyle(.secondary)
            .lineLimit(2)
        }
      }
      Spacer()
      Text(item.syncState.rawValue)
        .font(.caption2)
        .foregroundStyle(.secondary)
        .accessibilityLabel("Sync state \(item.syncState.rawValue)")
    }
  }
}

private struct JournalEntryDetailSurface: View {
  let item: JournalListItem

  var body: some View {
    Form {
      Section("Anxiety") {
        LabeledContent("Rating", value: "\(item.command.anxiety.rawValue) · \(item.command.anxiety.band.rawValue)")
        LabeledContent("Subject time", value: item.command.subjectAt)
        LabeledContent("Recorded time", value: item.command.recordedAt)
      }
      Section("Content") {
        LabeledContent("Sync", value: item.syncState.rawValue)
        if let note = item.command.note { Text(note) }
        if !item.command.tags.isEmpty { LabeledContent("Tags", value: item.command.tags.joined(separator: ", ")) }
      }
    }
    .navigationTitle("Journal Entry")
  }
}

private struct SongRowSurface: View {
  let song: SongRecord

  var body: some View {
    HStack {
      Image(systemName: "music.note")
        .frame(minWidth: 44, minHeight: 44)
        .accessibilityHidden(true)
      VStack(alignment: .leading) {
        Text(song.title).font(.headline)
        Text(song.artist).font(.subheadline).foregroundStyle(.secondary)
      }
      Spacer()
      Text("\(song.occurrenceCount) uses")
        .font(.caption)
        .foregroundStyle(.secondary)
    }
  }
}

private struct SongDetailSurface: View {
  let song: SongRecord
  let occurrences: [SongOccurrence]

  var body: some View {
    Form {
      Section("Metadata") {
        LabeledContent("Title", value: song.title)
        LabeledContent("Artist", value: song.artist)
        LabeledContent("Album", value: song.album ?? "Not provided")
        LabeledContent("Provider", value: song.provider.rawValue)
        LabeledContent("Song key", value: song.identity.songKey)
        LabeledContent("Sync", value: song.syncState.rawValue)
      }
      Section("Lyrics") {
        lyricsView
      }
      Section("Journal occurrences") {
        if occurrences.isEmpty {
          Text("No occurrence history has been observed.")
            .foregroundStyle(.secondary)
        } else {
          ForEach(occurrences) { occurrence in
            LabeledContent(occurrence.source, value: occurrence.subjectAt)
          }
        }
      }
    }
    .navigationTitle(song.title)
  }

  @ViewBuilder
  private var lyricsView: some View {
    switch song.lyrics {
    case .available(let text, let provider, let retrievedAt):
      Text(text)
      Text("\(provider.rawValue), retrieved \(retrievedAt)")
        .font(.caption)
        .foregroundStyle(.secondary)
    case .pending:
      Label("Lyrics pending", systemImage: "clock")
    case .unavailable(let reason):
      Label("Lyrics unavailable: \(reason)", systemImage: "questionmark")
    case .unknown(let reason):
      Label("Lyrics unknown: \(reason)", systemImage: "questionmark")
    }
  }
}

private struct EnrollmentSurface: View {
  @State private var showingRecovery = false

  var body: some View {
    NavigationStack {
      Form {
        Section("Pi connection") {
          Label("Not enrolled", systemImage: "lock.shield")
            .accessibilityLabel("Pi connection not enrolled")
          Text("Scan a one-time enrollment artifact from the private admin console to connect this iPhone.")
            .font(.footnote)
        }
        Section {
          Button("Start recovery") { showingRecovery = true }
        } footer: {
          Text("Recovery creates a new device key and does not discard queued journal entries.")
        }
      }
      .navigationTitle("Settings")
      .sheet(isPresented: $showingRecovery) { RecoveryView() }
    }
  }
}

private struct RecoveryView: View {
  @Environment(\.dismiss) private var dismiss
  @State private var confirmed = false

  var body: some View {
    NavigationStack {
      Form {
        Section("New device enrollment") {
          Text("Create a fresh enrollment artifact in the private admin console, then scan it here.")
          Toggle("I understand that this creates a new device key", isOn: $confirmed)
        }
        Section {
          Button("Continue") { dismiss() }
            .disabled(!confirmed)
        } footer: {
          Text("The previous private key is never copied or recovered. Pending entries remain protected until this device is re-enrolled.")
        }
      }
      .navigationTitle("Recovery")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") { dismiss() }
        }
      }
    }
  }
}

private struct PlaceholderSurface: View {
  let title: String
  let systemImage: String
  let message: String

  var body: some View {
    NavigationStack {
      ContentUnavailableView(title, systemImage: systemImage, description: Text(message))
        .navigationTitle(title)
    }
  }
}
