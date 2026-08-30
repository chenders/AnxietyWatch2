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
      PlaceholderSurface(
        title: "Settings", systemImage: "gear", message: "Enrollment is not configured."
      )
      .tabItem { Label("Settings", systemImage: "gear") }
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
