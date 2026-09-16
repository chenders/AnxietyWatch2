import AWTransport
import SwiftUI

/// Navigation responsibility derives from generation-one `AnxietyWatch/App/ContentView.swift`.
/// Stage 7 deliberately replaces every data-producing destination with a Pi-backed surface.
struct ClientRootView: View {
  var body: some View {
    TabView {
      PlaceholderSurface(
        title: "Today", systemImage: "heart.text.square",
        message: "Pi status is unknown until this device is enrolled."
      )
      .tabItem { Label("Today", systemImage: "heart.text.square") }
      PlaceholderSurface(
        title: "Journal", systemImage: "book",
        message: "Journal capture arrives in a later Stage 7 task."
      )
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
  }
}

private struct EnrollmentSurface: View {
  @State private var showingRecovery = false

  var body: some View {
    NavigationStack {
      Form {
        Section("Pi connection") {
          Label("Not enrolled", systemImage: "lock.shield")
          Text("Scan a one-time enrollment artifact from the private admin console to connect this iPhone.")
            .font(.footnote)
            .foregroundStyle(.secondary)
        }
        Section {
          Button("Start recovery") { showingRecovery = true }
        } footer: {
          Text("Recovery creates a new device key and does not restore unacknowledged entries from a deleted phone.")
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
