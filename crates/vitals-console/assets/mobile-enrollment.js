(() => {
  "use strict";

  const loginForm = document.querySelector("#login-form");
  const artifactPanel = document.querySelector("#artifact-panel");
  const artifactForm = document.querySelector("#artifact-form");
  const resultPanel = document.querySelector("#artifact-result");
  const artifactJSON = document.querySelector("#artifact-json");
  const status = document.querySelector("#status");
  let csrfToken = null;

  const setStatus = message => { status.textContent = message; };
  const responseBody = async response => response.json().catch(() => ({}));

  loginForm.addEventListener("submit", async event => {
    event.preventDefault();
    setStatus("Signing in…");
    const password = document.querySelector("#admin-password").value;
    try {
      const response = await fetch("/api/admin/login", {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        credentials: "same-origin",
        cache: "no-store",
        body: JSON.stringify({ password })
      });
      const body = await responseBody(response);
      if (!response.ok || typeof body.csrf_token !== "string") {
        throw new Error(body.error || `login failed with HTTP ${response.status}`);
      }
      csrfToken = body.csrf_token;
      document.querySelector("#admin-password").value = "";
      loginForm.hidden = true;
      artifactPanel.hidden = false;
      setStatus("Signed in. Create the one-time artifact now; recent authentication expires after five minutes.");
    } catch (error) {
      setStatus(`Sign-in blocked: ${error.message}`);
    }
  });

  artifactForm.addEventListener("submit", async event => {
    event.preventDefault();
    if (!csrfToken) {
      setStatus("Artifact creation blocked: operator authentication is missing.");
      return;
    }
    const scopes = [...document.querySelectorAll('input[name="scope"]:checked')].map(input => input.value);
    if (scopes.length === 0) {
      setStatus("Artifact creation blocked: select at least one capability.");
      return;
    }
    const deviceLabel = document.querySelector("#device-label").value.trim();
    setStatus("Creating one-time artifact…");
    try {
      const response = await fetch("/api/admin/mobile/enrollment-artifacts", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
          "x-anxietywatch-csrf": csrfToken
        },
        credentials: "same-origin",
        cache: "no-store",
        body: JSON.stringify({ device_label: deviceLabel, scopes })
      });
      const body = await responseBody(response);
      if (!response.ok || typeof body.token !== "string") {
        throw new Error(body.error || `artifact creation failed with HTTP ${response.status}`);
      }
      artifactJSON.value = JSON.stringify(body, null, 2);
      resultPanel.hidden = false;
      artifactForm.hidden = true;
      setStatus("Artifact created. Copy it into AnxietyWatch now; it expires in ten minutes and is single-use.");
      resultPanel.scrollIntoView({ behavior: "smooth", block: "start" });
    } catch (error) {
      setStatus(`Artifact creation blocked: ${error.message}`);
    }
  });

  document.querySelector("#copy-artifact").addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(artifactJSON.value);
      setStatus("Artifact copied. Paste it into AnxietyWatch; do not place it in logs or chat.");
    } catch {
      artifactJSON.focus();
      artifactJSON.select();
      setStatus("Clipboard access was unavailable; the artifact is selected for manual copy.");
    }
  });

  document.querySelector("#clear-artifact").addEventListener("click", () => {
    artifactJSON.value = "";
    resultPanel.hidden = true;
    setStatus("Artifact cleared from this page.");
  });

  window.addEventListener("pagehide", () => { artifactJSON.value = ""; });
})();
