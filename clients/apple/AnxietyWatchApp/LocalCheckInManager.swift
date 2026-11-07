import Foundation
#if canImport(UserNotifications)
@preconcurrency import UserNotifications
#endif

/// Manages random check-in notifications for periodic anxiety capture.
/// The phone schedules local notifications during active hours; the person answers on iPhone or Watch.
public enum RandomCheckInManager {
  private static let isEnabledKey = "RandomCheckIn_isEnabled"
  private static let frequencyKey = "RandomCheckIn_frequencyPerDay"
  private static let startHourKey = "RandomCheckIn_startHour"
  private static let endHourKey = "RandomCheckIn_endHour"
  private static let lastScheduledKey = "RandomCheckIn_lastScheduled"
  private static let pendingPromptIDKey = "RandomCheckIn_pendingPromptID"

  private static let notificationCategoryID = "ANXIETY_CHECK_IN"
  private static let notificationIdentifierPrefix = "random-checkin-"

  // MARK: - Settings

  public static var isEnabled: Bool {
    get { UserDefaults.standard.bool(forKey: isEnabledKey) }
    set {
      UserDefaults.standard.set(newValue, forKey: isEnabledKey)
      if newValue {
        Task { await scheduleNextCheckIn() }
      } else {
        cancelAllScheduled()
      }
    }
  }

  public static var frequencyPerDay: Int {
    get {
      let value = UserDefaults.standard.integer(forKey: frequencyKey)
      return value > 0 ? value : 2
    }
    set { UserDefaults.standard.set(max(1, min(10, newValue)), forKey: frequencyKey) }
  }

  public static var activeHoursStart: Int {
    get {
      let value = UserDefaults.standard.integer(forKey: startHourKey)
      return (0...23).contains(value) ? value : 8
    }
    set { UserDefaults.standard.set(max(0, min(23, newValue)), forKey: startHourKey) }
  }

  public static var activeHoursEnd: Int {
    get {
      let value = UserDefaults.standard.integer(forKey: endHourKey)
      return (0...23).contains(value) ? value : 22
    }
    set { UserDefaults.standard.set(max(0, min(23, newValue)), forKey: endHourKey) }
  }

  // MARK: - Scheduling

  public static func scheduleNextCheckIn(from now: Date = Date()) async {
    guard isEnabled else { return }

    #if canImport(UserNotifications)
    let center = UNUserNotificationCenter.current()

    // Ensure notification permissions
    let settings = await center.notificationSettings()
    guard settings.authorizationStatus == .authorized
      || settings.authorizationStatus == .provisional
    else { return }

    // Calculate next random time within active hours
    let calendar = Calendar.current
    var nextDate = now

    // If outside active hours, move to next active window
    let hour = calendar.component(.hour, from: now)
    if hour < activeHoursStart {
      nextDate = calendar.date(bySettingHour: activeHoursStart, minute: 0, second: 0, of: now) ?? now
    } else if hour >= activeHoursEnd {
      // Move to next day's start
      let tomorrow = calendar.date(byAdding: .day, value: 1, to: now) ?? now
      nextDate = calendar.date(bySettingHour: activeHoursStart, minute: 0, second: 0, of: tomorrow) ?? tomorrow
    }

    // Calculate random offset within active window
    let activeMinutes = (activeHoursEnd - activeHoursStart) * 60
    let intervalMinutes = activeMinutes / frequencyPerDay
    let randomOffset = Int.random(in: 0..<max(1, intervalMinutes))
    nextDate = calendar.date(byAdding: .minute, value: randomOffset, to: nextDate) ?? nextDate

    // Ensure it's in the future
    if nextDate <= now {
      nextDate = calendar.date(byAdding: .minute, value: max(15, intervalMinutes), to: now) ?? now.addingTimeInterval(900)
    }

    // Create notification
    let promptID = UUID()
    let content = UNMutableNotificationContent()
    content.title = "How are you feeling?"
    content.body = "Take a moment to log your current anxiety level."
    content.sound = .default
    content.categoryIdentifier = notificationCategoryID
    content.userInfo = ["prompt_id": promptID.uuidString, "type": "random_check_in"]

    let trigger = UNTimeIntervalNotificationTrigger(
      timeInterval: nextDate.timeIntervalSince(now),
      repeats: false
    )

    let request = UNNotificationRequest(
      identifier: "\(notificationIdentifierPrefix)\(promptID.uuidString)",
      content: content,
      trigger: trigger
    )

    do {
      try await center.add(request)
      UserDefaults.standard.set(nextDate.timeIntervalSince1970, forKey: lastScheduledKey)
      UserDefaults.standard.set(promptID.uuidString, forKey: pendingPromptIDKey)
    } catch {
      // Silently fail; will retry on next app launch
    }
    #endif
  }

  public static func cancelAllScheduled() {
    #if canImport(UserNotifications)
    let center = UNUserNotificationCenter.current()
    center.getPendingNotificationRequests { requests in
      let ids = requests
        .filter { $0.identifier.hasPrefix(notificationIdentifierPrefix) }
        .map(\.identifier)
      center.removePendingNotificationRequests(withIdentifiers: ids)
    }
    #endif
  }

  // MARK: - Response Handling

