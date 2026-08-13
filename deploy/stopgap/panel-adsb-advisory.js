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

  /* Type and operator glyphs.
   *
   * The glyph is a REDUNDANT channel, not a decorative one. Band colour already
   * carries urgency, but colour alone fails for a colour-blind reader and fails
   * again on the panel's washed-out daylight rendering -- so shape has to carry
   * the same distinction independently. Every glyph is a distinct silhouette,
   * not a distinct hue.
   *
   * Operator outranks airframe: knowing it is the air ambulance matters more
   * than knowing it is a rotorcraft, because the owner reports those as the
   * dominant noise source and they routinely broadcast no emitter category.
   *
   * The empty case is deliberately NOT blank. An unidentified aircraft gets a
   * visible "unknown" mark, because a blank cell reads as "ordinary" and this
   * panel must never render not-measured as measured-and-fine. */
  const GLYPH = {
    AIR_AMBULANCE: { mark: "✚", title: "air ambulance" },
    POLICE: { mark: "◎", title: "police air support (orbits)" },
    rotorcraft: { mark: "✜", title: "rotorcraft" },
    heavy: { mark: "▚", title: "heavy jet" },
    jet: { mark: "▞", title: "jet" },
    light: { mark: "▵", title: "light aircraft" },
    unknown: { mark: "◌", title: "type not reported" },
  };

  function glyphFor(plane) {
    if (!plane) return GLYPH.unknown;
    /* operator_kind is only ever set from the Mode S roster, which is a
       positive identification -- never inferred from a shared airframe type. */
    if (plane.operator_kind && GLYPH[plane.operator_kind]) return GLYPH[plane.operator_kind];
    if (plane.airframe_class && GLYPH[plane.airframe_class]) return GLYPH[plane.airframe_class];
    return GLYPH.unknown;
  }

  function glyphCell(plane) {
    const g = glyphFor(plane);
    /* Police get the radar's cyan rather than the klaxon red: the alarm colour
       is spoken for by the medical path and must not be diluted by aircraft. */
    const tint = plane && plane.operator_kind === "POLICE" ? "var(--spo2)"
      : plane && plane.operator_kind === "AIR_AMBULANCE" ? "var(--klaxon)"
      : "var(--unknown-hi)";
    const label = plane && plane.registration
      ? `${g.title} · ${plane.registration}` : g.title;
    return `<span title="${esc(label)}" aria-label="${esc(label)}" `
      + `style="color:${tint};font-size:13px;line-height:1">${g.mark}</span>`;
  }

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
        + "grid-template-columns:16px 76px 92px 62px 96px 1fr;gap:6px;padding:3px 4px;"
        + `border-left:3px solid ${edge};background:#0B1924;margin-bottom:3px`;
      /* An identified aircraft shows its registration in place of a blank or a
         raw hex: N406LF is meaningful to the reader, A4C3EB is not. */
      const name = r.registration || r.callsign || r.hex;
      return `<div style="${style}">`
        + glyphCell(r)
        + `<span style="color:var(--nibp)">${esc(name)}</span>`
        + loudnessCell(p)
        + `<b style="color:${urgent ? "var(--klaxon)" : "var(--label)"}">${esc(when)}</b>`
        + `<span style="color:var(--unknown)">${where} ${alt}</span>`
        + `<span style="color:var(--mute)">${esc(p.phase || p.proximity || "")}</span>`
        + `</div>`;
    }).join("");
    return header + body;
  }

  /* Unknown is a third state. An aircraft we could not score is never folded
     into "nothing expected" -- that would let a missing emitter category read as
     reassurance. It is counted here, in its own colour, whenever it exists.

     The standing "nothing is evaluating these" disclaimer was removed at the
     owner's request as redundant. The NOT ARMED claim itself is NOT redundant
     and stays in the list header on every frame. */
  function advisoryMarkup(advisory) {
    const unscored = (advisory && advisory.unscored) || [];
    if (!unscored.length) return "";
    return `<div style="grid-column:1/-1;color:var(--unknown-hi);margin-top:4px;`
      + `border:1px solid #52606A;padding:4px;letter-spacing:.5px;font-size:11px">`
      + `${unscored.length} FURTHER AIRCRAFT NOT SCORED — ${esc(unscored[0].reason)}</div>`;
  }

  /* The radar canvas hardcodes a "NNdB" suffix on every blip. Since the
     2026-08-13 recalibration that number IS an estimated outdoor dBA rather
     than an index against an invented reference -- but it carries +/-5-10 dB,
     so rendering it as a crisp two-digit dB figure still overstates what is
     known. The suffix stays stripped, for a better reason than before.
     The canvas is drawn imperatively, so the only way to correct it
     without forking the whole radar is to intercept fillText for the duration
     of the call. Restored in a finally so a throw cannot leave it patched. */
  const ctxProto = CanvasRenderingContext2D.prototype;
  const originalFillText = ctxProto.fillText;

  /* Identifier -> glyph for whatever the radar is about to draw.
   *
   * The radar's blip labels are the aircraft's callsign or hex, so keying on
   * both lets a glyph be prefixed onto the label without knowing anything about
   * the canvas projection -- which matters because drawRadar() and its
   * lat/lon-to-pixel maths live only in the deployed index.html and are not in
   * git. Overlaying by coordinate would require forking that file; prefixing a
   * label does not. */
  function glyphIndex(planes) {
    const index = new Map();
    (planes || []).forEach(p => {
      const g = glyphFor(p);
      if (!g || g === GLYPH.unknown) return;
      [p.callsign, p.hex, p.registration].forEach(key => {
        if (key) index.set(String(key).trim().toUpperCase(), g.mark);
      });
    });
    return index;
  }

  function withHonestLabels(index, draw) {
    ctxProto.fillText = function (text, x, y, maxWidth) {
      let fixed = text;
      if (typeof text === "string") {
        /* The canvas hardcodes a "NNdB" suffix. The figure is an estimated
           dBA with +/-5-10 dB error bars -- shown as a rank, not a reading. */
        fixed = text.replace(/\s*(-?\d+)dB\b/, " ·$1");
        /* Prefix the glyph when the label leads with an identifier we know.
           Matching the leading token only, so a substring inside some other
           caption cannot pick up a spurious aircraft mark. */
        const lead = fixed.trim().split(/\s+/)[0];
        const mark = lead ? index.get(lead.toUpperCase()) : undefined;
        if (mark) fixed = `${mark} ${fixed}`;
      }
      return maxWidth === undefined
        ? originalFillText.call(this, fixed, x, y)
        : originalFillText.call(this, fixed, x, y, maxWidth);
    };
    try { draw(); } finally { ctxProto.fillText = originalFillText; }
  }

  window.drawRadar = function drawRadarWithAdvisory(planes, topNoise) {
    withHonestLabels(glyphIndex(planes), () => originalDrawRadar(planes, topNoise));
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
      /* Identification is stated with its evidence, never as a bare label. A
         CONFIRMED here means a Mode S address matched the roster; UNCONFIRMED
         means we do not know, which is not the same as "not one of them". */
      const identified = p && p.operator_kind
        ? `${glyphFor(p).title.toUpperCase()}${p.registration ? ` · ${p.registration}` : ""}`
          + `${p.icao_type ? ` · ${p.icao_type}` : ""}`
        : null;
      body.insertAdjacentHTML("beforeend", [
        detail("IDENTIFIED AS", identified),
        detail("IDENTIFIED HOW", p && p.operator_reason),
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
