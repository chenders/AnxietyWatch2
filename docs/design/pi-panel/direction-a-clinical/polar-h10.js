"use strict";

const RANGE_SECONDS = Object.freeze({ all: 0, "24h": 86_400, "30m": 1_800, "5m": 300 });
const MAX_AGE_SECONDS = 10;
const HRV_WINDOW_SECONDS = 60;
const HRV_MIN_BEATS = 30;
const GRAPH_BUCKETS = 620;
const GAP_SECONDS = 5;
const POLL_INTERVAL_MS = 1_000;
const HISTORY_URL = "vitals_h10_history.json";
const STATUS_URL = "vitals_status.json";

class SeriesStore {
  constructor() {
    this.points = [];
    this.keys = new Set();
    this.version = 0;
  }

  merge(incoming) {
    if (!Array.isArray(incoming) || incoming.length === 0) return false;
    let changed = false;
    let needsSort = false;
    let lastTime = this.points.length ? this.points[this.points.length - 1].t : -Infinity;
    for (const raw of incoming) {
      const t = Number(raw && raw.t);
      const v = Number(raw && raw.v);
      if (!Number.isFinite(t) || !Number.isFinite(v)) continue;
      const key = `${t}:${v}`;
      if (this.keys.has(key)) continue;
      if (t < lastTime) needsSort = true;
      this.points.push({ t, v });
      this.keys.add(key);
      lastTime = Math.max(lastTime, t);
      changed = true;
    }
    if (!changed) return false;
    if (needsSort) this.points.sort((a, b) => a.t - b.t || a.v - b.v);
    this.version += 1;
    return true;
  }
}

function lowerBound(points, timestamp) {
  let low = 0;
  let high = points.length;
  while (low < high) {
    const middle = low + Math.floor((high - low) / 2);
    if (points[middle].t < timestamp) low = middle + 1;
    else high = middle;
  }
  return low;
}

function windowed(points, range, nowSeconds) {
  const seconds = RANGE_SECONDS[range];
  if (!seconds) return points;
  return points.slice(lowerBound(points, nowSeconds - seconds));
}

/** Compute both 60-second HRV series in one O(n) sliding-window pass. */
function computeHrv(points) {
  const rmssd = [];
  const sdnn = [];
  let left = 0;
  let sum = 0;
  let sumSquares = 0;
  let sumDifferenceSquares = 0;

  for (let right = 0; right < points.length; right += 1) {
    const value = points[right].v;
    sum += value;
    sumSquares += value * value;
    if (right > left) {
      const difference = value - points[right - 1].v;
      sumDifferenceSquares += difference * difference;
    }

    const cutoff = points[right].t - HRV_WINDOW_SECONDS;
    while (left <= right && points[left].t < cutoff) {
      if (left < right) {
        const difference = points[left + 1].v - points[left].v;
        sumDifferenceSquares -= difference * difference;
      }
      const removed = points[left].v;
      sum -= removed;
      sumSquares -= removed * removed;
      left += 1;
    }

    const count = right - left + 1;
    if (count < HRV_MIN_BEATS) continue;
    const varianceNumerator = Math.max(0, sumSquares - (sum * sum) / count);
    // RR is already in its registry-canonical seconds. RMSSD is a distinct
    // derived type whose canonical unit is milliseconds, so convert once when
    // that derived value is created; SDNN uses the same conventional unit.
    rmssd.push({
      t: points[right].t,
      v: Math.sqrt(Math.max(0, sumDifferenceSquares) / (count - 1)) * 1000,
    });
    sdnn.push({
      t: points[right].t,
      v: Math.sqrt(varianceNumerator / (count - 1)) * 1000,
    });
  }

  return { rmssd, sdnn };
}

function timeBounds(series) {
  let low = Infinity;
  let high = -Infinity;
  for (const points of Object.values(series)) {
    if (!points.length) continue;
    low = Math.min(low, points[0].t);
    high = Math.max(high, points[points.length - 1].t);
  }
  return Number.isFinite(low) ? [low, high] : [0, 1];
}

