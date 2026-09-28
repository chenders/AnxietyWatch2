import assert from "node:assert/strict";
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";

const require = createRequire(import.meta.url);
const directory = path.dirname(fileURLToPath(import.meta.url));
const helpers = require(path.join(directory, "polar-h10.js"));
const {
  SeriesStore,
  buildGraphModel,
  computeHrv,
  downsampleSeries,
  mergePayload,
  nearest,
  pathData,
  windowed,
} = helpers;

function close(actual, expected, tolerance = 1e-9) {
  assert.ok(
    Math.abs(actual - expected) < tolerance,
    `expected ${actual} to be within ${tolerance} of ${expected}`,
  );
}

function naiveHrv(points) {
  const rmssd = [];
  const sdnn = [];
  for (let index = 0; index < points.length; index += 1) {
    const current = points[index];
    const values = points
      .filter((point) => point.t >= current.t - 60 && point.t <= current.t)
      .map((point) => point.v);
    if (values.length < 30) continue;
    const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
    const differenceSquares = values.slice(1).map((value, differenceIndex) => (
      (value - values[differenceIndex]) ** 2
    ));
    rmssd.push({
      t: current.t,
      v: Math.sqrt(
        differenceSquares.reduce((sum, value) => sum + value, 0)
          / differenceSquares.length,
      ) * 1000,
    });
    sdnn.push({
      t: current.t,
      v: Math.sqrt(
        values.reduce((sum, value) => sum + (value - mean) ** 2, 0)
          / (values.length - 1),
      ) * 1000,
    });
  }
  return { rmssd, sdnn };
}

const referencePoints = Array.from({ length: 180 }, (_, index) => ({
  t: 1_700_000_000 + index * 0.83,
  v: 0.780 + Math.sin(index / 7) * 0.035 + (index % 5) / 1000,
}));
const expected = naiveHrv(referencePoints);
const actual = computeHrv(referencePoints);
assert.equal(actual.rmssd.length, expected.rmssd.length);
assert.equal(actual.sdnn.length, expected.sdnn.length);
for (let index = 0; index < actual.rmssd.length; index += 1) {
  assert.equal(actual.rmssd[index].t, expected.rmssd[index].t);
  close(actual.rmssd[index].v, expected.rmssd[index].v, 1e-8);
  close(actual.sdnn[index].v, expected.sdnn[index].v, 1e-8);
}
console.log("PASS: linear HRV matches the previous rolling-window formulas");

const largePoints = Array.from({ length: 100_000 }, (_, index) => ({
  t: 1_700_000_000 + index * 0.8,
  v: 0.800 + (index % 41) / 1000,
}));
const started = performance.now();
const largeHrv = computeHrv(largePoints);
const elapsed = performance.now() - started;
assert.equal(largeHrv.rmssd.length, 100_000 - 29);
assert.ok(elapsed < 2_000, `100k-point HRV pass took ${elapsed.toFixed(1)} ms`);
console.log(`PASS: 100k-point HRV pass is linear (${elapsed.toFixed(1)} ms)`);

const store = new SeriesStore();
assert.equal(store.merge([{ t: 2, v: 20 }, { t: 1, v: 10 }, { t: 2, v: 20 }]), true);
assert.deepEqual(store.points, [{ t: 1, v: 10 }, { t: 2, v: 20 }]);
assert.equal(store.merge([{ t: 2, v: 20 }]), false);
console.log("PASS: incremental merge deduplicates and sorts out-of-order arrivals");

assert.equal(
  mergePayload({
    units: { rr: "seconds" },
    h10: {
      sample_at: 1_700_000_500,
      hr: 72,
      test_origin: "polar_h10_emulator",
      rr_points: [{ t: 1_700_000_500, v: 0.8 }],
    },
  }),
  false,
  "non-physical current samples must not enter physical graph stores",
);
assert.equal(
  mergePayload({
    units: { rr: "seconds" },
    h10: {
      sample_at: 1_700_000_501,
      hr: 73,
      test_origin: "physical_device",
      rr_points: [{ t: 1_700_000_501, v: 0.81 }],
    },
  }),
  true,
  "physical current samples must enter graph stores",
);
console.log("PASS: graph stores reject non-physical current samples");

const dense = Array.from({ length: 50_000 }, (_, index) => ({
  t: index < 25_000 ? index * 0.5 : 20_000 + (index - 25_000) * 0.5,
  v: index % 997 === 0 ? 3.0 : 0.700 + (index % 80) / 1000,
}));
const sampled = downsampleSeries(dense, dense[0].t, dense[dense.length - 1].t, 620);
assert.ok(sampled.length <= 4 * 620 * 2, `downsample retained ${sampled.length} points`);
assert.equal(sampled.filter((point) => point.start).length, 2, "the real gap must remain two runs");
assert.ok(
  sampled.some((point) => Math.abs(point.v - 3.0) < 1e-9),
  "per-pixel maxima must survive",
);
const rendered = pathData(dense, [dense[0].t, dense[dense.length - 1].t], () => "timing");
assert.ok(rendered.paths.get("timing").length < 150_000, "render path must not scale with history");
console.log("PASS: pixel downsampling retains extrema and never bridges gaps");

assert.equal(nearest(referencePoints, referencePoints[50].t + 0.1), referencePoints[50]);
const recent = windowed(referencePoints, "5m", referencePoints.at(-1).t);
assert.equal(recent.length, referencePoints.length);
const model = buildGraphModel(referencePoints, referencePoints, "30m", referencePoints.at(-1).t);
assert.ok(model.series.rmssd.length > 0 && model.series.sdnn.length > 0);
console.log("PASS: cached-model primitives support binary hover lookup and all graph series");

const html = fs.readFileSync(path.join(directory, "polar-h10.html"), "utf8");
const source = fs.readFileSync(path.join(directory, "polar-h10.js"), "utf8");
assert.ok(html.includes('<script src="polar-h10.js"></script>'));
assert.equal((html.match(/<stop\b/g) || []).length, 0, "samples must not become SVG stop nodes");
assert.equal(source.includes("setInterval("), false, "polls must not overlap or backlog");
assert.ok(source.includes("requestAnimationFrame"), "hover and graph updates must be frame-throttled");
assert.ok(source.includes("h10_delta"), "the page must consume incremental history");
assert.ok(
  source.includes('rrUnit === "seconds"'),
  "ambiguous or non-canonical RR units must fail closed",
);
console.log("PASS: page has bounded SVG DOM, non-overlapping polls, and incremental history");
