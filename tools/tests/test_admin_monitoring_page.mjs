#!/usr/bin/env node
// Focused checks for the read-only admin monitoring/history read model.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const pagePath = resolve(root, "docs/design/admin-console/claude-design/a/monitoring.html");
const scriptPath = resolve(root, "docs/design/admin-console/claude-design/a/admin-monitoring.js");
const page = readFileSync(pagePath, "utf8");
const script = readFileSync(scriptPath, "utf8");

assert.match(page, /admin-monitoring\.js/);
assert.match(page, /READ ONLY/);
assert.doesNotMatch(page, /<form\b/i);
assert.doesNotMatch(page, /method\s*=\s*["'](?:POST|PUT|PATCH|DELETE)/i);
assert.doesNotMatch(page, /fetch\([^)]*method\s*:/i);

const context = { Date, Math, Number, String, Object, Array };
context.globalThis = context;
vm.runInNewContext(script, context);
const monitoring = context.AdminMonitoring;
assert.ok(monitoring);

const now = 1_700_000_000;
const point = (session_id, seq, value, source = 1, type = 1) => ({
  t: now - seq, v: value, source, type, session_id, seq,
});
const status = {
  schema: 2,
  written_at: now - 1,
  h10: { hr: 72, sample_at: now - 2, identity: "source_session_seq", durability: "database_committed" },
  emay: { spo2: 97, pulse_bpm: 70, sample_at: now - 3, identity: "anonymous_not_durable", durability: "status_only_not_committed" },
  pipeline: {
    state: "healthy",
    stages: Object.fromEntries(monitoring.STAGES.map((stage, index) => [stage, { count: index + 1 }])),
  },
  pending_live: { h10: null, emay: null },
  pending_durable: { hr: [], rr: [], spo2: [], pulse: [] },
  gaps: [],
};
const history = {
  schema: 2,
  written_at: now - 2,
  history_window_seconds: 86_400,
  gaps: [],
  h10_history: { hr: [point(4, 1, 72), point(5, 1, 72)], rr: [point(4, 2, 0.84, 1, 5)] },
  emay_history: { spo2: [point(8, 1, 97, 0, 2)], pulse: [point(8, 1, 70, 0, 1)] },
};
const health = { schema: 1, checked_at: now - 2, overall: "OK", components: {} };
const drain = { schema: 1, written_at: now - 2 };
const validated = {
  status: monitoring.validateStatus(status, now),
  history: monitoring.validateHistory(history, now),
  health: monitoring.validateHealth(health, now),
  drain: monitoring.validateDrain(drain, now),
};
for (const [name, result] of Object.entries(validated)) assert.equal(result.ok, true, name);
const model = monitoring.buildModel(validated, now);
assert.equal(model.state, "healthy");
assert.equal(model.counts["h10.hr"], 2);
assert.equal(model.counts["emay.spo2"], 1);
assert.equal(model.stages.length, 7);
assert.equal(model.gaps.length, 0);

const sameReadingDifferentIdentity = {
  ...history,
  h10_history: { ...history.h10_history, hr: [point(4, 1, 72), point(5, 1, 72)] },
};
assert.equal(monitoring.validateHistory(sameReadingDifferentIdentity, now).ok, true);
assert.equal(monitoring.validateStatus(status, now + 31).ok, false, "stale status must fail closed");
assert.equal(
  monitoring.validateHistory({ ...history, h10_history: { hr: [{ t: now, v: 72 }], rr: [] } }, now).ok,
  false,
  "unidentified history must fail closed",
);
const missing = monitoring.buildModel({
  status: { ok: false, reason: "status file missing" },
  history: { ok: false, reason: "history file missing" },
  health: { ok: false, reason: "health file stale" },
  drain: { ok: false, reason: "drainer evidence missing" },
}, now);
assert.equal(missing.state, "unknown");
assert.equal(missing.gaps.length, 4);

console.log("PASS admin monitoring read-only, identity, bounds, and fail-closed checks");
