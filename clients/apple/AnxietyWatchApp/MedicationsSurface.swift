import AWDomain
import AWOutbox
import SwiftUI

/// Medication definitions fetched from the Pi. The phone never creates definitions; it only logs doses.
struct MedicationDefinition: Codable, Equatable, Identifiable, Sendable {
  let id: UUID
  let name: String
  let category: String
  let defaultDoseMg: Double?
  let promptAnxietyOnLog: Bool
  let isActive: Bool

  var displayName: String { name }
}

/// A local dose record shown in the recent doses list.
struct DoseListItem: Identifiable {
  let id: UUID
  let medicationName: String
  let doseDescription: String?
  let subjectAt: String
  let syncState: DomainSyncState

  init(record: DomainRecordPayload, syncState: DomainSyncState) {
    self.id = record.id
    self.syncState = syncState
    if case .object(let content) = record.content {
      self.medicationName = {
        if case .string(let name) = content["medication_name"] { return name }
        return "Unknown"
      }()
      self.doseDescription = {
        if case .string(let desc) = content["dose_description"] { return desc }
        return nil
      }()
      self.subjectAt = {
        if case .string(let time) = content["subject_at"] { return time }
        return ""
      }()
    } else {
      self.medicationName = "Unknown"
      self.doseDescription = nil
      self.subjectAt = ""
    }
  }
}

/// Medications surface: Quick Log for active medications, recent doses list.
/// Medication definitions come from the Pi; the phone only captures dose events.
struct MedicationsSurface: View {
  @EnvironmentObject private var runtime: AppClientRuntime
  @State private var definitions: [MedicationDefinition] = []
  @State private var recentDoses: [DoseListItem] = []
  @State private var isLoading = true
  @State private var loadError: String?
  @State private var showingDosePrompt = false
  @State private var selectedMedication: MedicationDefinition?

  var body: some View {
    NavigationStack {
      Group {
        if isLoading {
          ProgressView("Loading medications…")
        } else if let error = loadError {
          ContentUnavailableView(
            "Medications Unavailable",
            systemImage: "exclamationmark.triangle",
            description: Text(error)
          )
        } else if definitions.isEmpty {
          ContentUnavailableView(
            "No Medications",
            systemImage: "pills",
            description: Text(
              runtime.isEnrolled
                ? "Your Pi does not provide medication definitions yet, so doses cannot be logged from this phone."
                : "Medication definitions will be provided by your Pi. Enroll this device and sync to load them."
            )
          )
        } else {
          medicationsForm
        }
      }
      .navigationTitle("Medications")
      .task {
        await loadMedications()
      }
      .refreshable {
        await loadMedications()
      }
      .sheet(isPresented: $showingDosePrompt) {
        if let medication = selectedMedication {
          DoseAnxietyPromptSheet(
            medication: medication,
            onComplete: { dose, anxiety in
              Task {
                await logDose(medication: medication, description: dose, anxiety: anxiety)
                showingDosePrompt = false
                selectedMedication = nil
              }
            },
            onCancel: {
              showingDosePrompt = false
              selectedMedication = nil
            }
          )
        }
      }
    }
  }

