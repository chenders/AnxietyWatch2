"use strict";

(function exposeAdminMonitoring(root) {
  const STAGES = [
    "received",
    "spooled",
    "fsynced",
    "drained",
    "committed",
    "published",
    "displayed",
  ];
  const SOURCES = ["h10", "emay"];
  const MAX_STATUS_AGE = 30;
  const MAX_HISTORY_AGE = 600;
  const MAX_HEALTH_AGE = 120;
  const MAX_HISTORY_WINDOW = 7 * 24 * 60 * 60;
  const MAX_HISTORY_POINTS = 1_000_000;

  function finite(value) {
    return typeof value === "number" && Number.isFinite(value);
  }

  function failure(reason) {
    return { ok: false, reason };
  }

  function validAge(timestamp, now, maximum) {
    return finite(timestamp) && finite(now) && now - timestamp >= 0 && now - timestamp <= maximum;
  }

  function validateStatus(value, now = Date.now() / 1000) {
    if (!value || typeof value !== "object" || value.schema !== 2)
      return failure("status schema is missing or unsupported");
    if (!validAge(value.written_at, now, MAX_STATUS_AGE))
      return failure("status evidence is missing, future-dated, or stale");
    if (typeof value.error === "string" && value.error.length)
      return failure(`status publisher degraded: ${value.error}`);
    if (!value.pipeline || typeof value.pipeline !== "object")
      return failure("pipeline evidence is missing");
    if (!["healthy", "degraded", "unknown"].includes(value.pipeline.state))
      return failure("pipeline state is malformed");
    if (!value.pending_live || typeof value.pending_live !== "object"
      || !("h10" in value.pending_live) || !("emay" in value.pending_live)
      || !value.pending_durable || typeof value.pending_durable !== "object")
      return failure("pending/live evidence is missing");
    if (!Array.isArray(value.gaps)
      || value.gaps.some(gap => !gap || typeof gap !== "object"
        || typeof gap.source !== "string" || typeof gap.reason !== "string"))
      return failure("explicit gap evidence is malformed");
    const stages = value.pipeline.stages;
    if (!stages || typeof stages !== "object" || STAGES.some(stage => !(stage in stages)))
      return failure("pipeline stage evidence is incomplete");
    const pending = value.pending_durable;
    if (!Array.isArray(pending.hr) || !validateSeries(pending.hr, 1, 1)
      || !Array.isArray(pending.rr) || !validateSeries(pending.rr, 1, 5)
      || !Array.isArray(pending.spo2) || !validateSeries(pending.spo2, 0, 2)
      || !Array.isArray(pending.pulse) || !validateSeries(pending.pulse, 0, 1))
      return failure("pending durable series or identity is malformed");
    return { ok: true, value };
  }

  function validPoint(point, source, type) {
    return point && typeof point === "object"
      && finite(point.t) && finite(point.v)
      && point.source === source && point.type === type
      && Number.isInteger(point.source) && Number.isInteger(point.type)
      && Number.isInteger(point.session_id) && Number.isInteger(point.seq);
  }

  function validateSeries(series, source, type) {
    return Array.isArray(series) && series.length <= MAX_HISTORY_POINTS
      && series.every(point => validPoint(point, source, type));
  }

  function validateHistory(value, now = Date.now() / 1000) {
    if (!value || typeof value !== "object" || value.schema !== 2)
      return failure("history schema is missing or unsupported");
    if (!validAge(value.written_at, now, MAX_HISTORY_AGE))
      return failure("history evidence is missing, future-dated, or stale");
    if (!finite(value.history_window_seconds)
      || value.history_window_seconds <= 0
      || value.history_window_seconds > MAX_HISTORY_WINDOW)
      return failure("history window is missing or unbounded");
    if (!Array.isArray(value.gaps)
      || value.gaps.some(gap => !gap || typeof gap !== "object"
        || typeof gap.source !== "string" || typeof gap.reason !== "string"))
      return failure("explicit history gaps are malformed");
    const h10 = value.h10_history;
    const emay = value.emay_history;
    if (!h10 || !validateSeries(h10.hr, 1, 1) || !validateSeries(h10.rr, 1, 5)
      || !emay || !validateSeries(emay.spo2, 0, 2) || !validateSeries(emay.pulse, 0, 1))
      return failure("history series or durable identity is malformed");
    return { ok: true, value };
  }

  function validateHealth(value, now = Date.now() / 1000) {
    if (!value || typeof value !== "object" || value.schema !== 1)
      return failure("health schema is missing or unsupported");
    if (!validAge(value.checked_at, now, MAX_HEALTH_AGE))
      return failure("health evidence is missing, future-dated, or stale");
    if (typeof value.overall !== "string")
      return failure("health overall state is missing");
    if (!value.components || typeof value.components !== "object" || Array.isArray(value.components))
      return failure("health components are missing");
    return { ok: true, value };
  }

  function validateDrain(value, now = Date.now() / 1000) {
    if (!value || typeof value !== "object" || !Number.isInteger(value.schema))
      return failure("drainer health schema is missing or unsupported");
    const timestamp = value.written_at ?? value.checked_at;
    if (!validAge(timestamp, now, MAX_HEALTH_AGE))
      return failure("drainer health evidence is missing, future-dated, or stale");
    return { ok: true, value };
  }

  function checked(result, label) {
    return result && result.ok
      ? result
      : { ok: false, reason: `${label}: ${result && result.reason ? result.reason : "evidence unavailable"}` };
  }

  function gapList(statusResult, historyResult, healthResult, drainResult) {
    const gaps = [];
    if (!statusResult.ok) gaps.push({ source: "status", reason: statusResult.reason });
    if (!historyResult.ok) gaps.push({ source: "history", reason: historyResult.reason });
    if (!healthResult.ok) gaps.push({ source: "health", reason: healthResult.reason });
    if (!drainResult.ok) gaps.push({ source: "drainer", reason: drainResult.reason });
    if (statusResult.ok) {
      for (const gap of statusResult.value.gaps) {
        if (gap && typeof gap === "object")
          gaps.push({ source: String(gap.source ?? "unknown"), reason: String(gap.reason ?? "unspecified gap") });
      }
    }
    if (historyResult.ok) {
      for (const gap of historyResult.value.gaps) {
        if (gap && typeof gap === "object")
          gaps.push({ source: String(gap.source ?? "unknown"), reason: String(gap.reason ?? "unspecified gap") });
      }
    }
    return gaps;
  }

  function buildModel(inputs, now = Date.now() / 1000) {
    const status = checked(inputs.status, "status");
    const history = checked(inputs.history, "history");
    const health = checked(inputs.health, "health");
    const drain = checked(inputs.drain, "drainer");
    const statusValue = status.ok ? status.value : null;
    const pipeline = statusValue && statusValue.pipeline;
    const stageValues = pipeline && pipeline.stages ? pipeline.stages : {};
    const sourceValues = {};
    for (const source of SOURCES) {
      sourceValues[source] = statusValue && statusValue[source] ? statusValue[source] : null;
    }
    const historyValue = history.ok ? history.value : null;
    const counts = {};
    if (historyValue) {
      for (const [source, metrics] of [["h10", historyValue.h10_history], ["emay", historyValue.emay_history]]) {
        if (!metrics || typeof metrics !== "object") continue;
        for (const [metric, points] of Object.entries(metrics)) {
          if (Array.isArray(points)) counts[`${source}.${metric}`] = points.length;
        }
      }
    }
    const pending = [];
    if (statusValue) {
      if (statusValue.pending_live.h10) pending.push("H10 LIVE OVERLAY · NOT COMMITTED");
      if (statusValue.pending_live.emay) pending.push("EMAY LIVE OVERLAY · ANONYMOUS · NOT COMMITTED");
      const durablePending = ["hr", "rr", "spo2", "pulse"].reduce((total, key) =>
        total + (Array.isArray(statusValue.pending_durable[key]) ? statusValue.pending_durable[key].length : 0), 0);
      if (durablePending) pending.push(`${durablePending} DURABLE ROWS PENDING HISTORY REFRESH`);
    }
    return {
      now,
      status,
      history,
      health,
      drain,
      sourceValues,
      pipeline: pipeline && status.ok ? pipeline : null,
      stages: STAGES.map(stage => ({ name: stage, value: status.ok ? stageValues[stage] : null })),
      counts,
      pending,
      gaps: gapList(status, history, health, drain),
      state: status.ok && history.ok && health.ok && drain.ok
        && pipeline && pipeline.state === "healthy" ? "healthy"
        : status.ok || history.ok || health.ok || drain.ok ? "degraded" : "unknown",
    };
  }

  function displayValue(value) {
    if (value == null) return "UNKNOWN";
    if (finite(value)) return String(Math.round(value));
    if (typeof value === "string") return value.toUpperCase();
    if (typeof value === "object") {
      if (finite(value.count)) return `COUNT ${Math.round(value.count)}`;
      if (typeof value.state === "string") return value.state.toUpperCase();
      return "EVIDENCE";
    }
    return "UNKNOWN";
  }

  function sourceSummary(source, value) {
    if (!value || typeof value !== "object") return `${source.toUpperCase()} · UNKNOWN`;
    const reading = source === "h10"
      ? `HR ${finite(value.hr) ? Math.round(value.hr) : "—"} bpm`
      : `SpO₂ ${finite(value.spo2) ? Math.round(value.spo2) : "—"}% · pulse ${finite(value.pulse_bpm) ? Math.round(value.pulse_bpm) : "—"}`;
    const identity = value.identity || (value.live ? "live overlay" : "unknown identity");
    const durability = value.durability || "durability unknown";
    return `${source.toUpperCase()} · ${reading} · ${identity} · ${durability}`;
  }

  function clear(element) {
    while (element.firstChild) element.removeChild(element.firstChild);
  }

  function appendLine(parent, className, value) {
    const line = document.createElement("div");
    line.className = className;
    line.textContent = value;
    parent.append(line);
    return line;
  }

  function renderModel(model, documentObject = document) {
    const state = documentObject.getElementById("monitor-state");
    state.textContent = model.state.toUpperCase();
    state.className = `monitor-state ${model.state}`;

    const sources = documentObject.getElementById("source-list");
    clear(sources);
    for (const source of SOURCES) appendLine(sources, "monitor-line", sourceSummary(source, model.sourceValues[source]));

    const stages = documentObject.getElementById("stage-list");
    clear(stages);
    for (const stage of model.stages) {
      const line = appendLine(stages, "monitor-line", `${stage.name.toUpperCase()} · ${displayValue(stage.value)}`);
      if (stage.value == null) line.classList.add("unknown");
    }

    const history = documentObject.getElementById("history-list");
    clear(history);
    if (!model.history.ok) {
      appendLine(history, "monitor-line unknown", `HISTORY · UNKNOWN · ${model.history.reason}`);
    } else {
      for (const [key, count] of Object.entries(model.counts).sort())
        appendLine(history, "monitor-line", `${key.toUpperCase()} · ${count} BOUNDED, IDENTITY-KEYED POINTS`);
      if (!Object.keys(model.counts).length) appendLine(history, "monitor-line unknown", "HISTORY · NO POINTS EVIDENCED");
    }

    const gaps = documentObject.getElementById("gap-list");
    clear(gaps);
    for (const pending of model.pending) appendLine(gaps, "monitor-line", `PENDING · ${pending}`);
    if (!model.gaps.length) appendLine(gaps, "monitor-line", "NO EXPLICIT GAPS REPORTED; NO VALUES WERE FILLED");
    else for (const gap of model.gaps) appendLine(gaps, "monitor-line gap", `${gap.source.toUpperCase()} · ${gap.reason}`);

    const evidence = documentObject.getElementById("evidence-list");
    clear(evidence);
    for (const [name, result] of [["STATUS", model.status], ["HISTORY", model.history], ["HEALTH", model.health], ["DRAINER", model.drain]])
      appendLine(evidence, `monitor-line ${result.ok ? "" : "unknown"}`, `${name} · ${result.ok ? "VALID" : `UNKNOWN · ${result.reason}`}`);
  }

  root.AdminMonitoring = {
    STAGES,
    validateStatus,
    validateHistory,
    validateHealth,
    validateDrain,
    buildModel,
    renderModel,
  };
})(typeof globalThis === "object" ? globalThis : this);
