/* Live-track aircraft proximity advisory for the stopgap kiosk.
 * Display only: this is not a measured sound forecast and is never armed. */
(() => {
  "use strict";

  const originalDrawRadar = window.drawRadar;
  const originalShowAircraftModal = window.showAircraftModal;
  if (typeof originalDrawRadar !== "function") return;

  function advisoryMarkup(advisory) {
    const next = advisory && advisory.next;
    let detail = "NO PROJECTED CLOSE PASS IN 3 MIN";
    if (next && next.prediction) {
      const p = next.prediction;
      const eta = p.eta_seconds < 60
        ? `${p.eta_seconds}s`
        : `${Math.floor(p.eta_seconds / 60)}m ${p.eta_seconds % 60}s`;
      const color = p.risk === "LIKELY" ? "var(--klaxon)" : "var(--watch)";
      const phase = p.phase ? ` · ${esc(p.phase)}` : "";
      detail = `<b style="color:${color}">${esc(p.risk)}</b> · ${esc(next.callsign || next.hex)}${phase}`
             + `<br>CPA ${(p.ground_distance_m / 1000).toFixed(1)} km at ${Math.round(p.altitude_ft).toLocaleString()} ft in ${eta}`;
    }
    /* Unknown is a third state. An aircraft we could not score is never folded
       into "no close pass" -- that would let a missing emitter category read as
       reassurance. It is counted here, in its own colour, always. */
    const unscored = (advisory && advisory.unscored) || [];
    const unknownLine = unscored.length
      ? `<br><span style="color:var(--unknown-hi)">${unscored.length} AIRCRAFT NOT SCORED — ${esc(unscored[0].reason)}</span>`
      : "";
    return `<div style="grid-column:1/-1;color:var(--unknown-hi);margin-bottom:5px;border:1px solid #52606A;padding:5px;letter-spacing:.5px">NOISE FORECAST · NOT ARMED · UNVALIDATED<br>${detail}${unknownLine}</div>`;
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
    const advisory = window.S && S.live && window.live && live.snap
      ? live.snap.adsb_advisory
      : null;
    list.innerHTML = advisoryMarkup(advisory) + list.innerHTML
      .replace("TOP 10 LOUDEST · 30 MIN", "RELATIVE LOUDNESS INDEX · NOT dB");
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
