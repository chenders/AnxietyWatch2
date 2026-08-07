/// Static HTML test page for the envelope API.
///
/// This page is deliberately not part of the console (spec §9, M0 criterion 7
/// footnote). It provides a form that sends a plaintext domain record via the
/// server-side `/api/envelope/plaintext-test` route, which auto-generates
/// credentials, registers the client, encrypts the payload, and submits it.
///
/// The server-side route was chosen over the client-side JS alternative
/// because it avoids bundling a NaCl secretbox implementation in the page.
pub const TEST_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Envelope API Test</title>
<style>
  body { font-family: system-ui, sans-serif; max-width: 640px; margin: 2em auto; padding: 0 1em; }
  h1 { font-size: 1.25em; }
  form { display: flex; flex-direction: column; gap: 0.75em; }
  label { font-weight: 600; font-size: 0.875em; }
  input, textarea { font-family: monospace; padding: 0.5em; border: 1px solid #ccc; border-radius: 4px; }
  textarea { min-height: 6em; }
  button { padding: 0.5em 1em; background: #2563eb; color: #fff; border: none; border-radius: 4px; cursor: pointer; }
  button:hover { background: #1d4ed8; }
  pre { background: #f5f5f5; padding: 1em; border-radius: 4px; overflow-x: auto; font-size: 0.875em; }
  .note { color: #666; font-size: 0.875em; }
</style>
</head>
<body>
<h1>Envelope API — plaintext test</h1>
<p class="note">
  This page auto-generates a client_id + secret, registers the client,
  encrypts the domain record with XSalsa20-Poly1305, and sends it through the
  envelope endpoint. Idempotency and attempt records are exercised.
  Requires <code>VITALS_CONSOLE_TEST_ROUTES=1</code>.
</p>

<form id="test-form">
  <label for="kind">Kind</label>
  <input id="kind" name="kind" value="test_entry" required>

  <label for="content">Content (JSON)</label>
  <textarea id="content" name="content" required>{"message": "hello from test page", "n": 1}</textarea>

  <button type="submit">Send</button>
</form>

<pre id="result" style="display:none;"></pre>

<script>
document.getElementById('test-form').addEventListener('submit', async (e) => {
  e.preventDefault();
  const resultEl = document.getElementById('result');
  resultEl.style.display = 'block';
  resultEl.textContent = 'Sending...';

  let content;
  try {
    content = JSON.parse(document.getElementById('content').value);
  } catch (err) {
    resultEl.textContent = 'Error: content is not valid JSON: ' + err.message;
    return;
  }

  try {
    const resp = await fetch('/api/envelope/plaintext-test', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        kind: document.getElementById('kind').value,
        content: content,
      }),
    });
    const body = await resp.json();
    resultEl.textContent = 'Status ' + resp.status + '\n' + JSON.stringify(body, null, 2);
  } catch (err) {
    resultEl.textContent = 'Error: ' + err.message;
  }
});
</script>
</body>
</html>
"#;