function valueBounds(points) {
  let low = Infinity;
  let high = -Infinity;
  for (const point of points) {
    low = Math.min(low, point.v);
    high = Math.max(high, point.v);
  }
  return [low, high];
}

function selectBucketExtrema(points, start, end) {
  if (end - start <= 4) {
    return Array.from({ length: end - start }, (_, offset) => start + offset);
  }
  let minIndex = start;
  let maxIndex = start;
  for (let index = start + 1; index < end; index += 1) {
    if (points[index].v < points[minIndex].v) minIndex = index;
    if (points[index].v > points[maxIndex].v) maxIndex = index;
  }
  return [...new Set([start, minIndex, maxIndex, end - 1])].sort((a, b) => a - b);
}

/**
 * Keep first/min/max/last for each horizontal-pixel bucket. Runs separated by
 * missing time remain separate, so downsampling never invents a line over a gap.
 */
function downsampleSeries(points, timeLow, timeHigh, buckets = GRAPH_BUCKETS) {
  if (!points.length) return [];
  const span = Math.max(Number.EPSILON, timeHigh - timeLow);
  const sampled = [];
  let runStart = 0;

  const appendRun = (start, end) => {
    let bucketStart = start;
    while (bucketStart < end) {
      const bucket = Math.min(
        buckets - 1,
        Math.max(0, Math.floor(((points[bucketStart].t - timeLow) / span) * buckets)),
      );
      let bucketEnd = bucketStart + 1;
      while (bucketEnd < end) {
        const candidate = Math.min(
          buckets - 1,
          Math.max(0, Math.floor(((points[bucketEnd].t - timeLow) / span) * buckets)),
        );
        if (candidate !== bucket) break;
        bucketEnd += 1;
      }
      for (const index of selectBucketExtrema(points, bucketStart, bucketEnd)) {
        sampled.push({ ...points[index], start: sampled.length === 0 || index === start });
      }
      bucketStart = bucketEnd;
    }
  };

  for (let index = 1; index <= points.length; index += 1) {
    if (index === points.length || points[index].t - points[index - 1].t > GAP_SECONDS) {
      appendRun(runStart, index);
      runStart = index;
    }
  }
  return sampled;
}

function nearest(points, timestamp) {
  if (!points.length) return null;
  const after = lowerBound(points, timestamp);
  if (after === 0) return points[0];
  if (after === points.length) return points[points.length - 1];
  const beforePoint = points[after - 1];
  const afterPoint = points[after];
  return timestamp - beforePoint.t <= afterPoint.t - timestamp ? beforePoint : afterPoint;
}

function nearestAcrossSeries(series, timestamp) {
  let best = null;
  for (const points of Object.values(series)) {
    const candidate = nearest(points, timestamp);
    if (
      candidate
      && (!best || Math.abs(candidate.t - timestamp) < Math.abs(best.t - timestamp))
    ) {
      best = candidate;
    }
  }
  return best;
}

function buildGraphModel(hrPoints, rrPoints, range, nowSeconds) {
  const hr = windowed(hrPoints, range, nowSeconds);
  const rr = windowed(rrPoints, range, nowSeconds);
  const hrv = computeHrv(rr);
  const series = { hr, rr, rmssd: hrv.rmssd, sdnn: hrv.sdnn };
  return { range, series, bounds: timeBounds(series) };
}

const stores = { hr: new SeriesStore(), rr: new SeriesStore() };
let selectedRange = "30m";
let graphModel = null;
let graphSignature = "";
let graphFrame = 0;
let hoverFrame = 0;
let pendingHover = null;
let historyRevision = null;
let historyRequest = null;
let rrUnitBlocker = null;
let chartRects = new WeakMap();

function currentSignature(nowSeconds) {
  const seconds = RANGE_SECONDS[selectedRange];
  const cutoff = seconds ? nowSeconds - seconds : -Infinity;
  return [
    selectedRange,
    stores.hr.version,
    stores.rr.version,
    lowerBound(stores.hr.points, cutoff),
    lowerBound(stores.rr.points, cutoff),
  ].join(":");
}