  public static var pendingPromptID: UUID? {
    guard let string = UserDefaults.standard.string(forKey: pendingPromptIDKey) else { return nil }
    return UUID(uuidString: string)
  }

  public static func completeCheckIn() {
    UserDefaults.standard.removeObject(forKey: pendingPromptIDKey)
    // Schedule the next check-in
    Task { await scheduleNextCheckIn() }
  }

  // MARK: - Category Registration

  public static func registerNotificationCategory() {
    #if canImport(UserNotifications)
    let center = UNUserNotificationCenter.current()

    // Quick response actions
    let actions = (1...10).map { level in
      UNNotificationAction(
        identifier: "anxiety_\(level)",
        title: "\(level)",
        options: []
      )
    }

    let category = UNNotificationCategory(
      identifier: notificationCategoryID,
      actions: Array(actions.prefix(5)), // Show first 5 in notification
      intentIdentifiers: [],
      options: .customDismissAction
    )

    center.setNotificationCategories([category])
    #endif
  }
}

/// Manages 30-minute follow-up prompts after logging promptable medication doses.
public enum DoseFollowUpManager {
  private static let pendingFollowUpsKey = "DoseFollowUp_pending"
  private static let followUpDelay: TimeInterval = 30 * 60 // 30 minutes

  private static let notificationCategoryID = "DOSE_FOLLOW_UP"
  private static let notificationIdentifierPrefix = "dose-followup-"

  public struct PendingFollowUp: Codable, Equatable, Sendable {
    public let doseID: UUID
    public let medicationName: String
    public let scheduledAt: Date
    public let promptID: UUID

    public init(doseID: UUID, medicationName: String, scheduledAt: Date, promptID: UUID = UUID()) {
      self.doseID = doseID
      self.medicationName = medicationName
      self.scheduledAt = scheduledAt
      self.promptID = promptID
    }
  }

  // MARK: - Scheduling

  public static func scheduleFollowUp(doseID: UUID, medicationName: String, from now: Date = Date()) async {
    let followUp = PendingFollowUp(
      doseID: doseID,
      medicationName: medicationName,
      scheduledAt: now.addingTimeInterval(followUpDelay)
    )

    // Store in pending list
    var pending = loadPendingFollowUps()
    pending.append(followUp)
    savePendingFollowUps(pending)

    #if canImport(UserNotifications)
    let center = UNUserNotificationCenter.current()

    let content = UNMutableNotificationContent()
    content.title = "How are you feeling now?"
    content.body = "It's been 30 minutes since you took \(medicationName). Log your current anxiety."
    content.sound = .default
    content.categoryIdentifier = notificationCategoryID
    content.userInfo = [
      "prompt_id": followUp.promptID.uuidString,
      "dose_id": doseID.uuidString,
      "medication_name": medicationName,
      "type": "dose_follow_up"
    ]

    let trigger = UNTimeIntervalNotificationTrigger(
      timeInterval: followUpDelay,
      repeats: false
    )

    let request = UNNotificationRequest(
      identifier: "\(notificationIdentifierPrefix)\(followUp.promptID.uuidString)",
      content: content,
      trigger: trigger
    )

    try? await center.add(request)
    #endif
  }

  public static func cancelFollowUp(doseID: UUID) {
    var pending = loadPendingFollowUps()
    guard let followUp = pending.first(where: { $0.doseID == doseID }) else { return }

    pending.removeAll { $0.doseID == doseID }
    savePendingFollowUps(pending)

    #if canImport(UserNotifications)
    let center = UNUserNotificationCenter.current()
    center.removePendingNotificationRequests(
      withIdentifiers: ["\(notificationIdentifierPrefix)\(followUp.promptID.uuidString)"]
    )
    #endif
  }

  // MARK: - Response Handling

  public static func pendingFollowUpIfDue(for doseID: UUID? = nil, now: Date = Date()) -> PendingFollowUp? {
    let pending = loadPendingFollowUps()
    if let doseID {
      return pending.first { $0.doseID == doseID && $0.scheduledAt <= now }
    }
    return pending.first { $0.scheduledAt <= now }
  }

  public static func completeFollowUp(doseID: UUID) {
    var pending = loadPendingFollowUps()
    pending.removeAll { $0.doseID == doseID }
    savePendingFollowUps(pending)
  }

  // MARK: - Persistence

  private static func loadPendingFollowUps() -> [PendingFollowUp] {
    guard let data = UserDefaults.standard.data(forKey: pendingFollowUpsKey) else { return [] }
    return (try? JSONDecoder().decode([PendingFollowUp].self, from: data)) ?? []
  }

  private static func savePendingFollowUps(_ followUps: [PendingFollowUp]) {
    guard let data = try? JSONEncoder().encode(followUps) else { return }
    UserDefaults.standard.set(data, forKey: pendingFollowUpsKey)
  }

  // MARK: - Category Registration

  public static func registerNotificationCategory() {
    #if canImport(UserNotifications)
    let center = UNUserNotificationCenter.current()

    let category = UNNotificationCategory(
      identifier: notificationCategoryID,
      actions: [],
      intentIdentifiers: [],
      options: .customDismissAction
    )

    center.setNotificationCategories([category])
    #endif
  }
}
