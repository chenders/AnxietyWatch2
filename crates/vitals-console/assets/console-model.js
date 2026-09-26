(() => {
  "use strict";

  const state = value => {
    const normalized = String(value || "UNKNOWN").toUpperCase();
    if (normalized === "NOT_CONFIGURED") return "NOT_CONFIGURED";
    if (normalized === "LIVE" || normalized === "SUCCEEDED") return "LIVE";
    if (normalized === "FAILING" || normalized === "FAILED") return "FAILING";
    return "UNKNOWN";
  };

  const normalizeCard = fact => {
    return {
      title: fact.title,
      state: state(fact.state),
      reason: fact.reason,
      age: fact.age ?? null,
      expectation: fact.expectation,
      detail: fact.detail || "",
    };
  };

  const clockCard = (clock, feedExpectation) => {
    if (!clock) return normalizeCard({ title: "Clock", state: "UNKNOWN", reason: "clock endpoint unavailable", expectation: feedExpectation });
    if (!clock.ntp_synchronized) {
      return normalizeCard({ title: "Clock", state: "FAILING", reason: `NTP synchronized=false; ${clock.rtc_battery_unmeasured_reason}`, age: 0, expectation: feedExpectation });
    }
    // RTCTimeUSec proves only that RTC time was reported. It cannot prove the
    // required battery is present, so clock health cannot be LIVE yet.
    return normalizeCard({ title: "Clock", state: "UNKNOWN", reason: `NTP synchronized=true; RTC time ${clock.rtc_time_measured ? "reported" : "not reported"}; ${clock.rtc_battery_unmeasured_reason}`, age: 0, expectation: feedExpectation });
  };

  const identityCards = version => {
    const buildMeasured = version?.git_commit && version.git_commit !== "UNKNOWN" && version?.build_epoch_s && version.build_epoch_s !== "UNKNOWN";
    const build = buildMeasured
      ? normalizeCard({ title: "Build identity", state: "LIVE", reason: `commit ${version.git_commit}; built at UTC epoch ${version.build_epoch_s}`, age: 0, expectation: "expect on every console build" })
      : normalizeCard({ title: "Build identity", state: "UNKNOWN", reason: "build commit/time could not be measured", age: null, expectation: "expect on every console build" });
    const schema = version?.schema_revision != null && version?.pending_migrations != null
      ? normalizeCard({ title: "Schema revision", state: version.pending_migrations === 0 ? "LIVE" : "FAILING", reason: `schema revision ${version.schema_revision}; pending migrations ${version.pending_migrations}`, age: 0, expectation: "expect on every console build" })
      : normalizeCard({ title: "Schema revision", state: "UNKNOWN", reason: version?.schema_unmeasured_reason || "schema revision/pending migrations not measured", age: null, expectation: "expect on every console build" });
    return [build, schema];
  };

  const attemptCard = (row, age) => normalizeCard({
    title: `${row.kind} · ${row.subject}`,
    state: row.outcome == null ? "UNKNOWN" : row.outcome,
    reason: row.outcome == null
      ? `attempt has no outcome; started ${row.started_at}`
      : `outcome=${row.outcome}; ${row.effect ? `effect created=${row.effect.created}, updated=${row.effect.updated}, replayed=${row.effect.replayed}, rejected=${row.effect.rejected}` : "effect not reported"}${row.detail ? `; ${row.detail}` : ""}`,
    age,
    expectation: `expect ≤ ${row.expected_duration_s}s`,
    detail: `Attempt #${row.id}`,
  });

  const advancingAge = (baseSeconds, observedAtMs, nowMs) => Number.isFinite(baseSeconds) && Number.isFinite(observedAtMs)
    ? baseSeconds + (nowMs - observedAtMs) / 1000
    : null;

  const applyFeedDom = (snapshot, elements, ageText) => {
    elements.feedAge.textContent = ageText(snapshot.elapsed);
    elements.overlay.hidden = !snapshot.stale;
    elements.body.dataset.feed = snapshot.stale ? "stale" : "fresh";
    elements.detail.textContent = snapshot.elapsed == null
      ? `No successful /api/health observation is available${snapshot.failureReason ? `: ${snapshot.failureReason}` : "."}`
      : `Last successful /api/health observation was ${ageText(snapshot.elapsed)}; expected within ${elements.expectSeconds}s${snapshot.failureReason ? `; latest refresh failed: ${snapshot.failureReason}` : "."}`;
  };

  class FeedState {
    constructor(expectSeconds) {
      this.expectSeconds = expectSeconds;
      this.lastSuccessMs = null;
      this.model = null;
      this.failureReason = null;
    }
    observe(nowMs, nextModel, healthSucceeded, failureReason = null) {
      if (healthSucceeded) {
        this.lastSuccessMs = nowMs;
        this.model = nextModel;
        this.failureReason = null;
      } else {
        // Preserve the complete last successful model. Before the first
        // success, the endpoint-error model is still useful evidence.
        if (this.model == null) this.model = nextModel;
        // A transport failure is document confidence, never a rewrite of
        // card outcomes/reasons.
        this.failureReason = failureReason || "status refresh failed";
      }
    }
    snapshot(nowMs) {
      const elapsed = this.lastSuccessMs == null ? null : Math.max(0, (nowMs - this.lastSuccessMs) / 1000);
      return { model: this.model, elapsed, stale: elapsed == null || elapsed > this.expectSeconds, failureReason: this.failureReason };
    }
  }

  window.ConsoleModel = { state, normalizeCard, clockCard, identityCards, attemptCard, advancingAge, applyFeedDom, FeedState };
})();