function ensureGraphModel(nowSeconds = Date.now() / 1000) {
  const signature = currentSignature(nowSeconds);
  if (graphModel && signature === graphSignature) return graphModel;
  graphModel = buildGraphModel(
    stores.hr.points,
    stores.rr.points,
    selectedRange,
    nowSeconds,
  );
  graphSignature = signature;
  return graphModel;
}

function pathData(points, bounds, colorFor) {
  const [timeLow, timeHigh] = bounds;
  const timeSpan = Math.max(Number.EPSILON, timeHigh - timeLow);
  const [valueLow, valueHigh] = valueBounds(points);
  const valueSpan = Math.max(Number.EPSILON, valueHigh - valueLow);
  const sampled = downsampleSeries(points, timeLow, timeHigh);
  const paths = new Map();
  let previous = null;
  let previousColor = null;

  const xy = (point) => ({
    x: 110 + ((point.t - timeLow) / timeSpan) * 620,
    y: 165 - ((point.v - valueLow) / valueSpan) * 140,
  });

  for (const point of sampled) {
    if (point.start) {
      previous = point;
      previousColor = null;
      continue;
    }
    const color = colorFor(point.v);
    const from = xy(previous);
    const to = xy(point);
    const prefix = color === previousColor ? "" : `M${from.x.toFixed(1)} ${from.y.toFixed(1)}`;
    paths.set(
      color,
      `${paths.get(color) || ""}${prefix}L${to.x.toFixed(1)} ${to.y.toFixed(1)}`,
    );
    previous = point;
    previousColor = color;
  }
  return { paths, valueLow, valueHigh };
}

function setText(id, text) {
  const element = document.querySelector(`#${id}`);
  if (element.textContent !== text) element.textContent = text;
}

function drawSeries(id, points, colorFor, ticksId, bounds) {
  const group = document.querySelector(`#${id}`);
  const ticks = document.querySelector(`#${ticksId}`);
  const waiting = group.parentElement.querySelector("text.axis");
  if (points.length < 2) {
    for (const path of group.querySelectorAll("path")) path.setAttribute("d", "");
    ticks.replaceChildren();
    waiting.style.display = "block";
    return;
  }

  waiting.style.display = "none";
  const rendered = pathData(points, bounds, colorFor);
  for (const path of group.querySelectorAll("path")) {
    path.setAttribute("d", rendered.paths.get(path.dataset.color) || "");
  }

  const [timeLow, timeHigh] = bounds;
  const formatValue = (value) => (Math.abs(value) >= 100 ? Math.round(value) : value.toFixed(1));
  const formatTime = (timestamp) => new Date(timestamp * 1000).toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  });
  ticks.innerHTML = [0, 0.5, 1].map((fraction) => (
    `<text x="34" y="${(165 - fraction * 140).toFixed(1)}">${formatValue(rendered.valueLow + fraction * (rendered.valueHigh - rendered.valueLow))}</text>`
    + `<text text-anchor="middle" x="${(110 + fraction * 620).toFixed(1)}" y="205">${formatTime(timeLow + fraction * (timeHigh - timeLow))}</text>`
  )).join("");
}

function drawGraphs() {
  const model = ensureGraphModel();
  drawSeries("hrWave", model.series.hr, (value) => (
    value >= 60 && value <= 100 ? "good" : "caution"
  ), "hrTicks", model.bounds);
  drawSeries("rrWave", model.series.rr, (value) => (
    value >= 0.2 && value <= 3.0 ? "timing" : "invalid"
  ), "rrTicks", model.bounds);
  drawSeries("rmssdWave", model.series.rmssd, () => "neutral", "rmssdTicks", model.bounds);
  drawSeries("sdnnWave", model.series.sdnn, () => "neutral", "sdnnTicks", model.bounds);
}

function scheduleGraphDraw() {
  if (graphFrame) return;
  graphFrame = requestAnimationFrame(() => {
    graphFrame = 0;
    drawGraphs();
  });
}

const hoverCharts = [
  ["hr", "Heart rate", "bpm", "hrCrosshair"],
  ["rr", "RR interval", "s", "rrCrosshair"],
  ["rmssd", "RMSSD", "ms", "rmssdCrosshair"],
  ["sdnn", "SDNN", "ms", "sdnnCrosshair"],
];

