import AWDomain
import AWTransport
import Charts
import SwiftUI

/// A projection point for charting with explicit nil for gaps.
struct ChartableProjection: Identifiable {
  let id: String
  let timestamp: Date
  let value: Double?
  let metric: String

  init(point: ProjectionPoint, metric: String) {
    self.id = "\(metric)-\(point.timestamp)"
    self.metric = metric
    // Parse ISO8601 timestamp
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    self.timestamp = formatter.date(from: point.timestamp) ?? {
      formatter.formatOptions = [.withInternetDateTime]
      return formatter.date(from: point.timestamp) ?? Date()
    }()
    self.value = point.value
  }
}

/// History surface: renders Pi projections as time-series charts with explicit gaps.
struct HistorySurface: View {
  @EnvironmentObject private var runtime: AppClientRuntime
  @State private var selectedMetric = "hr"
  @State private var dateRange: DateRange = .week
  @State private var projectionData: [ChartableProjection] = []
  @State private var isLoading = false
  @State private var loadError: String?
  @State private var missingIntervals: [[String]] = []

  private let metrics = [
    ("hr", "Heart Rate", "bpm"),
    ("spo2", "Oxygen Saturation", "%"),
    ("respiratory_rate", "Respiratory Rate", "breaths/min"),
    ("resting_hr", "Resting Heart Rate", "bpm"),
    ("step_count", "Steps", "steps")
  ]

  enum DateRange: String, CaseIterable {
    case day = "Day"
    case week = "Week"
    case month = "Month"

    var interval: TimeInterval {
      switch self {
      case .day: return 86400
      case .week: return 86400 * 7
      case .month: return 86400 * 30
      }
    }
  }

  var body: some View {
    NavigationStack {
      VStack(spacing: 0) {
        // Metric picker
        Picker("Metric", selection: $selectedMetric) {
          ForEach(metrics, id: \.0) { metric in
            Text(metric.1).tag(metric.0)
          }
        }
        .pickerStyle(.segmented)
        .padding(.horizontal)
        .padding(.top)

        // Date range picker
        Picker("Range", selection: $dateRange) {
          ForEach(DateRange.allCases, id: \.self) { range in
            Text(range.rawValue).tag(range)
          }
        }
        .pickerStyle(.segmented)
        .padding(.horizontal)
        .padding(.top, 8)

        // Chart or loading/error state
        Group {
          if !runtime.isEnrolled {
            ContentUnavailableView(
              "Not Enrolled",
              systemImage: "lock.shield",
              description: Text("Enroll this device with a Pi to view projection history.")
            )
          } else if isLoading {
            ProgressView("Loading projections…")
              .frame(maxWidth: .infinity, maxHeight: .infinity)
          } else if let error = loadError {
            ContentUnavailableView(
              "Load Failed",
              systemImage: "exclamationmark.triangle",
              description: Text(error)
            )
          } else if projectionData.isEmpty {
            ContentUnavailableView(
              "No Data",
              systemImage: "chart.xyaxis.line",
              description: Text("No \(selectedMetricLabel) measurements in the selected range.")
            )
          } else {
            projectionChart
          }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)

        // Missing intervals indicator
        if !missingIntervals.isEmpty {
          HStack {
            Image(systemName: "exclamationmark.triangle")
              .foregroundStyle(.orange)
            Text("\(missingIntervals.count) gap(s) in coverage")
              .font(.caption)
              .foregroundStyle(.secondary)
          }
          .padding(.horizontal)
          .padding(.bottom, 8)
        }
      }
      .navigationTitle("History")
      .toolbar {
        ToolbarItem(placement: .topBarTrailing) {
          Button {
            Task { await loadProjections() }
          } label: {
            Image(systemName: "arrow.clockwise")
          }
          .disabled(isLoading || !runtime.isEnrolled)
        }
      }
      .task {
        await loadProjections()
      }
      .onChange(of: selectedMetric) { _, _ in
        Task { await loadProjections() }
      }
      .onChange(of: dateRange) { _, _ in
        Task { await loadProjections() }
      }
    }
  }

