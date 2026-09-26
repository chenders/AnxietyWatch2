(() => {
  "use strict";

  const FEED_EXPECT_S = 45;
  const REFRESH_MS = 30000;
  const VALID_DIRS = new Set(["a", "b", "c"]);
  const VALID_SCREENS = new Set(["overview", "display", "ingest", "attempts"]);
  const params = new URLSearchParams(location.search);
  const cookieValue = document.cookie.split(";").map(v => v.trim()).find(v => v.startsWith("console_dir="))?.split("=")[1];
  const queryDir = params.get("dir");
  const direction = VALID_DIRS.has(queryDir) ? queryDir : (VALID_DIRS.has(cookieValue) ? cookieValue : "a");
  const screen = VALID_SCREENS.has(params.get("screen")) ? params.get("screen") : "overview";
  const names = { a: "Service Mode", b: "Preflight", c: "Flowpath" };
  const main = document.querySelector("main");
  const feedState = new window.ConsoleModel.FeedState(FEED_EXPECT_S);
  let model = null;
  let timer = null;

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
  const state = window.ConsoleModel.state;
  const glyph = value => ({ LIVE: "●", FAILING: "▲", UNKNOWN: "?", NOT_CONFIGURED: "—" })[state(value)];
  const statusCard = (title, status, reason, age, expectation, detail = "") => window.ConsoleModel.normalizeCard({ title, state: status, reason, age, expectation, detail });
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
    return health.data.sources.map(source => {
      // M1's current API contract is registry-left-joined health evidence.
      // Configuration declarations and high-water history do not exist in
      // that response yet; never invent unreachable fields here.
      return statusCard(source.name, source.state, source.reason, source.observation_age_s, sourceExpectation(source), own[source.name] || "Owns observations assigned to this registry source.");
    });
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
    cards.push(results.clock.ok ? window.ConsoleModel.clockCard(results.clock.data, `expect ≤ ${FEED_EXPECT_S}s`) : unavailable("Clock", results.clock));
    cards.push(...window.ConsoleModel.identityCards(results.version.ok ? results.version.data : null));
    const priority = { UNKNOWN: 0, FAILING: 1, NOT_CONFIGURED: 2, LIVE: 3 };
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
    const cards = results.attempts.data.map(row => window.ConsoleModel.attemptCard(row, isoAge(row.finished_at || row.started_at)));
    if (cards.length === 0) cards.push(statusCard("Attempt ledger", "UNKNOWN", "endpoint returned no attempt rows", 0, "expect when work is attempted"));
    return { screen: "attempts", eyebrow: "Durable evidence", title: "Attempt history", intro: "Actual rows from /api/attempts, newest first.", sections: [{ title: "Attempt ledger", cards }] };
  }

  async function loadModel() {
    const requests = { health: get("/api/health"), version: get("/api/version") };
    if (screen === "overview") { requests.host = get("/api/host"); requests.clock = get("/api/clock"); }
    if (screen === "display") requests.display = get("/api/display");
    if (screen === "attempts") requests.attempts = get("/api/attempts?limit=100");
    const keys = Object.keys(requests);
    const values = await Promise.all(Object.values(requests));
    const results = Object.fromEntries(keys.map((key, i) => [key, values[i]]));
    if (results.version.ok) document.querySelector("#build-id").textContent = results.version.data.git_commit !== "UNKNOWN" ? `Build: ${results.version.data.git_commit} · epoch ${results.version.data.build_epoch_s}` : "Build: UNKNOWN — commit/time not measured";
    const nextModel = ({ overview, display, ingest, attempts })[screen](results);
    const failureReason = results.health.ok ? null : results.health.error;
    feedState.observe(Date.now(), nextModel, results.health.ok, failureReason);
    return feedState.snapshot(Date.now()).model;
  }

  function render() {
    if (!model || !window.ConsoleDirections?.[direction]) return;
    main.innerHTML = window.ConsoleDirections[direction](model, { esc, ageText, glyph });
    tick();
  }

  function tick() {
    const snapshot = feedState.snapshot(Date.now());
    const elapsed = snapshot.elapsed;
    document.querySelector("#feed-age").textContent = ageText(elapsed);
    document.querySelectorAll("[data-age-base]").forEach(el => {
      // Number("") is 0 — an absent age must stay "not observed", never
      // become a fresh-looking "0s ago".
      const raw = el.dataset.ageBase;
      const base = raw === "" || raw == null ? NaN : Number(raw);
      const at = el.dataset.ageAt === "" || el.dataset.ageAt == null ? NaN : Number(el.dataset.ageAt);
      el.textContent = ageText(window.ConsoleModel.advancingAge(base, at, Date.now()));
    });
    window.ConsoleModel.applyFeedDom(snapshot, {
      feedAge: document.querySelector("#feed-age"),
      overlay: document.querySelector("#feed-overlay"),
      body: document.body,
      detail: document.querySelector("#feed-detail"),
      expectSeconds: FEED_EXPECT_S,
    }, ageText);
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
  timer = setInterval(() => { tick(); const snapshot = feedState.snapshot(Date.now()); if (snapshot.elapsed == null || snapshot.elapsed * 1000 >= REFRESH_MS) refresh(); }, 1000);
  window.addEventListener("pagehide", () => clearInterval(timer));
})();
