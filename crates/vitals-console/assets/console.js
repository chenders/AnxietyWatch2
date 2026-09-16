(() => {
  "use strict";

  const FEED_EXPECT_S = 45;
  const REFRESH_MS = 30000;
  const VALID_DIRS = new Set(["a", "b", "c"]);
  const VALID_SCREENS = new Set(["overview", "display", "ingest", "attempts", "air11"]);
  const params = new URLSearchParams(location.search);
  const cookieValue = document.cookie.split(";").map(v => v.trim()).find(v => v.startsWith("console_dir="))?.split("=")[1];
  const queryDir = params.get("dir");
  const direction = VALID_DIRS.has(queryDir) ? queryDir : (VALID_DIRS.has(cookieValue) ? cookieValue : "a");
  const screen = VALID_SCREENS.has(params.get("screen")) ? params.get("screen") : "overview";
  const names = { a: "Service Mode", b: "Preflight", c: "Flowpath" };
  const main = document.querySelector("main");
  let lastSuccessMs = null;
  let model = null;
  let timer = null;
  let air11ActionNotice = null;
  let air11AdminToken = "";

  document.cookie = `console_dir=${direction}; Path=/console; SameSite=Strict; Max-Age=31536000`;
  document.body.dataset.dir = direction;
  document.querySelector("#direction-style").href = `/console/dir-${direction}.css`;
  document.querySelector("#layout-direction").value = direction;
  document.querySelector("#screen-field").value = screen;
  document.querySelector("#direction-name").textContent = names[direction];
  document.querySelectorAll("[data-screen-link]").forEach(link => {
    const target = link.dataset.screenLink;
    link.href = `?screen=${target}&dir=${direction}`;
    if (target === screen) link.setAttribute("aria-current", "page");
  });
  document.querySelector("#reconnect").addEventListener("click", refresh);

  const esc = value => String(value ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"})[c]);
  const ageText = seconds => {
    if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return "not observed";
    if (seconds < 60) return `${Math.max(0, Math.floor(seconds))}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  };
  const isoAge = value => {
    const ms = Date.parse(value);
    return Number.isFinite(ms) ? Math.max(0, (Date.now() - ms) / 1000) : null;
  };
  const state = value => {
    const normalized = String(value || "UNKNOWN").toUpperCase();
    return normalized === "LIVE" || normalized === "SUCCEEDED" ? "LIVE" : normalized === "FAILING" || normalized === "FAILED" ? "FAILING" : "UNKNOWN";
  };
  const glyph = value => ({ LIVE: "●", FAILING: "▲", UNKNOWN: "?" })[state(value)];
  const statusCard = (title, status, reason, age, expectation, detail = "") => ({ title, state: state(status), reason, age, expectation, detail });
  const unavailable = (title, result, expectation = `expect ≤ ${FEED_EXPECT_S}s`) => statusCard(title, "UNKNOWN", result.error, null, expectation);
  const own = {
    PolarH10: "Owns chest-strap heart rate and RR intervals.", PolarOH1: "Owns optical arm-sensor pulse observations.",
    Fenix7: "Owns watch-origin activity and physiology observations.", SmartScale: "Owns weight and body-composition observations.",
    BpCuff: "Owns cuff blood-pressure observations.", Manual: "Owns operator-entered observations."
  };

  async function get(path) {
    try {
      const response = await fetch(path, { headers: { Accept: "application/json" }, cache: "no-store" });
      const body = await response.json().catch(() => null);
      if (!response.ok) return { ok: false, error: `${path} returned HTTP ${response.status}${body?.error ? `: ${body.error}` : ""}` };
      return { ok: true, data: body };
    } catch (error) {
      return { ok: false, error: `${path} unavailable: ${error.message}` };
    }
  }

  function sourceExpectation(source) {
    if (source.degrade_deadline_epoch_s != null && source.observation_age_s != null) {
      const remaining = source.degrade_deadline_epoch_s - Date.now() / 1000;
      return `expect ≤ ${Math.max(0, Math.round(source.observation_age_s + remaining))}s`;
    }
    return "expect registry cadence (deadline not reported)";
  }

  function sourceCards(health) {
    if (!health.ok) return [unavailable("Registry sources", health)];
    return health.data.sources.map(source => statusCard(
      source.name, source.state, source.reason,
      source.observation_age_s, sourceExpectation(source), own[source.name] || "Owns observations assigned to this registry source."
    ));
  }

  function overview(results) {
    const cards = [];
    if (results.health.ok) {
      const h = results.health.data;
      const reachable = h.vitalsd === "reachable";
      cards.push(statusCard("vitalsd", reachable ? "LIVE" : "UNKNOWN", reachable ? `daemon answered; uptime ${h.uptime ?? "not reported"}s` : `daemon unreachable: ${JSON.stringify(h.vitalsd)}`, 0, `expect ≤ ${FEED_EXPECT_S}s`));
      cards.push(statusCard("Hardware heartbeat", h.heartbeat_pulsing === true ? "LIVE" : h.heartbeat_pulsing === false ? "FAILING" : "UNKNOWN", h.heartbeat_pulsing === true ? "heartbeat is pulsing" : h.heartbeat_pulsing === false ? "heartbeat is not pulsing" : "daemon did not report heartbeat state", 0, `expect ≤ ${FEED_EXPECT_S}s`));
    } else cards.push(unavailable("vitalsd", results.health));
    if (results.host.ok) {
      const h = results.host.data;
      const disk = h.disk_total_bytes > 0 ? `${(100 * h.disk_free_bytes / h.disk_total_bytes).toFixed(1)}% free (${h.disk_free_bytes} of ${h.disk_total_bytes} bytes)` : "disk total is not reported";
      cards.push(statusCard("Disk", "UNKNOWN", `${disk}; API supplies no health threshold`, 0, `expect ≤ ${FEED_EXPECT_S}s`));
      cards.push(statusCard("Thermal", "UNKNOWN", h.thermal_c == null ? "unmeasured / not available on this host" : `${h.thermal_c} °C; API supplies no health threshold`, 0, `expect ≤ ${FEED_EXPECT_S}s`));
      if (h.throttle == null) {
        cards.push(statusCard("Undervoltage", "UNKNOWN", h.throttle_unmeasured_reason || "firmware throttle state was not measured", 0, `expect ≤ ${FEED_EXPECT_S}s`));
      } else {
        const under = h.throttle.undervoltage_now || h.throttle.undervoltage_latched;
        cards.push(statusCard("Undervoltage", under ? "FAILING" : "LIVE", under ? `undervoltage flags: now=${h.throttle.undervoltage_now}, latched=${h.throttle.undervoltage_latched}` : "no current or latched undervoltage", 0, `expect ≤ ${FEED_EXPECT_S}s`));
      }
      cards.push(statusCard("Network", h.network_interface ? "LIVE" : "UNKNOWN", h.network_interface ? `default route via ${h.network_interface}; wireless signal ${h.wireless_signal ?? "not reported"}` : "no default-route interface reported", 0, `expect ≤ ${FEED_EXPECT_S}s`));
      cards.push(statusCard("Watchdog", h.watchdog_present ? "LIVE" : "FAILING", h.watchdog_present ? "/dev/watchdog is present" : "/dev/watchdog is absent", 0, `expect ≤ ${FEED_EXPECT_S}s`));
    } else cards.push(unavailable("Host readouts", results.host));
    if (results.clock.ok) {
      const c = results.clock.data;
      cards.push(statusCard("Clock", c.ntp_synchronized ? "LIVE" : "FAILING", `NTP synchronized=${c.ntp_synchronized}; RTC drift=${c.drift_usec ?? "not reported"} µs`, 0, `expect ≤ ${FEED_EXPECT_S}s`));
    } else cards.push(unavailable("Clock", results.clock));
    const priority = { UNKNOWN: 0, FAILING: 1, LIVE: 2 };
    cards.sort((a, b) => priority[a.state] - priority[b.state]);
    return { screen: "overview", eyebrow: "Appliance posture", title: "Overview", intro: "Exceptions lead; healthy systems recede.", sections: [{ title: "Host and daemon", cards }] };
  }

  function display(results) {
    const cards = [];
    if (!results.display.ok) cards.push(unavailable("Display path", results.display));
    else {
      const d = results.display.data;
      cards.push(statusCard("Compositor", d.compositor_alive ? "LIVE" : "FAILING", d.compositor_alive ? "cage process is running" : "cage process is not running", 0, `expect ≤ ${FEED_EXPECT_S}s`));
      cards.push(statusCard("Browser", d.browser_alive ? "LIVE" : "FAILING", d.browser_alive ? "chromium process is running" : "chromium process is not running", 0, `expect ≤ ${FEED_EXPECT_S}s`));
      cards.push(statusCard("Wayland socket", d.wayland_socket_exists ? "LIVE" : "FAILING", d.wayland_socket_exists ? "a /run/user/*/wayland-* socket exists" : "no Wayland socket exists", 0, `expect ≤ ${FEED_EXPECT_S}s`));
      if (d.drm_connectors.length === 0) cards.push(statusCard("DRM connector", "UNKNOWN", "no DRM connector status was reported", 0, `expect ≤ ${FEED_EXPECT_S}s`));
      d.drm_connectors.forEach(connector => cards.push(statusCard(`DRM ${connector.name}`, connector.status === "connected" ? "LIVE" : connector.status === "disconnected" ? "FAILING" : "UNKNOWN", `connector reports ${connector.status}`, 0, `expect ≤ ${FEED_EXPECT_S}s`)));
    }
    return { screen: "display", eyebrow: "Independent evidence", title: "Display path", intro: "Service-up is not treated as proof that the screen is lit.", sections: [{ title: "Signal chain", cards }] };
  }

  function ingest(results) {
    return { screen: "ingest", eyebrow: "Registry ownership", title: "Ingest", intro: "Every registered source is shown; absent evidence remains unknown.", sections: [{ title: "Sources", cards: sourceCards(results.health) }] };
  }

  function attempts(results) {
    if (!results.attempts.ok) return { screen: "attempts", eyebrow: "Durable evidence", title: "Attempt history", intro: "Started work remains visible even when it never finishes.", sections: [{ title: "Attempt ledger", cards: [unavailable("Attempt ledger", results.attempts, "expect on each attempt ≤ its recorded duration")] }] };
    const cards = results.attempts.data.map(row => {
      const age = isoAge(row.finished_at || row.started_at);
      const s = row.outcome == null ? (age != null && age > row.expected_duration_s ? "UNKNOWN" : "UNKNOWN") : row.outcome;
      const effect = row.effect ? `effect created=${row.effect.created}, updated=${row.effect.updated}, replayed=${row.effect.replayed}, rejected=${row.effect.rejected}` : "effect not reported";
      const reason = row.outcome == null ? `attempt has no outcome; started ${row.started_at}` : `outcome=${row.outcome}; ${effect}${row.detail ? `; ${row.detail}` : ""}`;
      return statusCard(`${row.kind} · ${row.subject}`, s, reason, age, `expect ≤ ${row.expected_duration_s}s`, `Attempt #${row.id}`);
    });
    if (cards.length === 0) cards.push(statusCard("Attempt ledger", "UNKNOWN", "endpoint returned no attempt rows", 0, "expect when work is attempted"));
    return { screen: "attempts", eyebrow: "Durable evidence", title: "Attempt history", intro: "Actual rows from /api/attempts, newest first.", sections: [{ title: "Attempt ledger", cards }] };
  }

  const air11Text = value => String(value == null ? "not observed" : value).replaceAll("_", " ").toUpperCase();
  const air11Health = value => {
    const normalized = String(value || "").toLowerCase();
    if (["streaming", "authenticated", "connected", "measured", "active", "running", "ready", "observed", "succeeded"].includes(normalized)) return "live";
    if (["degraded", "disconnected", "failed", "invalid", "insecure", "stale"].includes(normalized)) return "failing";
    return "unknown";
  };
  const air11TimestampAge = micros => {
    if (micros == null || !Number.isFinite(Number(micros))) return null;
    const age = Date.now() / 1000 - Number(micros) / 1000000;
    return Number.isFinite(age) && age >= 0 ? age : null;
  };
  const air11Value = value => value == null ? "not measured" : String(value);
  const air11TimestampLabel = micros => {
    if (micros == null || !Number.isFinite(Number(micros))) return "not observed";
    const date = new Date(Number(micros) / 1000);
    return Number.isNaN(date.getTime()) ? "invalid timestamp" : date.toISOString();
  };

  function air11Page(results) {
    if (!results.air11.ok) {
      return {
        screen: "air11", eyebrow: "Read-only BLE integration", title: "AirSense 11",
        intro: "The AirSense endpoint could not be read; no device state is inferred.",
        air11: null, endpointError: results.air11.error, actionNotice: air11ActionNotice,
      };
    }
    return {
      screen: "air11", eyebrow: "Read-only BLE integration", title: "AirSense 11",
      intro: "Therapy telemetry is observed without pairing, control, or direct database writes.",
      air11: results.air11.data, endpointError: null, actionNotice: air11ActionNotice,
    };
  }

  function renderAir11(model, h, direction) {
    const data = model.air11;
    if (!data) return `<div class="air11-page direction-${direction}">
      <header class="page-head"><p class="eyebrow">${h.esc(model.eyebrow)} · ${direction.toUpperCase()}</p><h1>${h.esc(model.title)}</h1><p>${h.esc(model.intro)}</p></header>
      <div class="air11-alert air11-alert--unknown" role="alert"><span class="air11-alert__label">UNKNOWN · ENDPOINT NOT OBSERVED</span><h2>AirSense status cannot be confirmed</h2><p>${h.esc(model.endpointError || "No response was received from /api/air11.")}</p></div>
    </div>`;

    const status = data.status || {};
    const config = data.config || {};
    const credentials = data.credentials || {};
    const service = data.service || {};
    const logs = data.logs || { state: "unavailable", entries: [], blocker: "service journal was not observed" };
    const optedIn = config.opted_in === true;
    const alerts = [];
    const addAlert = (kind, label, detail) => { if (detail) alerts.push({ kind, label, detail }); };
    if (optedIn && credentials.state !== "ready") addAlert("critical", "CREDENTIAL BLOCKER", credentials.blocker || `credential state=${credentials.state || "unknown"}`);
    if (config.blocker) addAlert(config.state === "unavailable" ? "unknown" : "critical", "CONFIGURATION BLOCKER", config.blocker);
    if (service.blocker) addAlert("unknown", "SERVICE STATE UNKNOWN", service.blocker);
    if (status.blocker) addAlert(air11Health(status.link_state) === "failing" ? "critical" : "unknown", "LAST COLLECTOR BLOCKER", status.blocker);
    if (status.durability_unknown === true) addAlert("critical", "DURABILITY UNKNOWN", "vitalsd did not confirm the last append; the raw journal is retained for replay.");
    if (logs.blocker) addAlert("unknown", "LOG FEED UNKNOWN", logs.blocker);
    const logErrors = Array.isArray(logs.entries) ? logs.entries.filter(entry => entry.level === "error") : [];
    if (logErrors.length) addAlert("critical", "COLLECTOR ERROR LOG", `${logErrors.length} error log entr${logErrors.length === 1 ? "y" : "ies"} are shown below.`);

    const overall = alerts.some(alert => alert.kind === "critical") ? "failing" : alerts.length ? "unknown" : optedIn ? "live" : "unknown";
    const alertHtml = alerts.length ? `<div class="air11-summary" aria-label="AirSense attention items">${alerts.map(alert => `<div class="air11-alert air11-alert--${alert.kind === "critical" ? "critical" : "unknown"}" role="${alert.kind === "critical" ? "alert" : "status"}"><span class="air11-alert__label">${h.esc(alert.label)}</span><h2>${alert.kind === "critical" ? "Action required" : "Current evidence is incomplete"}</h2><p>${h.esc(alert.detail)}</p></div>`).join("")}</div>` : `<div class="air11-alert" role="status"><span class="air11-alert__label">${overall === "live" ? "NOMINAL" : "NOT CONFIGURED"}</span><h2>${overall === "live" ? "Read-only observation is active" : "AirSense is not opted in"}</h2><p>${overall === "live" ? "No current blocker has been reported by the collector, service manager, or journal." : "Opt-in is a deliberate non-secret configuration choice; no credential is required until the collector is started."}</p></div>`;

    const fact = (title, value, health, detail) => `<article class="air11-fact air11-fact--${health}"><h2>${h.esc(title)}</h2><span class="air11-fact__value">${h.esc(value)}</span><p>${h.esc(detail)}</p></article>`;
    const linkHealth = air11Health(status.link_state);
    const measurementHealth = status.measurement_state === "measured" ? "live" : status.measurement_state === "stale" || status.measurement_state === "quarantined" ? "failing" : "unknown";
    const processHealth = service.active_state === "active" && service.sub_state === "running" ? "live" : service.active_state === "failed" ? "failing" : "unknown";
    const therapyHealth = status.therapy_state === "active" || status.therapy_state === "complete" ? "live" : "unknown";
    const credentialHealth = credentials.state === "ready" ? "live" : optedIn && ["invalid", "insecure"].includes(credentials.state) ? "failing" : "unknown";
    const receiptAge = air11TimestampAge(status.last_host_receipt_unix_micros);
    const receiptDetail = receiptAge == null ? "host receipt time not measured" : `${h.ageText(receiptAge)} · host clock is authoritative`;
    const configLabel = config.state === "opted_in" ? "OPTED IN" : config.state === "not_opted_in" || config.state === "not_configured" ? "NOT CONFIGURED" : "UNKNOWN";
    const statusLabel = status.status_state === "observed" ? `${air11Text(status.link_state)} / ${air11Text(status.measurement_state)}` : "NOT OBSERVED";
    const configHealth = config.state === "opted_in" ? "live" : ["not_opted_in", "not_configured"].includes(config.state) ? "absent" : "unknown";
    const facts = [
      fact("Collector link", statusLabel, linkHealth, status.blocker || "BLE and authenticated-stream state; no control operation is exposed."),
      fact("Measurement evidence", air11Text(status.measurement_state), measurementHealth, status.measurement_state === "measured" ? `${h.ageText(receiptAge)} · latest journaled host receipt` : "null/unavailable values remain unmeasured; they are never shown as zero."),
      fact("Service process", air11Text(service.active_state), processHealth, `${air11Text(service.sub_state)} · restarts=${air11Value(service.restart_count)}${service.started_at ? ` · started ${service.started_at}` : ""}`),
      fact("Credential readiness", air11Text(credentials.state), credentialHealth, credentials.blocker || "External file shape is ready; the key value is never returned."),
      fact("Opt-in configuration", configLabel, configHealth, config.blocker || "Stored in PostgreSQL as non-secret intent only; it does not contain credentials."),
      fact("Last host receipt", receiptAge == null ? "NOT OBSERVED" : h.ageText(receiptAge), receiptAge == null ? "unknown" : "live", receiptDetail),
    ].join("");

    const metric = (label, value) => `<div><dt>${h.esc(label)}</dt><dd class="air11-mono">${h.esc(air11Value(value))}</dd></div>`;
    const counters = [
      metric("Session id", status.session_id), metric("Reconnects", status.reconnect_count), metric("Retries", status.retry_count),
      metric("Received envelopes", status.received_envelopes), metric("Decoded frames", status.decoded_frames), metric("Published samples", status.published_samples),
      metric("Unavailable/null", status.unavailable), metric("Duplicates", status.duplicates), metric("Quarantined", status.quarantined),
      metric("Dropped", status.dropped), metric("Durability unknown", status.durability_unknown), metric("Device time provenance", status.last_device_start_time),
    ].join("");
    const latestRows = Array.isArray(data.latest) ? data.latest.map(row => {
      const latestAge = air11TimestampAge(row.t_micros);
      const displayValue = Number.isFinite(Number(row.value)) ? Number(row.value).toLocaleString(undefined, {maximumFractionDigits: 3}) : "invalid";
      return `<tr><th scope="row">${h.esc(row.name)}</th><td class="air11-mono">${h.esc(displayValue)}</td><td>${h.esc(row.unit)}</td><td class="air11-mono">${h.esc(h.ageText(latestAge))}</td></tr>`;
    }).join("") : "";
    const latestHtml = latestRows ? `<div class="air11-table-wrap"><table class="air11-table"><thead><tr><th>Measurement</th><th>Value</th><th>Unit</th><th>Age</th></tr></thead><tbody>${latestRows}</tbody></table></div>` : `<p class="air11-empty">${h.esc(data.latest_blocker || "No stored AirSense measurements are available.")}</p>`;

    const blockerHtml = status.blocker ? `<p class="air11-blocker ${linkHealth === "failing" ? "" : "air11-blocker--unknown"}"><strong>Last collector blocker:</strong> ${h.esc(status.blocker)}</p>` : `<p class="air11-callout"><strong>No blocker reported.</strong> This is an observation, not a guarantee that the therapy device is available beyond the latest host receipt.</p>`;
    const credentialDetails = [
      metric("Client configured", credentials.client_id_present), metric("Target configured", credentials.target_present),
      metric("Master key shape", credentials.master_key_present), metric("File permissions", credentials.permissions),
    ].join("");
    const therapyDetails = [
      metric("Session id", status.therapy_session_id), metric("Lifecycle state", status.therapy_state),
      metric("Event coverage", status.event_subscription_state), metric("Event records", status.event_records),
      metric("Start evidence", air11TimestampLabel(status.therapy_start_host_unix_micros)),
      metric("Stop evidence", air11TimestampLabel(status.therapy_stop_host_unix_micros)),
      metric("Finalized evidence", air11TimestampLabel(status.therapy_finalized_host_unix_micros)),
    ].join("");

    const actionDisabled = data.actions_enabled ? "" : " disabled";
    const button = (action, label, className, confirmation = "") => `<button type="button" class="air11-button ${className}" data-air11-action="${action}" data-air11-confirm="${confirmation}"${actionDisabled}>${h.esc(label)}</button>`;
    const startReason = !optedIn ? "Opt in first" : credentials.state !== "ready" ? "Credential file is not ready" : "Start the fixed read-only collector";
    const lifecycle = `<div class="air11-actions">
      ${optedIn ? button("opt_out", "Opt out", "air11-button--danger", "OPT_OUT_AIR11") : button("opt_in", "Opt in", "air11-button--safe", "OPT_IN_AIR11")}
      ${button("start", "Start collector", "air11-button--safe", "START_AIR11")}
      ${button("stop", "Stop collector", "air11-button--danger", "STOP_AIR11")}
      ${button("restart", "Restart collector", "air11-button--danger", "RESTART_AIR11")}
      ${button("credential_check", "Recheck credentials", "air11-button--unknown")}
    </div><p class="air11-fact__detail">Start gate: ${h.esc(startReason)}. Stop/restart disconnect the observation; they do not send a device stop command.</p>`;
    const auth = data.admin_auth_configured ? `<div class="air11-auth"><label for="air11-admin-token">Console-admin token for this tab (separate from the AirSense credential)</label><input id="air11-admin-token" type="password" autocomplete="off" spellcheck="false"><div class="air11-actions"><button type="button" class="air11-button air11-button--safe" data-air11-save-auth>Use token for this tab</button><button type="button" class="air11-button" data-air11-clear-auth>Clear token</button></div><p class="air11-fact__detail">The token is held only in page memory and is sent only in the authorization header for a fixed action.</p></div>` : `<div class="air11-blocker air11-blocker--unknown">Actions are disabled because the external console-admin capability is not configured. Provision that separate file outside this page; never paste the AirSense key here.</div>`;
    const actionNotice = model.actionNotice ? `<p class="air11-result ${model.actionNotice.ok ? "air11-result--ok" : "air11-result--error"}" role="status">${h.esc(model.actionNotice.message)}</p>` : "";

    const logHtml = Array.isArray(logs.entries) && logs.entries.length ? logs.entries.map(entry => `<article class="air11-log air11-log--${h.esc(entry.level === "error" ? "error" : entry.level === "warning" ? "warning" : "info")}"${entry.level === "error" ? " role=alert" : ""}><span class="air11-log__level">${h.esc(entry.level)}</span><span class="air11-log__time air11-mono">${h.esc(entry.observed_at)}</span><span class="air11-log__message">${h.esc(entry.message)}</span></article>`).join("") : `<p class="air11-empty">${logs.state === "empty" ? "The journal was checked and returned no entries for this unit." : "No sanitized journal entries are available."}</p>`;
    const auditHtml = Array.isArray(data.audit) && data.audit.length ? `<div class="air11-table-wrap"><table class="air11-table"><thead><tr><th>When</th><th>Action</th><th>Outcome</th><th>Detail</th></tr></thead><tbody>${data.audit.map(row => `<tr><td class="air11-mono">${h.esc(row.created_at)}</td><td>${h.esc(row.action)}</td><td class="air11-outcome--${h.esc(row.outcome)}">${h.esc(row.outcome)}</td><td>${h.esc(row.detail)}</td></tr>`).join("")}</tbody></table></div>` : `<p class="air11-empty">No AirSense admin actions are recorded yet.</p>`;

    return `<div class="air11-page direction-${direction}">
      <header class="page-head"><p class="eyebrow">${h.esc(model.eyebrow)} · ${direction.toUpperCase()}</p><h1>${h.esc(model.title)}</h1><p>${h.esc(model.intro)}</p></header>
      ${alertHtml}
      <section class="air11-facts" aria-label="AirSense summary">${facts}</section>
      <section class="air11-section" aria-labelledby="air11-evidence"><header><h2 id="air11-evidence">Current evidence</h2><p>Last status is retained; unknown is never healthy.</p></header><div class="air11-grid"><article class="air11-panel"><h3>Status document</h3><dl class="air11-facts-list">${metric("Status file", status.status_state)}${metric("Link state", status.link_state)}${metric("Measurement state", status.measurement_state)}${metric("Host receipt age", receiptAge == null ? null : h.ageText(receiptAge))}${metric("Untrusted device time", status.last_device_start_time)}</dl>${blockerHtml}</article><article class="air11-panel"><h3>Ingest and durability counters</h3><dl class="air11-facts-list">${counters}</dl></article></div></section>
      <section class="air11-section" aria-labelledby="air11-latest"><header><h2 id="air11-latest">Latest stored AirSense measurements</h2><p>Canonical registry values from PostgreSQL; age is measured from the host receipt timestamp.</p></header><div class="air11-panel">${latestHtml}</div></section>
      <section class="air11-section" aria-labelledby="air11-therapy"><header><h2 id="air11-therapy">Therapy lifecycle and event coverage</h2><p>Therapy sessions remain separate from BLE transport sessions; unresolved boundaries remain unknown or provisional.</p></header><div class="air11-grid"><article class="air11-panel air11-panel--${therapyHealth}"><h3>Current therapy evidence</h3><dl class="air11-facts-list">${therapyDetails}</dl><p class="air11-callout"><strong>Boundary blocker:</strong> ${h.esc(status.therapy_blocker || status.event_blocker || "No lifecycle blocker reported.")}</p></article><article class="air11-panel"><h3>Event evidence</h3><dl class="air11-facts-list">${metric("Notifications", status.event_notifications)}${metric("Retained records", status.event_records)}${metric("Truncated notifications", status.event_truncated_notifications)}${metric("Last data id", status.last_event_data_id)}</dl><p class="air11-callout"><strong>AHI:</strong> not derived from the live stream. A trustworthy value requires finalized session coverage and the device summary/report path.</p></article></div></section>
      <section class="air11-section" aria-labelledby="air11-credentials"><header><h2 id="air11-credentials">Credential and opt-in configuration</h2><p>No AirSense key or device address is displayed.</p></header><div class="air11-grid"><article class="air11-panel"><h3>External credential readiness</h3><dl class="air11-facts-list">${credentialDetails}</dl><p class="air11-callout"><strong>Provisioning boundary:</strong> the operator-managed credential file remains outside PostgreSQL and this browser. The page only checks presence, permissions, and shape.</p></article><article class="air11-panel"><h3>Lifecycle controls</h3><dl class="air11-facts-list">${metric("Opt-in state", configLabel)}${metric("Service active", service.active_state)}${metric("Service substate", service.sub_state)}${metric("Restart count", service.restart_count)}</dl>${lifecycle}${auth}${actionNotice}</article></div></section>
      <section class="air11-section" aria-labelledby="air11-logs"><header><h2 id="air11-logs">Last status and service logs</h2><p>Journal entries are bounded and sanitized before display.</p></header><div class="air11-panel"><div class="air11-log-list" aria-live="polite">${logHtml}</div></div></section>
      <section class="air11-section" aria-labelledby="air11-audit"><header><h2 id="air11-audit">AirSense admin audit</h2><p>${h.esc(data.audit_state === "observed" ? "Append-only fixed-action history." : data.audit_blocker || "Audit state not observed.")}</p></header><div class="air11-panel">${auditHtml}</div></section>
      <dialog id="air11-confirm-dialog" class="air11-confirm"><form id="air11-confirm-form"><h2>Confirm AirSense action</h2><p>This changes only the collector process or non-secret opt-in state. It never controls therapy on the device.</p><p>Type <span id="air11-confirm-phrase" class="air11-confirm__phrase"></span> to continue.</p><input id="air11-confirm-input" autocomplete="off" spellcheck="false" required><input id="air11-confirm-action" type="hidden"><div class="air11-actions"><button type="button" class="air11-button" data-air11-cancel>Cancel</button><button type="submit" class="air11-button air11-button--danger">Confirm action</button></div></form></dialog>
    </div>`;
  }
  window.renderAir11 = renderAir11;

  async function loadModel() {
    const requests = { health: get("/api/health"), version: get("/api/version") };
    if (screen === "overview") { requests.host = get("/api/host"); requests.clock = get("/api/clock"); }
    if (screen === "display") requests.display = get("/api/display");
    if (screen === "attempts") requests.attempts = get("/api/attempts?limit=100");
    if (screen === "air11") requests.air11 = get("/api/air11");
    const keys = Object.keys(requests);
    const values = await Promise.all(Object.values(requests));
    const results = Object.fromEntries(keys.map((key, i) => [key, values[i]]));
    if (results.health.ok) lastSuccessMs = Date.now();
    if (results.version.ok) document.querySelector("#build-id").textContent = `Build: ${results.version.data.git || results.version.data.version}`;
    return ({ overview, display, ingest, attempts, air11: air11Page })[screen](results);
  }

  function actionMessage(ok, message) {
    air11ActionNotice = { ok, message };
    model = model && model.screen === "air11" ? { ...model, actionNotice: air11ActionNotice } : model;
  }

  async function postAir11(action, confirmation = null) {
    const data = model?.air11;
    if (!data?.actions_enabled || !data.action_token) {
      actionMessage(false, "AirSense action blocked: the page has no enabled action capability.");
      render();
      return;
    }
    if (!air11AdminToken) {
      actionMessage(false, "AirSense action blocked: enter the separate console-admin token before acting.");
      render();
      document.querySelector("#air11-admin-token")?.focus();
      return;
    }
    const response = await fetch("/api/air11", {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        Authorization: `Bearer ${air11AdminToken}`,
        "X-Air11-Action-Token": data.action_token,
      },
      body: JSON.stringify({ action, confirmation }),
      cache: "no-store",
    }).catch(error => ({ ok: false, status: 0, json: async () => ({ error: error.message }) }));
    const body = await response.json().catch(() => ({}));
    if (!response.ok) {
      if (response.status === 401) air11AdminToken = "";
      actionMessage(false, body.error || `AirSense action failed: HTTP ${response.status}`);
      render();
      return;
    }
    actionMessage(true, body.message || "AirSense action completed; current evidence is refreshing.");
    await refresh();
  }

  function bindAir11Actions() {
    document.querySelectorAll("[data-air11-save-auth]").forEach(button => button.addEventListener("click", () => {
      const value = document.querySelector("#air11-admin-token")?.value || "";
      if (!value) {
        actionMessage(false, "Console-admin token not saved: the field is empty.");
        render();
        return;
      }
      air11AdminToken = value;
      actionMessage(true, "Console-admin token held in memory for this tab; it has not been sent yet.");
      render();
    }));
    document.querySelectorAll("[data-air11-clear-auth]").forEach(button => button.addEventListener("click", () => {
      air11AdminToken = "";
      actionMessage(true, "Console-admin token cleared from this tab.");
      render();
    }));
    document.querySelectorAll("[data-air11-action]").forEach(button => button.addEventListener("click", () => {
      const action = button.dataset.air11Action;
      const phrase = button.dataset.air11Confirm || "";
      if (!action) return;
      if (!phrase) {
        postAir11(action);
        return;
      }
      const dialog = document.querySelector("#air11-confirm-dialog");
      const phraseElement = document.querySelector("#air11-confirm-phrase");
      const actionElement = document.querySelector("#air11-confirm-action");
      const input = document.querySelector("#air11-confirm-input");
      if (!dialog || !phraseElement || !actionElement || !input) return;
      phraseElement.textContent = phrase;
      actionElement.value = action;
      input.value = "";
      dialog.showModal();
      input.focus();
    }));
    document.querySelector("[data-air11-cancel]")?.addEventListener("click", () => document.querySelector("#air11-confirm-dialog")?.close());
    document.querySelector("#air11-confirm-form")?.addEventListener("submit", event => {
      event.preventDefault();
      const action = document.querySelector("#air11-confirm-action")?.value;
      const confirmation = document.querySelector("#air11-confirm-input")?.value;
      document.querySelector("#air11-confirm-dialog")?.close();
      if (action) postAir11(action, confirmation);
    });
  }

  function render() {
    if (!model || !window.ConsoleDirections?.[direction]) return;
    main.innerHTML = window.ConsoleDirections[direction](model, { esc, ageText, glyph });
    bindAir11Actions();
    tick();
  }

  function tick() {
    const elapsed = lastSuccessMs == null ? null : (Date.now() - lastSuccessMs) / 1000;
    document.querySelector("#feed-age").textContent = ageText(elapsed);
    document.querySelectorAll("[data-age-base]").forEach(el => {
      // Number("") is 0 — an absent age must stay "not observed", never
      // become a fresh-looking "0s ago".
      const raw = el.dataset.ageBase;
      const base = raw === "" || raw == null ? NaN : Number(raw);
      const at = el.dataset.ageAt === "" || el.dataset.ageAt == null ? NaN : Number(el.dataset.ageAt);
      const rendered = Number.isFinite(base) && Number.isFinite(at)
        ? base + (Date.now() - at) / 1000
        : null;
      el.textContent = ageText(rendered);
    });
    const stale = elapsed == null || elapsed > FEED_EXPECT_S;
    const overlay = document.querySelector("#feed-overlay");
    overlay.hidden = !stale;
    document.body.dataset.feed = stale ? "stale" : "fresh";
    document.querySelector("#feed-detail").textContent = elapsed == null ? "No successful /api/health observation is available." : `Last successful /api/health observation was ${ageText(elapsed)}; expected within ${FEED_EXPECT_S}s.`;
  }

  async function refresh() {
    document.querySelector("#reconnect").disabled = true;
    model = await loadModel();
    render();
    document.querySelector("#reconnect").disabled = false;
  }

  const script = document.createElement("script");
  script.src = `/console/dir-${direction}.js`;
  script.onload = refresh;
  script.onerror = () => { main.innerHTML = "<h1>Console renderer unavailable</h1><p>The selected direction script could not be loaded.</p>"; tick(); };
  document.head.appendChild(script);
  timer = setInterval(() => { tick(); if (lastSuccessMs == null || Date.now() - lastSuccessMs >= REFRESH_MS) refresh(); }, 1000);
  window.addEventListener("pagehide", () => clearInterval(timer));
})();