  @ViewBuilder
  private var projectionChart: some View {
    let validPoints = projectionData.filter { $0.value != nil }
    let unit = metrics.first { $0.0 == selectedMetric }?.2 ?? ""

    Chart {
      ForEach(validPoints) { point in
        if let value = point.value {
          LineMark(
            x: .value("Time", point.timestamp),
            y: .value(selectedMetricLabel, value)
          )
          .foregroundStyle(metricColor)

          PointMark(
            x: .value("Time", point.timestamp),
            y: .value(selectedMetricLabel, value)
          )
          .foregroundStyle(metricColor)
          .symbolSize(20)
        }
      }

      // Show gaps as rule marks
      ForEach(missingIntervals.indices, id: \.self) { index in
        let interval = missingIntervals[index]
        if interval.count >= 2,
           let start = parseDate(interval[0]),
           let end = parseDate(interval[1]) {
          RectangleMark(
            xStart: .value("Gap Start", start),
            xEnd: .value("Gap End", end)
          )
          .foregroundStyle(.gray.opacity(0.2))
        }
      }
    }
    .chartXAxis {
      AxisMarks(values: .automatic) { _ in
        AxisGridLine()
        AxisValueLabel(format: dateRange == .day ? .dateTime.hour() : .dateTime.weekday(.abbreviated))
      }
    }
    .chartYAxis {
      AxisMarks { value in
        AxisGridLine()
        AxisValueLabel {
          if let doubleValue = value.as(Double.self) {
            Text("\(doubleValue.formatted()) \(unit)")
          }
        }
      }
    }
    .padding()
  }

  private var selectedMetricLabel: String {
    metrics.first { $0.0 == selectedMetric }?.1 ?? "Value"
  }

  private var metricColor: Color {
    switch selectedMetric {
    case "hr", "resting_hr": return .red
    case "spo2": return .green
    case "respiratory_rate": return .blue
    case "step_count": return .orange
    default: return .primary
    }
  }

  private func parseDate(_ string: String) -> Date? {
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    return formatter.date(from: string) ?? {
      formatter.formatOptions = [.withInternetDateTime]
      return formatter.date(from: string)
    }()
  }

  private func loadProjections() async {
    guard runtime.isEnrolled else {
      projectionData = []
      return
    }

    isLoading = true
    loadError = nil
    defer { isLoading = false }

    let now = Date()
    let start = now.addingTimeInterval(-dateRange.interval)
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]

    // TODO: Use ProjectionReadService to fetch real data
    // For now, generate sample data for demonstration
    var points: [ChartableProjection] = []
    let interval: TimeInterval = dateRange == .day ? 3600 : (dateRange == .week ? 3600 * 4 : 86400)

    var current = start
    while current <= now {
      let baseValue: Double
      switch selectedMetric {
      case "hr": baseValue = 72
      case "spo2": baseValue = 97
      case "respiratory_rate": baseValue = 14
      case "resting_hr": baseValue = 62
      case "step_count": baseValue = 8000
      default: baseValue = 50
      }

      // Simulate some variation and occasional gaps
      let hasValue = Double.random(in: 0...1) > 0.1
      let point = ProjectionPoint(
        timestamp: formatter.string(from: current),
        value: hasValue ? baseValue + Double.random(in: -5...5) : nil
      )
      points.append(ChartableProjection(point: point, metric: selectedMetric))
      current = current.addingTimeInterval(interval)
    }

    projectionData = points
    missingIntervals = [] // TODO: Parse from actual Pi response
  }
}

/// Wire type matching the Pi API schema.
struct ProjectionPoint: Codable, Equatable, Sendable {
  let timestamp: String
  let value: Double?
}

#Preview {
  HistorySurface()
    .environmentObject(AppClientRuntime())
}
