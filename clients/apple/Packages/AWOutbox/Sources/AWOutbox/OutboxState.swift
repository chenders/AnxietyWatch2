/// Delivery evidence only. Payload persistence and terminal deletion arrive in T-I5.
public enum OutboxState: String, Codable, Equatable, Sendable {
  case pending
  case retryable
  case rejected
  case conflict
  case acknowledged

  public var isTerminal: Bool {
    switch self {
    case .rejected, .conflict, .acknowledged:
      true
    case .pending, .retryable:
      false
    }
  }
}
