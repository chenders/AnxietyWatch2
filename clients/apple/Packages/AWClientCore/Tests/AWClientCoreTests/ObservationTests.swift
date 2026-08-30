import Foundation
import Testing

@testable import AWClientCore

@Test("unknown observations contain no reassuring numeric value")
func unknownHasNoValue() {
  let observation = Observation<Double>.unknown
  #expect(observation.value == nil)
  #expect(!observation.isStale)
}

@Test("stale observations preserve their evidence and state")
func stalePreservesEvidence() {
  let observedAt = Date(timeIntervalSince1970: 1_700_000_000)
  let asOf = Date(timeIntervalSince1970: 1_700_000_300)
  let observation = Observation.stale(value: 71.0, observedAt: observedAt, asOf: asOf)
  #expect(observation.value == 71.0)
  #expect(observation.isStale)
}
