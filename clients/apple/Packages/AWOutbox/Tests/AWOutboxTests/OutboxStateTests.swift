import Testing

@testable import AWOutbox

@Test("pending and retryable writes are never terminal")
func pendingIsNotTerminal() {
  #expect(!OutboxState.pending.isTerminal)
  #expect(!OutboxState.retryable.isTerminal)
  #expect(OutboxState.acknowledged.isTerminal)
}
