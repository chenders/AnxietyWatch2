(() => {
  "use strict";
  const loginPanel = document.querySelector("#login-panel");
  const sendPanel = document.querySelector("#send-panel");
  const status = document.querySelector("#status");
  let csrfToken = null;
  const makeIdempotencyKey = () => {
    const bytes = new Uint8Array(16);
    if (globalThis.crypto && typeof globalThis.crypto.getRandomValues === "function") {
      globalThis.crypto.getRandomValues(bytes);
    } else {
      for (let index = 0; index < bytes.length; index += 1) bytes[index] = Math.floor(Math.random() * 256);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    const hex = Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
  };
  const body = response => response.json().catch(() => ({}));
  document.querySelector("#login-form").addEventListener("submit", async event => {
    event.preventDefault(); status.textContent = "Signing in…";
    try {
      const response = await fetch("/api/admin/login", { method: "POST", headers: { "Content-Type": "application/json" }, credentials: "same-origin", body: JSON.stringify({ password: document.querySelector("#password").value }) });
      const result = await body(response);
      if (!response.ok || typeof result.csrf_token !== "string") throw new Error(result.error || `HTTP ${response.status}`);
      csrfToken = result.csrf_token; document.querySelector("#password").value = ""; loginPanel.hidden = true; sendPanel.hidden = false;
      status.textContent = "Authenticated. Measuring recipients…";
      await previewRecipients();
    } catch (error) { status.textContent = `Authentication blocked: ${error.message}`; }
  });
  const previewRecipients = async () => {
    if (!csrfToken) return;
    const scope = document.querySelector("#recipient-scope").value;
    const request = { recipient_scope: scope, device_id: scope === "device" ? document.querySelector("#device-id").value.trim() : null, user_id: scope === "user" ? document.querySelector("#user-id").value.trim() : null, payload_class: "preview", category: null, deep_link: null, is_test: true };
    const response = await fetch("/api/admin/notifications/apns/recipients", { method: "POST", headers: { "Content-Type": "application/json", "x-anxietywatch-csrf": csrfToken }, credentials: "same-origin", body: JSON.stringify(request) });
    const result = await body(response);
    if (!response.ok) throw new Error(result.error || `HTTP ${response.status}`);
    document.querySelector("#confirmed-count").value = result.recipient_count;
    status.textContent = `Measured recipients: ${result.recipient_count}. Confirm this count before sending.`;
  };
  for (const selector of ["#recipient-scope", "#device-id", "#user-id"]) document.querySelector(selector).addEventListener("change", () => previewRecipients().catch(error => { status.textContent = `Recipient preview blocked: ${error.message}`; }));
  document.querySelector("#send-form").addEventListener("submit", async event => {
    event.preventDefault(); if (!csrfToken) { status.textContent = "Send blocked: operator authentication is missing."; return; }
    const scope = document.querySelector("#recipient-scope").value;
    const request = { recipient_scope: scope, idempotency_key: makeIdempotencyKey(), device_id: scope === "device" ? document.querySelector("#device-id").value.trim() : null, user_id: scope === "user" ? document.querySelector("#user-id").value.trim() : null, confirmed_recipient_count: Number(document.querySelector("#confirmed-count").value), payload_class: document.querySelector("#payload-class").value.trim(), category: document.querySelector("#category").value.trim() || null, deep_link: null, is_test: document.querySelector("#is-test").checked };
    status.textContent = "Sending…";
    try {
      const response = await fetch("/api/admin/notifications/apns/send", { method: "POST", headers: { "Content-Type": "application/json", "x-anxietywatch-csrf": csrfToken }, credentials: "same-origin", body: JSON.stringify(request) });
      const result = await body(response);
      if (!response.ok) throw new Error(result.error || `HTTP ${response.status}`);
      status.textContent = `Send result: ${result.status}; accepted=${result.accepted_count}/${result.recipient_count}; message=${result.message_id}`;
    } catch (error) { status.textContent = `Send blocked: ${error.message}`; }
  });
})();
