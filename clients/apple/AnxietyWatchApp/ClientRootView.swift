import AWDomain
import AWOutbox
import AWTransport
import SwiftUI

/// Navigation responsibility derives from generation-one `AnxietyWatch/App/ContentView.swift`.
/// Stage 7 keeps subjective records on the phone only as protected outbox/mirror data and sends
/// authoritative history and enrichment work to the Pi.
struct ClientRootView: View {
  @EnvironmentObject private var runtime: AppClientRuntime
  @Environment(\.scenePhase) private var scenePhase

  var body: some View {
    TabView {
      RuntimeStatusSurface()
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
    .task {
      await runtime.observeDrainRequests()
    }
    .onChange(of: scenePhase) { _, phase in
      guard phase == .active else { return }
      Task {
        await runtime.reload()
        await runtime.drain()
      }
    }
  }
}

extension Notification.Name {
  static let domainOutboxNeedsDrain = Notification.Name("AnxietyWatch.domainOutboxNeedsDrain")
}

struct AppImmediateDrainTrigger: DomainDrainTrigger {
  func requestImmediateDrain() async {
    NotificationCenter.default.post(name: .domainOutboxNeedsDrain, object: nil)
  }
}

actor AppDomainCommandSink: DomainCommandSink {
  private let outbox: FileOutboxStore
  private let journalMirror: AppJournalMirrorStore

  init(outbox: FileOutboxStore, journalMirror: AppJournalMirrorStore) {
    self.outbox = outbox
    self.journalMirror = journalMirror
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
    if record.kind == "journal_entry" || record.kind == "tombstone" {
      try await journalMirror.put(record: record, syncState: .queued)
    }
  }
}

private struct JournalListItem: Identifiable, Equatable {
  let command: JournalCommand
  let syncState: DomainSyncState
  let deleted: Bool
  var id: UUID { command.id }
}

private func journalListItem(from entry: AppJournalMirrorEntry) -> JournalListItem? {
  guard entry.record.kind == "journal_entry",
    case .object(let content) = entry.record.content,
    case .string(let subjectAt)? = content["subject_at"],
    case .number(let anxietyValue)? = content["anxiety"],
    let anxietyRawValue = UInt8(exactly: anxietyValue),
    let anxiety = AnxietyRating(rawValue: anxietyRawValue),
    case .array(let tagValues)? = content["tags"],
    case .bool(let expressMode)? = content["express_mode"]
  else { return nil }
  let tags = tagValues.compactMap { value -> String? in
    guard case .string(let tag) = value else { return nil }
    return tag
  }
  guard tags.count == tagValues.count else { return nil }
  let note: String?
  switch content["note"] {
  case .string(let value): note = value
  case .null, nil: note = nil
  default: return nil
  }
  let songID: UUID?
  switch content["song_id"] {
  case .string(let value): songID = UUID(uuidString: value)
  case .null, nil: songID = nil
  default: return nil
  }
  guard let command = try? JournalCommand(
    id: entry.record.id,
    subjectAt: subjectAt,
    recordedAt: entry.record.recordedAt,
    anxiety: anxiety,
    note: note,
    tags: tags,
    songID: songID,
    expressMode: expressMode
  ) else { return nil }
  return JournalListItem(command: command, syncState: entry.syncState, deleted: entry.deleted)
}

private struct RuntimeStatusSurface: View {
  @EnvironmentObject private var runtime: AppClientRuntime
  @State private var isSyncing = false

