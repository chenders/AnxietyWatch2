import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const read = name => readFileSync(join(root, name), "utf8");
const FIXED_NOW_MS = Date.parse("2026-08-07T03:10:00Z");

function loadShippedModel() {
  const context = { window: {} };
  vm.createContext(context);
  vm.runInContext(read("console-model.js"), context, { filename: "console-model.js" });
  return context.window.ConsoleModel;
}
function loadRenderer(direction) {
  const context = { window: {}, Date: class extends Date { static now() { return FIXED_NOW_MS; } } };
  vm.createContext(context);
  vm.runInContext(read(`dir-${direction}.js`), context, { filename: `dir-${direction}.js` });
  return context.window.ConsoleDirections[direction];
}
const helpers = {
  esc: value => String(value ?? ""),
  ageText: seconds => seconds == null ? "not observed" : `${seconds}s ago`,
  glyph: state => ({ LIVE: "●", FAILING: "▲", UNKNOWN: "?", NOT_CONFIGURED: "—" })[state],
};

export function runM1RegressionChecks() {
  const modelApi = loadShippedModel();
  const failed = modelApi.attemptCard({ id: 9, kind: "collector", subject: "PolarH10", started_at: "2026-08-07T03:06:00Z", expected_duration_s: 600, finished_at: "2026-08-07T03:06:30Z", outcome: "failed", effect: null, detail: "sensor disconnected" }, 240);
  assert.equal(failed.state, "FAILING");
  assert.match(failed.reason, /sensor disconnected/);

  const batteryMissing = "RTC battery presence is not exposed by the available timedatectl interface";
  const clockUnknown = modelApi.clockCard({ ntp_synchronized: true, rtc_time_measured: true, rtc_battery_unmeasured_reason: batteryMissing }, "expect ≤ 45s");
  const clockFailing = modelApi.clockCard({ ntp_synchronized: false, rtc_time_measured: true, rtc_battery_unmeasured_reason: batteryMissing }, "expect ≤ 45s");
  assert.equal(clockUnknown.state, "UNKNOWN");
  assert.match(clockUnknown.reason, /RTC time reported/);
  assert.match(clockUnknown.reason, /battery presence is not exposed/);
  assert.equal(clockFailing.state, "FAILING");

  const [buildUnknown, schemaUnknown] = modelApi.identityCards({ version: "0.1.0", git_commit: "UNKNOWN", build_epoch_s: "UNKNOWN", schema_revision: null, pending_migrations: null, schema_unmeasured_reason: "database unavailable" });
  assert.equal(buildUnknown.state, "UNKNOWN");
  assert.match(buildUnknown.reason, /could not be measured/);
  assert.equal(schemaUnknown.state, "UNKNOWN");
  assert.match(schemaUnknown.reason, /database unavailable/);
  const [buildLive, schemaLive] = modelApi.identityCards({ version: "0.1.0", git_commit: "abc123", build_epoch_s: "1786130000", schema_revision: "sqitch:42", pending_migrations: 0 });
  assert.equal(buildLive.state, "LIVE");
  assert.match(buildLive.reason, /built at UTC epoch/);
  assert.equal(schemaLive.state, "LIVE");
  assert.match(schemaLive.reason, /pending migrations 0/);
  const [, schemaFailing] = modelApi.identityCards({ git_commit: "abc123", build_epoch_s: "1786130000", schema_revision: "sqitch:41", pending_migrations: 1 });
  assert.equal(schemaFailing.state, "FAILING");

  const lastKnown = { screen: "ingest", eyebrow: "Registry", title: "Ingest", intro: "Actual model", sections: [{ title: "Sources", cards: [failed] }] };
  const replacementErrorModel = { screen: "ingest", sections: [{ cards: [{ state: "UNKNOWN", reason: "fetch failed" }] }] };
  const feed = new modelApi.FeedState(45);
  feed.observe(FIXED_NOW_MS, lastKnown, true);
  feed.observe(FIXED_NOW_MS + 30_000, replacementErrorModel, false, "/api/health unavailable");
  let snapshot = feed.snapshot(FIXED_NOW_MS + 45_000);
  assert.equal(snapshot.stale, false, "cadence boundary is still current");
  assert.equal(snapshot.model, lastKnown, "failed refresh must retain the successful card model");
  assert.equal(snapshot.model.sections[0].cards[0].state, "FAILING");
  assert.match(snapshot.model.sections[0].cards[0].reason, /sensor disconnected/);
  snapshot = feed.snapshot(FIXED_NOW_MS + 46_000);
  assert.equal(snapshot.stale, true, "document must become stale after cadence");
  assert.equal(snapshot.model, lastKnown);
  assert.match(snapshot.failureReason, /unavailable/);
  const dom = { feedAge: { textContent: "" }, overlay: { hidden: true }, body: { dataset: {} }, detail: { textContent: "" }, expectSeconds: 45 };
  modelApi.applyFeedDom(snapshot, dom, seconds => `${seconds}s ago`);
  assert.equal(dom.overlay.hidden, false);
  assert.equal(dom.body.dataset.feed, "stale");
  assert.match(dom.detail.textContent, /latest refresh failed/);
  assert.match(dom.detail.textContent, /46s ago/);
  assert.equal(modelApi.advancingAge(240, FIXED_NOW_MS, FIXED_NOW_MS + 46_000), 286);
  assert.equal(modelApi.advancingAge(Number.NaN, FIXED_NOW_MS, FIXED_NOW_MS + 46_000), null);

  for (const direction of ["a", "b", "c"]) {
    const rendered = loadRenderer(direction)(snapshot.model, helpers);
    assert.match(rendered, /FAILING/);
    assert.match(rendered, /240s ago/);
    assert.match(rendered, /expect ≤ 600s/);
    assert.match(rendered, /sensor disconnected/);
    assert.match(rendered, /aria-hidden="true"/);
  }

  console.log("Reachable M1 regressions passed against shipped API-facing model/renderers: failed attempts, fixed ages/expectations, success→failure→cadence retention, factual build/schema identity, and honest clock evidence.");
}
