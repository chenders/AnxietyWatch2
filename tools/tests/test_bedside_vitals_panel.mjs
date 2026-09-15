#!/usr/bin/env node
// Contract checks for the fixed bedside view and its source-precedence rules.
// Run with: node tools/tests/test_bedside_vitals_panel.mjs

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "../..");
const htmlPath = resolve(root, "docs/design/pi-panel/direction-a-clinical/bedside-vitals.html");
const html = readFileSync(htmlPath, "utf8");

assert.match(html, /--W:800px/);
assert.match(html, /--H:480px/);
for (const size of ["800x480", "1024x600", "1280x720"]) assert.ok(html.includes(size), `${size} review size must be offered`);
assert.match(html, /query\.get\("bp-check"\) === "1"/);
assert.doesNotMatch(html, /OPERATOR ACTION REQUIRED/);
assert.match(html, /CURRENT SIGNALS/);
assert.match(html, /FLOW = FRESH · GREY = STALE · GAP = NOT MEASURED/);
assert.match(html, /EMAY.*Wellue/);
assert.match(html, /CPAP WHEN ACTIVE|direct respiration/);
assert.match(html, /STALE/);
assert.match(html, /breath-visual/);
assert.doesNotMatch(html, /pap-pressure/);
assert.match(html, /No value is fabricated|no value is fabricated/i);

const script = html.split("<script>\n", 2)[1].split("\n</script>", 1)[0];
const helperEnd = script.indexOf("const model =");
assert.ok(helperEnd > 0, "source-selection helpers must be defined before rendering");
const elements = new Map();
const context = {
  URLSearchParams,
  location: { search: "" },
  Date,
  Math,
  Number,
  Object,
  Set,
  String,
  document: {
    documentElement: { style: { setProperty() {} } },
    body: { classList: { add() {} } },
    getElementById: id => elements.get(id),
  },
  window: {},
};
const run = new Function("context", `with (context) { ${script.slice(0, helperEnd)} }`);
run(context);
const { chooseOximeter, chooseRespiration, heartRate, cpapObject, breathState, metricState, oximeterIssue } = context.window.__bedsideTest;
const now = 1_700_000_100;
const live = sample => ({ ...sample, sample_at: now - 1, written_at: now - 1 });

const emay = live({ spo2: 96, pulse_bpm: 64 });
const wellue = live({ spo2: 95, pulse_bpm: 62 });
assert.equal(chooseOximeter(now, emay, wellue).name, "EMAY", "EMAY wins when both oximeters are present");
assert.equal(chooseOximeter(now, { ...emay, sample_at: now - 60, written_at: now - 60 }, wellue).name, "WELLUE", "stale EMAY yields to live Wellue");
assert.equal(chooseOximeter(now, null, wellue).name, "WELLUE", "Wellue is the fallback");
assert.equal(chooseOximeter(now, null, null), null, "no oximeter must remain unknown");

const cpap = cpapObject(live({ on: true, respiratory_rate: 13, ahi: 2.1, leak_lpm: 8, pressure_cmh2o: 10 }));
assert.equal(cpap.active, true);
assert.equal(chooseRespiration(now, { h10: live({ respiratory_rate: 18 }) }, cpap).name, "CPAP", "active CPAP wins for respiration");
assert.equal(chooseRespiration(now, { direct_respiration: live({ respiratory_rate: 14 }) }, null).name, "DIRECT SENSOR");
assert.equal(chooseRespiration(now, { h10: live({ respiratory_rate: 16 }) }, null).name, "H10 INFERRED");
assert.equal(chooseRespiration(now, {}, null), null, "absent respiration remains unknown");
assert.equal(breathState(now, cpap), "good", "active fresh CPAP gets live breathing state");
assert.equal(breathState(now, { ...cpap, active: false }), "warn", "inactive CPAP is stale, not live");
assert.equal(breathState(now, null), "bad", "absent CPAP has no live breathing state");
assert.equal(metricState({ value: 9, live: false }), "stale");
assert.equal(metricState({ value: 9, live: true }), "live");
assert.equal(metricState(null), "unknown");
assert.match(oximeterIssue({ emay: { link: "down" } }, { link: "lost" }), /EMAY DOWN.*Wellue LOST/);

assert.equal(heartRate(now, { h10: live({ hr: 58 }) }, chooseOximeter(now, emay, wellue)).name, "POLAR H10", "ECG HR wins over pulse-rate fallback");
assert.equal(heartRate(now, {}, chooseOximeter(now, emay, wellue)).name, "EMAY", "selected oximeter supplies fallback pulse");

console.log("PASS bedside-vitals dimensions, precedence, unknown-state, and BP-check contracts");