  var body: some View {
    NavigationStack {
      Form {
        Section("Pi connection") {
          Label(
            runtime.isEnrolled ? "Pi enrolled" : "Pi not enrolled",
            systemImage: runtime.isEnrolled ? "checkmark.shield" : "heart.text.square"
          )
          .accessibilityLabel(runtime.isEnrolled ? "Pi enrolled" : "Pi not enrolled")
          Text(runtime.syncStatus)
            .font(.footnote)
            .foregroundStyle(.secondary)
            .accessibilityLabel("Sync status: \(runtime.syncStatus)")
        }
        Section("Local delivery") {
          LabeledContent("Outbox items", value: "\(runtime.outboxItemCount)")
          Button {
            Task {
              isSyncing = true
              await runtime.reload()
              await runtime.drain()
              await runtime.refreshOutboxState()
              isSyncing = false
            }
          } label: {
            if isSyncing {
              ProgressView("Syncing…")
            } else {
              Label("Sync Now", systemImage: "arrow.triangle.2.circlepath")
            }
          }
          .disabled(isSyncing || !runtime.isEnrolled)
        }
        Section("Journal privacy") {
          Toggle(
            "Include local journal mirror in backups",
            isOn: Binding(
              get: { runtime.journalBackupIncluded },
              set: { included in
                Task { await runtime.setJournalBackupIncluded(included) }
              }
            )
          )
          Text("The temporary outbox is always excluded. The local journal mirror remains protected and is excluded until you enable this setting.")
            .font(.footnote)
            .foregroundStyle(.secondary)
        }
      }
      .navigationTitle("Today")
    }
  }
}

private struct JournalSurface: View {
  @State private var showingCapture = false
  @State private var selectedSegment = 0
  @EnvironmentObject private var runtime: AppClientRuntime
  @State private var entries: [JournalListItem] = []
  @State private var catalog = SongCatalog()
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
          if runtime.journalBackupChoiceRequired {
            backupChoicePrompt
          } else {
            journalList
          }
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
            .disabled(runtime.journalBackupChoiceRequired)
          }
        }
      }
      .sheet(isPresented: $showingCapture) {
        JournalCaptureSurface(coordinator: coordinator) { command, song in
          entries.insert(JournalListItem(command: command, syncState: .queued, deleted: false), at: 0)
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
          try await reloadMirror()
          coordinator = try runtime.makeJournalCaptureCoordinator()
        } catch {
          setupError = "Journal storage unavailable: \(error.localizedDescription)"
        }
      }
      .onChange(of: runtime.syncStatus) { _, _ in
        Task {
          try? await reloadMirror()
        }
      }
      .alert("Journal backup setting required", isPresented: backupChoiceBinding) {
        Button("Keep mirror out of backups") {
          Task { await runtime.setJournalBackupIncluded(false) }
        }
        Button("Include mirror in backups") {
          Task { await runtime.setJournalBackupIncluded(true) }
        }
      } message: {
        Text("Choose whether the protected local journal mirror may be included in device backups before using Journal.")
      }
      .alert("Journal unavailable", isPresented: setupErrorBinding) {
        Button("OK", role: .cancel) { setupError = nil }
      } message: {
        Text(setupError ?? "Journal storage is unavailable.")
      }
    }
  }

  private func reloadMirror() async throws {
    let mirroredEntries = try await runtime.journalMirrorEntries()
    entries = mirroredEntries.compactMap(journalListItem(from:))
  }

  private var backupChoiceBinding: Binding<Bool> {
    Binding(
      get: { runtime.journalBackupChoiceRequired },
      set: { _ in }
    )
  }

  private var setupErrorBinding: Binding<Bool> {
    Binding(
      get: { setupError != nil },
      set: { if !$0 { setupError = nil } }
    )
  }

  private var backupChoicePrompt: some View {
    ContentUnavailableView(
      "Choose Journal Backup Setting",
      systemImage: "externaldrive.badge.icloud",
      description: Text("Journal capture is locked until you choose whether the protected local mirror may be included in device backups.")
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
  @EnvironmentObject private var runtime: AppClientRuntime
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
#if os(iOS)
      .navigationBarTitleDisplayMode(.inline)
#endif
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
        SongPickerSurface(searchCoordinator: runtime.songSearchCoordinator) { result in
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
      let songCommand: SongCommand?
      if let selectedSong, !expressMode {
        songCommand = try SongCommand(
          id: selectedSong.id,
          subjectAt: command.subjectAt,
          recordedAt: command.recordedAt,
          title: selectedSong.title,
          artist: selectedSong.artist,
          album: selectedSong.album,
          provider: selectedSong.provider.rawValue,
          providerID: selectedSong.providerID
        )
      } else {
        songCommand = nil
      }
      _ = try await coordinator.capture(
        id: command.id,
        capturedAt: capturedAt,
        anxietyRawValue: selectedRating,
        note: command.note,
        tags: command.tags,
        songID: command.songID,
        song: songCommand,
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
  let searchCoordinator: SongSearchCoordinator
  let onSelect: (SongSearchResult) -> Void
  @State private var title = ""
  @State private var results: [SongSearchResult] = []
  @State private var isSearching = false
  @State private var searchError: String?
  @State private var artist = ""
  @State private var album = ""

  var body: some View {
    NavigationStack {
      Form {
        Section("Search the catalog") {
          TextField("Title or artist", text: $title)
            .onSubmit { Task { await search() } }
          Button {
            Task { await search() }
          } label: {
            if isSearching {
              ProgressView("Searching…")
            } else {
              Label("Search iTunes", systemImage: "magnifyingglass")
            }
          }
          .disabled(title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || isSearching)
          if let searchError {
            Text(searchError)
              .font(.footnote)
              .foregroundStyle(.secondary)
          }
        }
        if !results.isEmpty {
          Section("Results") {
            ForEach(results) { result in
              Button {
                onSelect(result)
              } label: {
                VStack(alignment: .leading, spacing: 3) {
                  Text(result.title)
                  Text("\(result.artist) · \(result.album ?? "Album not provided")")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
              }
              .accessibilityLabel("Select \(result.title) by \(result.artist)")
            }
          }
        }
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
#if os(iOS)
      .navigationBarTitleDisplayMode(.inline)
#endif
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") { dismiss() }
        }
      }
    }
  }

  private func search() async {
    let query = title.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !query.isEmpty, !isSearching else { return }
    isSearching = true
    searchError = nil
    defer { isSearching = false }
    do {
      results = try await searchCoordinator.search(
        query: query,
        capabilities: SongSearchCapabilities(
          piReachable: false,
          piGeniusAvailable: false,
          directGeniusProvisioned: false
        )
      )
    } catch {
      results = []
      searchError = "Song search unavailable: \(error)"
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
        Text(item.deleted ? "Deleted" : item.command.anxiety.band.rawValue)
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
      Text(item.deleted ? "Deleted" : item.syncState.rawValue)
        .font(.caption2)
        .foregroundStyle(.secondary)
        .accessibilityLabel(item.deleted ? "Journal entry deleted" : "Sync state \(item.syncState.rawValue)")
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
        LabeledContent("Sync", value: item.deleted ? "Deleted" : item.syncState.rawValue)
        if item.deleted {
          Text("This entry is retained as an immutable tombstone.")
            .foregroundStyle(.secondary)
        }
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
  @EnvironmentObject private var runtime: AppClientRuntime
  @State private var artifactText = ""
  @State private var deviceLabel = "iPhone"
  @State private var accepted = false
  @State private var isEnrolling = false
  @State private var statusMessage: String?
  @State private var showingRecovery = false

  var body: some View {
    NavigationStack {
      Form {
        Section("Pi connection") {
          Label(
            runtime.isEnrolled ? "Enrolled on this device" : "Not enrolled",
            systemImage: runtime.isEnrolled ? "checkmark.shield" : "lock.shield"
          )
          .accessibilityLabel(runtime.isEnrolled ? "Pi connection enrolled" : "Pi connection not enrolled")
          if let preview = artifactPreview {
            LabeledContent("Server", value: preview.serverDisplayLabel)
            LabeledContent("Origin", value: preview.origin.normalizedString)
            LabeledContent("Expires", value: preview.expiresAt.formatted())
          } else if !runtime.isEnrolled {
            Text("Paste the one-time enrollment artifact created by the private admin console.")
              .font(.footnote)
          }
        }

        if !runtime.isEnrolled {
          Section("One-time enrollment artifact") {
            TextEditor(text: $artifactText)
              .frame(minHeight: 120)
              .font(.footnote.monospaced())
              .accessibilityLabel("Enrollment artifact JSON")
            TextField("Device label", text: $deviceLabel)
              .textContentType(.name)
          }
          Section {
            Toggle("I confirm this server identity and origin", isOn: $accepted)
            Button {
              Task { await enroll() }
            } label: {
              if isEnrolling {
                ProgressView("Enrolling…")
              } else {
                Label("Enroll this iPhone", systemImage: "arrow.right.circle")
              }
            }
            .disabled(!accepted || artifactPreview == nil || isEnrolling)
          } header: {
            Text("Confirmation")
          } footer: {
            Text("The artifact is single-use. The private signing key remains in this device's Secure Enclave.")
          }
        }

        if let statusMessage {
          Section("Enrollment status") {
            Text(statusMessage)
              .foregroundStyle(runtime.isEnrolled ? .secondary : .primary)
              .accessibilityLabel("Enrollment status: \(statusMessage)")
          }
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

  private var artifactPreview: EnrollmentArtifact? {
    guard let data = artifactText.data(using: .utf8) else { return nil }
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .iso8601
    return try? decoder.decode(EnrollmentArtifact.self, from: data)
  }

  private func enroll() async {
    guard let artifact = artifactPreview else {
      statusMessage = "Enrollment blocked: artifact JSON is invalid or incomplete."
      return
    }
    isEnrolling = true
    defer { isEnrolling = false }
    do {
      let confirmation = try EnrollmentConfirmation(
        artifact: artifact,
        confirmedOrigin: artifact.origin,
        confirmedServerInstanceID: artifact.serverInstanceID,
        accepted: accepted,
        now: Date()
      )
      let keyStore = SecureEnclaveKeyStore()
      let credentialStore = KeychainCredentialStore()
      let transport = URLSessionEnrollmentTransport(origin: artifact.origin)
      let coordinator = EnrollmentCoordinator(
        keyStore: keyStore,
        credentialStore: credentialStore,
        transport: transport
      )
      _ = try await coordinator.enroll(
        confirmation: confirmation,
        deviceLabel: deviceLabel,
        appVersion: Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "development"
      )
      await runtime.reload()
      statusMessage = "Enrollment succeeded; this iPhone can now authenticate queued writes."
      artifactText = ""
    } catch {
      statusMessage = "Enrollment failed: \(error)"
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