function showHover(timestamp, clientX, clientY) {
  const model = ensureGraphModel();
  const snap = nearestAcrossSeries(model.series, timestamp);
  if (!snap) return;
  const [low, high] = model.bounds;
  const x = high > low ? 110 + ((snap.t - low) / (high - low)) * 620 : 400;
  for (const line of document.querySelectorAll(".crosshair")) {
    line.style.display = "block";
    line.setAttribute("x1", x);
    line.setAttribute("x2", x);
  }

  let rows = "";
  for (const [key, label, unit] of hoverCharts) {
    const point = nearest(model.series[key], snap.t);
    rows += point
      ? `<div class="tip-row"><span>${label}</span><span>${point.v.toFixed(2)} ${unit} <small style="display:inline">(${point.t - snap.t >= 0 ? "+" : ""}${(point.t - snap.t).toFixed(2)}s)</small></span></div>`
      : `<div class="tip-row"><span>${label}</span><span>not measured</span></div>`;
  }
  const tip = document.querySelector("#hoverTip");
  tip.innerHTML = `<strong>Shared time · ${new Date(snap.t * 1000).toLocaleTimeString()}</strong>${rows}<small>Values are exact recorded/calculated points; offsets show when a series’ nearest point occurred. No interpolation is invented.</small>`;
  tip.hidden = false;
  tip.style.left = `${Math.min(clientX + 14, window.innerWidth - 345)}px`;
  tip.style.top = `${Math.min(clientY + 14, window.innerHeight - 190)}px`;
}

function hideHover() {
  pendingHover = null;
  if (hoverFrame) cancelAnimationFrame(hoverFrame);
  hoverFrame = 0;
  document.querySelector("#hoverTip").hidden = true;
  for (const line of document.querySelectorAll(".crosshair")) line.style.display = "none";
}

function queueHover(chart, clientX, clientY) {
  let rect = chartRects.get(chart);
  if (!rect) {
    rect = chart.getBoundingClientRect();
    chartRects.set(chart, rect);
  }
  const fraction = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  const model = ensureGraphModel();
  const [low, high] = model.bounds;
  pendingHover = { timestamp: low + fraction * (high - low), clientX, clientY };
  if (hoverFrame) return;
  hoverFrame = requestAnimationFrame(() => {
    hoverFrame = 0;
    const pending = pendingHover;
    pendingHover = null;
    if (pending) showHover(pending.timestamp, pending.clientX, pending.clientY);
  });
}

function installHover() {
  for (const chart of document.querySelectorAll(".chart[data-series]")) {
    chart.addEventListener("mouseenter", () => chartRects.set(chart, chart.getBoundingClientRect()));
    chart.addEventListener("mousemove", (event) => queueHover(chart, event.clientX, event.clientY));
    chart.addEventListener("mouseleave", hideHover);
    chart.addEventListener("touchstart", (event) => {
      const touch = event.touches[0];
      if (touch) queueHover(chart, touch.clientX, touch.clientY);
    }, { passive: true });
  }
  window.addEventListener("resize", () => {
    chartRects = new WeakMap();
  });
}

function mergePayload(data) {
  if (!data) return false;
  const incoming = data.h10_delta || data.h10_history || {};
  let changed = stores.hr.merge(incoming.hr);
  const rrUnit = data.units && data.units.rr;
  const rrIsCanonical = rrUnit === "seconds";
  if (rrIsCanonical) {
    changed = stores.rr.merge(incoming.rr) || changed;
    rrUnitBlocker = null;
  } else if (Array.isArray(incoming.rr) && incoming.rr.length) {
    const blocker = rrUnit || "missing";
    if (rrUnitBlocker !== blocker) console.warn(`RR data blocked: units.rr=${blocker}`);
    rrUnitBlocker = blocker;
  }
  const h10 = data.h10;
  if (h10 && h10.sample_at != null && h10.hr != null) {
    changed = stores.hr.merge([{ t: h10.sample_at, v: h10.hr }]) || changed;
  }
  if (rrIsCanonical && h10 && Array.isArray(h10.rr_points)) {
    changed = stores.rr.merge(h10.rr_points) || changed;
  }
  return changed;
}