  @ViewBuilder
  private var medicationsForm: some View {
    Form {
      Section("Quick Log") {
        ForEach(definitions.filter(\.isActive)) { medication in
          Button {
            if medication.promptAnxietyOnLog {
              selectedMedication = medication
              showingDosePrompt = true
            } else {
              Task { await quickLogDose(medication: medication) }
            }
          } label: {
            HStack {
              VStack(alignment: .leading) {
                Text(medication.displayName)
                  .font(.headline)
                if let dose = medication.defaultDoseMg {
                  Text("\(dose.formatted()) mg")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
              }
              Spacer()
              Image(systemName: "plus.circle.fill")
                .font(.title2)
                .foregroundStyle(.blue)
            }
          }
          .buttonStyle(.plain)
        }
      }

      if !recentDoses.isEmpty {
        Section("Recent Doses") {
          ForEach(recentDoses.prefix(10)) { dose in
            HStack {
              VStack(alignment: .leading) {
                Text(dose.medicationName)
                  .font(.headline)
                if let description = dose.doseDescription {
                  Text(description)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                Text(dose.subjectAt)
                  .font(.caption2)
                  .foregroundStyle(.tertiary)
              }
              Spacer()
              syncStateIndicator(dose.syncState)
            }
          }
        }
      }

      if definitions.contains(where: { !$0.isActive }) {
        Section("Inactive Medications") {
          ForEach(definitions.filter { !$0.isActive }) { medication in
            Text(medication.displayName)
              .foregroundStyle(.secondary)
          }
        }
      }
    }
  }

  @ViewBuilder
  private func syncStateIndicator(_ state: DomainSyncState) -> some View {
    switch state {
    case .queued:
      Image(systemName: "clock")
        .foregroundStyle(.orange)
        .accessibilityLabel("Queued for sync")
    case .acknowledged:
      Image(systemName: "checkmark.circle")
        .foregroundStyle(.green)
        .accessibilityLabel("Synced to Pi")
    case .conflict:
      Image(systemName: "exclamationmark.triangle")
        .foregroundStyle(.red)
        .accessibilityLabel("Sync conflict")
    case .unavailable:
      Image(systemName: "xmark.circle")
        .foregroundStyle(.secondary)
        .accessibilityLabel("Unavailable")
    case .unknown:
      Image(systemName: "questionmark.circle")
        .foregroundStyle(.secondary)
        .accessibilityLabel("Unknown state")
    }
  }

  private func loadMedications() async {
    isLoading = true
    defer { isLoading = false }

    // The Pi does not serve medication definitions yet, and the phone never
    // creates them. Until it does, there are none: an invented list would be
    // rendered as if it were the user's real regimen.
    if runtime.isEnrolled {
      // TODO: Fetch definitions from the Pi once an endpoint exists.
      definitions = []

      // Load recent doses from outbox/mirror
      do {
        let entries = try await runtime.journalMirrorEntries()
        recentDoses = entries
          .filter { $0.record.kind == "medication_dose" && !$0.deleted }
          .map { DoseListItem(record: $0.record, syncState: $0.syncState) }
          .sorted { $0.subjectAt > $1.subjectAt }
      } catch {
        loadError = error.localizedDescription
      }
    } else {
      definitions = []
      loadError = "Enroll this device with a Pi to load medication definitions."
    }
  }

  private func quickLogDose(medication: MedicationDefinition) async {
    await logDose(medication: medication, description: nil, anxiety: nil)
  }

  private func logDose(medication: MedicationDefinition, description: String?, anxiety: UInt8?) async {
    // TODO: Use MedicationDoseCommand and enqueue via outbox
    // For now, trigger a sync refresh
    await loadMedications()
  }
}

/// Sheet for logging a dose with optional anxiety rating.
private struct DoseAnxietyPromptSheet: View {
  let medication: MedicationDefinition
  let onComplete: (String?, UInt8?) -> Void
  let onCancel: () -> Void

  @State private var doseDescription: String = ""
  @State private var anxietyRating: Double = 5
  @State private var includeAnxiety = true

  var body: some View {
    NavigationStack {
      Form {
        Section("Dose") {
          Text(medication.displayName)
            .font(.headline)
          if let defaultDose = medication.defaultDoseMg {
            TextField("Dose (default \(defaultDose.formatted()) mg)", text: $doseDescription)
          } else {
            TextField("Dose description", text: $doseDescription)
          }
        }

        Section("Current Anxiety") {
          Toggle("Include anxiety rating", isOn: $includeAnxiety)
          if includeAnxiety {
            VStack {
              Text("\(Int(anxietyRating))")
                .font(.system(.largeTitle, design: .default, weight: .bold))
                .foregroundStyle(anxietyColor)
              Slider(value: $anxietyRating, in: 1...10, step: 1)
              Text(anxietyLabel)
                .font(.caption)
                .foregroundStyle(.secondary)
            }
            .padding(.vertical)
          }
        }
      }
      .navigationTitle("Log Dose")
      .navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel", action: onCancel)
        }
        ToolbarItem(placement: .confirmationAction) {
          Button("Log") {
            let desc = doseDescription.isEmpty ? nil : doseDescription
            let anxiety = includeAnxiety ? UInt8(anxietyRating) : nil
            onComplete(desc, anxiety)
          }
        }
      }
    }
  }

  private var anxietyColor: Color {
    switch Int(anxietyRating) {
    case 1...2: return .green
    case 3...4: return .yellow
    case 5...6: return .orange
    case 7...8: return .red
    default: return Color(red: 0.6, green: 0.0, blue: 0.0)
    }
  }

  private var anxietyLabel: String {
    switch Int(anxietyRating) {
    case 1...2: return "Minimal"
    case 3...4: return "Mild"
    case 5...6: return "Moderate"
    case 7...8: return "High"
    default: return "Severe"
    }
  }
}

#Preview {
  MedicationsSurface()
    .environmentObject(AppClientRuntime())
}
