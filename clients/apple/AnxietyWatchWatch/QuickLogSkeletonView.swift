import AWDomain
import SwiftUI

/// The 1...10 grid derives from generation-one `AnxietyWatch Watch App/QuickLogView.swift`.
/// Buttons stay disabled until T-I16 provides a durable phone relay; this skeleton never claims an
/// ephemeral tap was saved.
struct QuickLogSkeletonView: View {
  private let ratings = (1...10).compactMap { AnxietyRating(rawValue: UInt8($0)) }

  var body: some View {
    ScrollView {
      VStack(spacing: 8) {
        Text("Quick Log")
          .font(.headline)
        LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())]) {
          ForEach(ratings, id: \.rawValue) { rating in
            Button("\(rating.rawValue)") {}
              .frame(minWidth: 44, minHeight: 44)
              .disabled(true)
              .accessibilityLabel("Anxiety \(rating.rawValue) of 10")
          }
        }
        Text("Unavailable: durable iPhone relay is not implemented yet.")
          .font(.footnote)
          .foregroundStyle(.secondary)
      }
    }
  }
}
