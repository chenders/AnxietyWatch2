#!/usr/bin/env node
// Focused checks for the live-vitals page's responsive health messages and
// EMAY no-finger state. Run with: node tools/tests/test_live_vitals_panel.mjs

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const htmlPath = resolve(root, "docs/design/pi-panel/direction-a-clinical/live-vitals.html");
const html = readFileSync(htmlPath, "utf8");

for (const selector of [".reason", ".health-name", ".health-detail"]) {
  const escaped = selector.replace(".", "\\.");
  const match = html.match(new RegExp(`${escaped}\\s*\\{([\\s\\S]*?)\\}`));
  assert.ok(match, `${selector} CSS block must exist`);
  assert.match(match[1], /white-space:\s*normal/);
  assert.match(match[1], /overflow:\s*visible/);
  assert.doesNotMatch(match[1], /text-overflow:\s*ellipsis/);
}
assert.match(html, /min-height:\s*174px/);
assert.match(html, /Place the sensor on a finger\./);
assert.match(html, /the last frame reported no finger/);
assert.match(html, /PERSISTED \+ LIVE · LAST 10 MIN · GAPS PRESERVED/);
assert.match(
  html,
  /scale: \{ min: 75, max: 100, step: 5, ticks: \[75, 80, 85, 90, 95, 100\] \}/,
);
assert.match(html, /emay_history/);
assert.match(html, /emay_delta/);

const script = html.split("<script>\n", 2)[1].split("\n</script>", 1)[0];
const beforeStartup = script.slice(0, script.lastIndexOf("\nrender();"));
const elements = new Map();
function makeElement() {
  return { className: "", textContent: "", innerHTML: "", classList: { toggle() {} } };
}
for (const id of ["emay-card", "emay-badge", "emay-reason", "emay-spo2", "emay-pulse", "emay-frames", "emay-age"])
  elements.set(id, makeElement());
const context = {
  URLSearchParams,
  location: { search: "" },
  document: { getElementById: id => elements.get(id) },
  console,
  Date,
  Math,
  Number,
  Set,
  Object,
  String,
};
context.globalThis = context;
const run = new Function("context", `with (context) { ${beforeStartup}; context.renderEmay = renderEmay; }`);
run(context);

const now = 1_700_000_100;
const base = {
  schema: 1, synthetic: false, link: "up", spo2: null, pulse_bpm: null,
  frames: 10, measurements: 5, written_at: now, sample_at: now - 120,
  finger: false,
};
context.renderEmay(base, now, true);
assert.equal(elements.get("emay-badge").textContent, "STALE");
assert.match(elements.get("emay-reason").textContent, /last frame reported no finger/);
assert.match(elements.get("emay-reason").textContent, /Place the sensor on a finger/);
assert.match(elements.get("emay-age").innerHTML, /^LAST SENSOR FRAME/);

context.renderEmay({ ...base, sample_at: now - 1 }, now, true);
assert.equal(elements.get("emay-badge").textContent, "NO FINGER");
assert.equal(elements.get("emay-reason").textContent, "The sensor reports no finger. Place the sensor on a finger.");

context.renderEmay({ ...base, sample_at: null }, now, true);
assert.doesNotMatch(elements.get("emay-reason").textContent, /finger/);
assert.match(elements.get("emay-reason").textContent, /no sensor frame has arrived yet/);

context.renderEmay({ ...base, sample_at: now - 1 }, now, false);
assert.match(elements.get("emay-reason").textContent, /Cannot verify EMAY currently/);
assert.match(elements.get("emay-reason").textContent, /last frame reported no finger/);

context.renderEmay({
  stored: true, synthetic: false, spo2: 97, pulse_bpm: 70,
  sample_at: now - 2, session_id: 123, seq: 4,
}, now, true);
assert.equal(elements.get("emay-badge").textContent, "HISTORY");
assert.equal(
  elements.get("emay-reason").textContent,
  "Latest persisted EMAY value; live sensor state is not claimed.",
);
assert.match(elements.get("emay-age").innerHTML, /^LAST PERSISTED SAMPLE/);

console.log("PASS live-vitals responsive/no-finger/history checks");