function maximumGap(points) {
  let result = 0;
  for (let index = 1; index < points.length; index += 1) {
    result = Math.max(result, points[index].t - points[index - 1].t);
  }
  return result;
}

function renderStatus(data, error = null) {
  const h10 = data && data.h10;
  const now = Date.now() / 1000;
  const age = h10 ? Math.max(0, now - Number(h10.sample_at)) : Infinity;
  const live = Boolean(
    h10 && age <= MAX_AGE_SECONDS && h10.test_origin === "physical_device",
  );
  const known = Boolean(h10);
  const visibleHeartRate = windowed(stores.hr.points, selectedRange, now);
  const maxGap = maximumGap(visibleHeartRate);
  const banner = document.querySelector("#graphState");
  banner.className = `signal-banner ${!live || maxGap > 10 ? "lost" : ""}`;
  banner.textContent = live
    ? (maxGap > 10
      ? `Live signal current; selected window contains a ${Math.round(maxGap)} s gap marked not measured.`
      : "Live signal current; lines show received measurements only.")
    : (h10
      ? `No live H10 signal for ${Math.round(age)} s; last value is historical.`
      : "No H10 measurement received yet.");

  const dot = document.querySelector("#dot");
  dot.className = `dot ${live ? "live" : known ? "warn" : ""}`;
  setText("state", live ? "Receiving valid H10 data" : known ? "Last H10 data is stale" : "No accepted H10 sample");
  const origin = document.querySelector("#origin");
  origin.textContent = h10
    ? (h10.test_origin === "physical_device" ? "PHYSICAL" : h10.test_origin === "polar_h10_emulator" ? "EMULATOR" : "UNVERIFIED")
    : "NO DATA";
  origin.className = `pill ${live ? "" : "warn"}`;
  setText("hr", h10 && h10.hr != null ? Math.round(Number(h10.hr)) : "—");
  setText("age", h10
    ? `Sample age ${age.toFixed(1)} s · ${live ? "current live view" : "historical/stale view"}`
    : "No accepted physiological sample yet.");

  const bpm = h10 && h10.hr != null ? Number(h10.hr) : NaN;
  const hrMeaning = document.querySelector("#hrMeaning");
  hrMeaning.className = `meaning ${Number.isFinite(bpm) ? (bpm >= 60 && bpm <= 100 ? "good" : "caution") : "unusable"}`;
  hrMeaning.innerHTML = Number.isFinite(bpm)
    ? `<strong>At rest:</strong> ${bpm >= 60 && bpm <= 100 ? "within the common adult resting reference of 60–100 bpm." : bpm < 60 ? "below the common adult resting reference; this may be normal during sleep, training, or with some medicines." : "above the common adult resting reference; activity, stress, caffeine, illness, and medicines can all affect it."} <span class="meta">Reference only—not a diagnosis.</span>`
    : "<strong>Reference:</strong> waiting for a heart-rate value.";

  const rrCount = h10 && Number.isFinite(Number(h10.rr_count)) ? Number(h10.rr_count) : null;
  setText("rr", rrCount == null ? "—" : rrCount);
  const latestRrPoint = stores.rr.points.length ? stores.rr.points[stores.rr.points.length - 1] : null;
  const latestRr = h10 && Array.isArray(h10.rr_points) && h10.rr_points.length
    ? Number(h10.rr_points[h10.rr_points.length - 1].v)
    : latestRrPoint ? latestRrPoint.v : NaN;
  const rrMeaning = document.querySelector("#rrMeaning");
  rrMeaning.className = `meaning ${Number.isFinite(latestRr) && latestRr >= 0.2 && latestRr <= 3.0 ? "good" : "unusable"}`;
  rrMeaning.innerHTML = Number.isFinite(latestRr)
    ? `<strong>Signal quality:</strong> ${latestRr >= 0.2 && latestRr <= 3.0 ? "accepted by the system’s 0.2–3.0 s filter." : "outside the system’s accepted filter."} <span class="meta">Not a clinical normal range.</span>`
    : "No interval to interpret.";

  setText("session", h10 ? h10.session_id : "—");
  setText("interpretation", live ? "The strap is sensing and sending." : "Connection is not the same as a valid measurement.");
  const packetDetail = rrCount == null
    ? "Heart rate arrived; beat timing is shown when accepted RR intervals are available."
    : rrCount > 0
      ? "This packet includes beat timing. A longer window is needed before interpreting HRV."
      : "Heart rate arrived, but this packet contains no RR intervals.";
  setText("detail", error
    ? `The local status feed could not be read: ${error}.`
    : rrUnitBlocker
      ? `RR data blocked: units.rr=${rrUnitBlocker}; canonical seconds are required.`
      : live ? packetDetail : "The page will not turn silence into a reassuring number.");
  setText("observed", live
    ? `${Math.round(Number(h10.hr))} bpm observed${rrCount == null ? "." : `; ${rrCount} RR interval${rrCount === 1 ? "" : "s"} in the latest packet.`}`
    : "No current physical H10 observation.");
  setText("updated", `Last page update: ${new Date().toLocaleTimeString()}`);
  setText("source", `Source: ${live ? "physical Polar H10 via local pipeline" : "local H10 status feed"}`);
}

