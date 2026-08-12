/* Live-track aircraft proximity advisory for the stopgap kiosk.
 * Display only: this is not a measured sound forecast and is never armed. */
(() => {
  "use strict";

  const originalDrawRadar = window.drawRadar;
  const originalShowAircraftModal = window.showAircraftModal;
  if (typeof originalDrawRadar !== "function") return;

  /* Colour follows the estimate, and UNKNOWN gets the hue-free slate the panel
     already uses for "present but not judged" rather than a reassuring green. */
  const BAND_COLOUR = {
    "VERY LOUD": "var(--klaxon)",
    "LOUD": "var(--klaxon)",
    "AUDIBLE": "var(--watch)",
    "UNKNOWN": "var(--unknown-hi)",
  };

  /* Counted down from an absolute instant, not from a stored duration, so a
     stalled feed runs to NOW and stops instead of promising a flyover forever. */
  function countdown(atEpoch) {
    if (!Number.isFinite(atEpoch)) return "--";
    const left = Math.round(atEpoch - Date.now() / 1000);
    if (left <= 0) return "NOW";
    if (left < 60) return `${left}s`;
    return `${Math.floor(left / 60)}m ${String(left % 60).padStart(2, "0")}s`;
  }

  function loudnessCell(p) {
    const band = p.loudness_band || "UNKNOWN";
    const colour = BAND_COLOUR[band] || "var(--unknown-hi)";
    const index = Number.isFinite(p.noise_index) ? ` ${p.noise_index.toFixed(0)}` : "";
    return `<span style="color:${colour}">${esc(band)}${index}</span>`;
  }

  /* Forward-looking: what is about to be loud here, and when. Replaces the
     30-minute history, which recorded what had already happened and so could
     not warn anyone about anything. */
  function upcomingMarkup(snap) {
    const rows = (snap && snap.adsb_predictions) || [];
    const header = "<div style='grid-column:1/-1;color:var(--spo2);letter-spacing:1px;"
      + "border-bottom:1px solid #24465A;padding-bottom:4px;margin-bottom:4px'>"
      + "EXPECTED NOISE · NOT ARMED · ESTIMATED, NEVER MEASURED</div>";
    if (!rows.length) {
      return header + "<div style='grid-column:1/-1;color:var(--unknown-hi);padding:14px;"
        + "text-align:center'>NOTHING EXPECTED IN THE NEXT 3 MIN</div>";
    }
    const body = rows.slice(0, 6).map(r => {
      const p = r.prediction || {};
      const when = countdown(p.at_epoch);
      const urgent = when === "NOW";
      const where = Number.isFinite(p.ground_distance_m)
        ? `${(p.ground_distance_m / 1000).toFixed(1)}km` : "--";
      const alt = Number.isFinite(p.altitude_ft)
        ? `${Math.round(p.altitude_ft).toLocaleString()}ft` : "--";
      const edge = BAND_COLOUR[p.loudness_band] || "var(--unknown-hi)";
      const style = "grid-column:1/-1;display:grid;"
        + "grid-template-columns:76px 92px 62px 96px 1fr;gap:6px;padding:3px 4px;"
        + `border-left:3px solid ${edge};background:#0B1924;margin-bottom:3px`;
      return `<div style="${style}">`
        + `<span style="color:var(--nibp)">${esc(r.callsign || r.hex)}</span>`
        + loudnessCell(p)
        + `<b style="color:${urgent ? "var(--klaxon)" : "var(--label)"}">${esc(when)}</b>`
        + `<span style="color:var(--unknown)">${where} ${alt}</span>`
        + `<span style="color:var(--mute)">${esc(p.phase || p.proximity || "")}</span>`
        + `</div>`;
    }).join("");
    return header + body;
  }

  function advisoryMarkup(advisory) {
    /* Unknown is a third state. An aircraft we could not score is never folded
       into "nothing expected" -- that would let a missing emitter category read
       as reassurance. It is counted here, in its own colour, always. */
    const unscored = (advisory && advisory.unscored) || [];
    const unknownLine = unscored.length
      ? `<br><span style="color:var(--unknown-hi)">${unscored.length} FURTHER AIRCRAFT NOT SCORED — ${esc(unscored[0].reason)}</span>`
      : "";
    return `<div style="grid-column:1/-1;color:var(--unknown-hi);margin-bottom:5px;`
      + `border:1px solid #52606A;padding:4px;letter-spacing:.5px;font-size:11px">`
      + `NOTHING IS EVALUATING THESE · NO ALARM CAN FIRE${unknownLine}</div>`;
  }

  /* The radar canvas hardcodes a "NNdB" suffix on every blip. The number behind
     it is a relative index against an unvalidated reference, not a sound
     pressure level, and labelling it dB is the single most misleading thing on
     the screen. The canvas is drawn imperatively, so the only way to correct it
     without forking the whole radar is to intercept fillText for the duration
     of the call. Restored in a finally so a throw cannot leave it patched. */
  const ctxProto = CanvasRenderingContext2D.prototype;
  const originalFillText = ctxProto.fillText;
  function withHonestLabels(draw) {
    ctxProto.fillText = function (text, x, y, maxWidth) {
      const fixed = typeof text === "string" ? text.replace(/\s*(-?\d+)dB\b/, " ·$1") : text;
      return maxWidth === undefined
        ? originalFillText.call(this, fixed, x, y)
        : originalFillText.call(this, fixed, x, y, maxWidth);
    };
    try { draw(); } finally { ctxProto.fillText = originalFillText; }
  }

  window.drawRadar = function drawRadarWithAdvisory(planes, topNoise) {
    withHonestLabels(() => originalDrawRadar(planes, topNoise));
    const list = document.getElementById("live-radar-list");
    if (!list) return;
    /* Reached by bare name, not through `window`. The page declares `S` and
       `live` as top-level `const`, which binds them in the global lexical
       environment and NOT as properties of window -- so `window.live` is
       forever undefined and every advisory silently rendered empty. */
    const snap = (typeof S !== "undefined" && S.live
                  && typeof live !== "undefined" && live.snap) ? live.snap : null;
    const advisory = snap ? snap.adsb_advisory : null;
    /* The base radar wrote a 30-minute history here. Replace it outright
       rather than appending: the panel has one list-sized hole and a forecast
       is worth more in it than a record of what already flew over. */
    list.innerHTML = upcomingMarkup(snap) + advisoryMarkup(advisory);
  };

  if (typeof originalShowAircraftModal === "function") {
    window.showAircraftModal = function showAircraftPrediction(p) {
      originalShowAircraftModal(p);
      const body = document.getElementById("aircraft-modal-body");
      if (!body) return;
      const prediction = p && p.prediction;
      const closest = prediction && Number.isFinite(prediction.ground_distance_m)
        ? `${(prediction.ground_distance_m / 1000).toFixed(1)} km at ${Math.round(prediction.altitude_ft).toLocaleString()} ft in ${prediction.eta_seconds}s`
        : null;
      body.insertAdjacentHTML("beforeend", [
        detail("PASS RISK", prediction ? `${prediction.risk} · ${prediction.confidence} data confidence · NOT ARMED` : null),
        detail("WHY", prediction && prediction.reasons && prediction.reasons[0]),
        detail("CLOSEST APPROACH", closest),
        detail("FLIGHT PHASE", prediction && prediction.phase),
        detail("ARRIVING AT", prediction && prediction.arrival_at),
        detail("VERTICAL RATE", Number.isFinite(p.vertical_rate_fpm) ? `${Math.round(p.vertical_rate_fpm)} ft/min` : null),
        detail("LOUDNESS INDEX", Number.isFinite(p.noise_index) ? `${p.noise_index} relative dB · NOT measured` : null),
        detail("MODEL", prediction && prediction.model),
      ].join(""));
    };
  }
})();
