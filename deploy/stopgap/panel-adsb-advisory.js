/* Live-track aircraft proximity advisory for the stopgap kiosk.
 * Display only: this is not a measured sound forecast and is never armed. */
(() => {
  "use strict";

  const originalDrawRadar = window.drawRadar;
  const originalShowAircraftModal = window.showAircraftModal;
  if (typeof originalDrawRadar !== "function") return;

  function advisoryMarkup(advisory) {
    const next = advisory && advisory.next;
    let detail = "NO PROJECTED CLOSE PASS IN 10 MIN";
    if (next && next.prediction) {
      const p = next.prediction;
      const eta = p.eta_seconds < 60
        ? `${p.eta_seconds}s`
        : `${Math.floor(p.eta_seconds / 60)}m ${p.eta_seconds % 60}s`;
      const color = p.risk === "LIKELY" ? "var(--klaxon)" : "var(--watch)";
      detail = `<b style="color:${color}">${esc(p.risk)}</b> · ${esc(next.callsign || next.hex)} · CPA ${(p.ground_distance_m / 1000).toFixed(1)} km in ${eta}`;
    }
    return `<div style="grid-column:1/-1;color:var(--unknown-hi);margin-bottom:5px;border:1px solid #52606A;padding:5px;letter-spacing:.5px">NOISE FORECAST · NOT ARMED · UNVALIDATED<br>${detail}</div>`;
  }

  window.drawRadar = function drawRadarWithAdvisory(planes, topNoise) {
    originalDrawRadar(planes, topNoise);
    const list = document.getElementById("live-radar-list");
    if (!list) return;
    const advisory = window.S && S.live && window.live && live.snap
      ? live.snap.adsb_advisory
      : null;
    list.innerHTML = advisoryMarkup(advisory) + list.innerHTML
      .replace("TOP 10 LOUDEST · 30 MIN", "EST. PROXIMITY HISTORY · NOT MEASURED dB");
    const heading = document.querySelector(".r-radar [style*='letter-spacing:1.5px']");
    if (heading && !heading.textContent.includes("NOT ARMED")) heading.textContent += " · NOT ARMED";
  };

  if (typeof originalShowAircraftModal === "function") {
    window.showAircraftModal = function showAircraftPrediction(p) {
      originalShowAircraftModal(p);
      const body = document.getElementById("aircraft-modal-body");
      if (!body) return;
      const prediction = p && p.prediction;
      body.insertAdjacentHTML("beforeend", [
        detail("PASS RISK", prediction ? `${prediction.risk} · ${prediction.confidence} data confidence · NOT ARMED` : null),
        detail("CLOSEST APPROACH", prediction ? `${(prediction.ground_distance_m / 1000).toFixed(1)} km in ${prediction.eta_seconds}s` : null),
        detail("VERTICAL RATE", Number.isFinite(p.vertical_rate_fpm) ? `${Math.round(p.vertical_rate_fpm)} ft/min` : null),
        detail("MODEL", prediction && prediction.model),
      ].join(""));
    };
  }
})();