async function fetchJson(url) {
  const separator = url.includes("?") ? "&" : "?";
  const response = await fetch(`${url}${separator}${Date.now()}`, { cache: "no-store" });
  if (!response.ok) throw new Error(`status ${response.status}`);
  return response.json();
}

async function syncHistory() {
  if (historyRequest) return historyRequest;
  historyRequest = (async () => {
    try {
      const data = await fetchJson(HISTORY_URL);
      const changed = mergePayload(data);
      if (Number.isFinite(Number(data.revision))) historyRevision = Number(data.revision);
      if (changed) scheduleGraphDraw();
    } catch (error) {
      // Compatibility with the previous publisher: its status payload contains
      // history. Polling below still works, but a current publisher keeps this
      // one-time failure off the operator-facing physiological status.
      console.warn(`H10 history sync unavailable: ${error.message}`);
    } finally {
      historyRequest = null;
    }
  })();
  return historyRequest;
}

async function poll() {
  try {
    const data = await fetchJson(STATUS_URL);
    const changed = mergePayload(data);
    const statusRevision = Number(data.history_revision);
    if (
      Number.isFinite(statusRevision)
      && (historyRevision === null || statusRevision > historyRevision)
    ) {
      void syncHistory();
    }
    if (changed || currentSignature(Date.now() / 1000) !== graphSignature) {
      scheduleGraphDraw();
    }
    renderStatus(data, data.error || null);
  } catch (error) {
    renderStatus(null, error.message);
  } finally {
    window.setTimeout(poll, POLL_INTERVAL_MS);
  }
}

function applyTheme(deep) {
  document.documentElement.classList.toggle("deep-dark", deep);
  const button = document.querySelector("#theme");
  button.textContent = deep ? "Use standard dark theme" : "Use darker theme";
  button.setAttribute("aria-pressed", String(deep));
}

function start() {
  document.querySelector("#range").addEventListener("change", (event) => {
    selectedRange = event.target.value;
    graphSignature = "";
    hideHover();
    scheduleGraphDraw();
  });
  const themeButton = document.querySelector("#theme");
  applyTheme(localStorage.getItem("h10-theme") === "deep");
  themeButton.addEventListener("click", () => {
    const deep = !document.documentElement.classList.contains("deep-dark");
    localStorage.setItem("h10-theme", deep ? "deep" : "standard");
    applyTheme(deep);
  });
  installHover();
  scheduleGraphDraw();
  void syncHistory().finally(poll);
}

if (typeof document !== "undefined") start();

if (typeof module !== "undefined") {
  module.exports = {
    SeriesStore,
    buildGraphModel,
    computeHrv,
    downsampleSeries,
    lowerBound,
    nearest,
    nearestAcrossSeries,
    pathData,
    timeBounds,
    valueBounds,
    windowed,
  };
}
