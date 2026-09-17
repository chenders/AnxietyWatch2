import Foundation
import Testing

@testable import AWHealthKit

private let registryData = Data(
  #"""
  {
    "type": {
      "hr": {"id": 1, "unit": "bpm", "range": [20.0, 250.0], "precision": 1.0},
      "hrv_rmssd": {"id": 4, "unit": "ms", "range": [1.0, 500.0], "precision": 0.1},
      "sleep_stage": {"id": 9, "unit": "stage_code", "range": [0.0, 4.0], "precision": 1.0}
    },
    "source": {
      "healthkit": {"id": 2, "transport": "push", "provides": ["hr", "hrv_rmssd", "sleep_stage"], "expected_cadence_s": 21600}
    }
  }
  """#.utf8
)

private func validRegistry() throws -> RegistryDocument {
  try RegistryDocument.decode(registryData)
}

private func quantitySample(value: Double? = 72.0, metadata: [String: String] = [:]) -> HealthKitTransportSample {
  HealthKitTransportSample(
    healthKitSampleUUID: UUID(uuidString: "00000000-0000-4000-8000-000000000101")!,
    healthKitType: PlatformTypeIdentifier(rawValue: "HKQuantityTypeIdentifierHeartRate")!,
    startAt: "2025-01-15T08:00:00Z",
    endAt: "2025-01-15T08:00:05Z",
    originalValue: value,
    originalUnit: "count/min",
    sourceRevision: HealthKitSourceRevision(sourceName: "Fixture Watch"),
    metadata: metadata,
    queryBatchID: UUID(uuidString: "00000000-0000-7000-8000-000000000201")!,
    recordedAt: "2025-01-15T08:01:00Z"
  )
}

@Test("AWRegistryTests validate registry ranges and source references")
func registryValidationRejectsUnknownProvidedType() throws {
  let document = try validRegistry()
  #expect(document.type(named: "hr")?.unit == "bpm")
  #expect(document.source(named: "healthkit")?.provides.contains("sleep_stage") == true)

  let invalid = Data(#"{"type":{"hr":{"id":1,"unit":"bpm","range":[20,10],"precision":1}},"source":{"healthkit":{"id":2,"transport":"push","provides":["hr"]}}}"#.utf8)
  #expect(throws: RegistryError.invalidType("hr")) {
    try RegistryDocument.decode(invalid)
  }
}

@Test("AWHealthKitMapperTests preserve original quantity and provenance")
func quantityMappingPreservesTransportFacts() throws {
  let mapper = try HealthKitMapper(registry: validRegistry())
  let sample = quantitySample()
  let mapped = try mapper.map(sample)
  #expect(mapped == sample)
  let envelope = try mapper.envelope(for: [sample])
  #expect(envelope.class == "platform_sample_batch")
  #expect(envelope.schemaVersion == 1)
  #expect(envelope.samples.first?.healthKitSampleUUID == sample.healthKitSampleUUID)
  #expect(envelope.samples.first?.sourceRevision.sourceName == "Fixture Watch")
}

@Test("AWHealthKitMapperTests reject unsupported, non-finite, and unknown metadata")
func mapperRejectsUnsafeInputs() throws {
  let mapper = try HealthKitMapper(registry: validRegistry())
  #expect(throws: HealthKitMappingError.unsupportedType("HKQuantityTypeIdentifierBodyTemperature")) {
    let unsupported = HealthKitTransportSample(
      healthKitSampleUUID: UUID(),
      healthKitType: PlatformTypeIdentifier(rawValue: "HKQuantityTypeIdentifierBodyTemperature")!,
      startAt: "2025-01-15T08:00:00Z",
      endAt: "2025-01-15T08:00:01Z",
      originalValue: 90,
      originalUnit: "degC",
      queryBatchID: UUID(),
      recordedAt: "2025-01-15T08:01:00Z"
    )
    _ = try mapper.map(unsupported)
  }
  #expect(throws: HealthKitMappingError.nonFiniteValue("hr value=nan")) {
    _ = try mapper.map(quantitySample(value: .nan))
  }
  #expect(throws: HealthKitMappingError.unknownMetadata("private")) {
    _ = try mapper.map(quantitySample(metadata: ["private": "secret"]))
  }
}

@Test("AWHealthKitMapperTests preserve sleep category without local canonical conversion")
func sleepMappingPreservesCategory() throws {
  let mapper = try HealthKitMapper(registry: validRegistry())
  let sample = HealthKitTransportSample(
    healthKitSampleUUID: UUID(uuidString: "00000000-0000-4000-8000-000000000102")!,
    healthKitType: PlatformTypeIdentifier(rawValue: "HKCategoryTypeIdentifierSleepAnalysis")!,
    startAt: "2025-01-15T01:00:00Z",
    endAt: "2025-01-15T01:30:00Z",
    originalCategory: "HKCategoryValueSleepAnalysisAsleepCore",
    queryBatchID: UUID(uuidString: "00000000-0000-7000-8000-000000000202")!,
    recordedAt: "2025-01-15T08:01:00Z"
  )
  let mapped = try mapper.map(sample)
  #expect(mapped.originalCategory == "HKCategoryValueSleepAnalysisAsleepCore")
  #expect(mapped.originalValue == nil)
}
