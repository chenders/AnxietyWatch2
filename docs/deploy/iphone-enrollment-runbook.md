# Enroll an iPhone with AnxietyWatch2

This is the plain-English operator procedure for connecting one iPhone to the AnxietyWatch2 Pi.

## What password is required?

There is **no separate phone-enrollment password** and no documented default password.

Use the existing **AnxietyWatch2 console admin password** at the first step. The server stores only an Argon2id password hash in its root-only configuration; the plaintext password cannot be recovered from the Pi or from this repository. Do not put the password in this document, chat, email, logs, screenshots, or tickets.

If the admin password is unknown, stop and have the authorized operator reset or retrieve it through the approved password-management process. Do not guess repeatedly: login attempts are throttled.

## Before you begin

- The iPhone and Pi must be able to reach the same private network/origin.
- Use the Pi's configured private-LAN URL, represented below as `$PI_HOST`.
- Confirm that the URL's host name matches the server identity you expect.
- Use the current AnxietyWatch2 iPhone app with the enrollment screen available.
- Have a safe way to transfer one sensitive JSON artifact directly into the phone. Do not use chat, email, a shared note, or a ticket.

The current console uses HTTP on the private LAN rather than public HTTPS. Do not perform this procedure over an untrusted or public network.

## Part 1: Create the one-time artifact on the Pi

### 1. Open the enrollment page

Open:

```text
http://$PI_HOST:8480/console/mobile-enrollment.html
```

You can also open the console at `http://$PI_HOST:8480/console/` and select **Mobile enrollment**.

This is the AnxietyWatch2 mobile-enrollment page. It is separate from the legacy Noise Watch admin page on port 8976.

### 2. Sign in with the console admin password

On **Operator authentication**:

1. Enter the existing console admin password.
2. Select **Sign in**.

What happens:

- The browser sends the password only to the same-origin console login endpoint.
- The server creates a short-lived admin session and CSRF token.
- The password field is cleared after successful login.
- The successful login starts the five-minute recent-authentication window required for enrollment-artifact creation.

If the page says that recent authentication is required, sign in again and create the artifact promptly.

### 3. Choose the phone label and capabilities

After login, the **Create enrollment artifact** section appears.

Set a useful **Device label**, such as `Personal iPhone` or another non-sensitive operator label.

Select only the capabilities this phone needs:

- **Submit HealthKit source samples** — permits phone HealthKit samples to be submitted.
- **Submit journal records** — permits journal records to be submitted.
- **Read projections** — permits the phone to read server projections.
- **Read registry and API compatibility** — permits API/registry discovery.
- **Read this device's credential status** — permits device credential-status checks.
- **Rotate this device's key** — permits the phone's supported key-rotation flow.
- **Receive operator-directed notifications** — permits the notification path; leave this off unless it is needed.

The form requires at least one capability. Prefer the smallest practical set.

### 4. Generate the artifact

Select **Generate one-time artifact**.

The server creates:

- a random enrollment identifier;
- a random opaque enrollment token;
- the configured server identity and origin;
- the selected capability scopes;
- a ten-minute expiration time.

The token is shown as plaintext only in the successful response and in the page's textarea. The server persists only a hash of the token. The page clears the artifact when it is left, and the artifact is not reusable.

### 5. Copy the complete JSON once

Select **Copy artifact JSON** and transfer the complete JSON directly to the iPhone enrollment screen.

Treat this JSON like a one-time credential:

- Do not paste it into chat, email, logs, screenshots, or tickets.
- Do not save it in a shared clipboard/history tool.
- Do not leave it visible after transfer.
- If it is exposed, discard it and create a fresh artifact.

## Part 2: Claim the artifact on the iPhone

### 6. Open the iPhone enrollment screen

In the AnxietyWatch2 iPhone app:

1. Open the **Settings** tab.
2. Find the **Pi connection** section.
3. Confirm that the device currently says **Not enrolled**.
4. Paste the complete one-time artifact JSON into **Enrollment artifact JSON**.
5. Set the device label if needed.

The app parses the artifact and displays the server label, server origin, and expiration time.

### 7. Check the server identity and origin

Before continuing, compare the displayed server identity and origin with the Pi you intended to enroll.

Turn on:

> I confirm this server identity and origin

Do not continue if the origin, host, server label, or expiration is unexpected. Create a fresh artifact from the intended private console instead.

### 8. Enroll the iPhone

Select **Enroll this iPhone**.

The iPhone then:

1. Creates a new device signing key.
2. Keeps the private key in the device's Secure Enclave/key storage.
3. Sends the one-time artifact claim to the configured server origin.
4. Calls the versioned enrollment endpoint:
   `POST http://$PI_HOST:8480/api/v1/client/enroll`
5. Receives a scoped device credential if the claim is accepted.
6. Stores the credential in Keychain.
7. Clears the pasted artifact from the enrollment form.

The server atomically checks the artifact's identity, expiry, token, API major, key format, and scopes. It then consumes the artifact and records the device credential and audit event. A second claim with the same artifact is rejected.

### 9. Confirm success

The iPhone should show:

> Enrollment succeeded; this iPhone can now authenticate queued writes.

The Settings screen should identify the device as enrolled. The phone can then use its scoped credential for the capabilities selected in Part 1.

If operator notifications were selected and are wanted, register them separately from the enrolled-device notification control. Enrollment itself does not silently grant additional scopes.

## Timing and limits

- The admin recent-authentication window is **five minutes**.
- Each enrollment artifact is valid for **ten minutes**.
- Each artifact is **single-use**.
- The server permits at most **three live unconsumed artifacts** at once.
- The device credential has its own lifecycle and can be rotated or revoked through the supported credential controls.

## Common failures

| Message or symptom | Meaning | Action |
|---|---|---|
| Login fails | The admin password is wrong, the host is not accepted, or login throttling is active | Stop guessing; verify the console URL/host and use the authorized password process |
| Recent authentication required | More than five minutes passed after login | Sign in again and generate a new artifact |
| Artifact expired | More than ten minutes passed | Generate a fresh artifact |
| Artifact already consumed | The one-time artifact was already claimed | Generate a fresh artifact; do not reuse the old JSON |
| Server identity/origin mismatch | The phone is not connecting to the intended console origin | Stop and verify the URL and private network; do not override the confirmation |
| No capabilities selected | The server refuses an empty scope set | Select the minimum required capabilities and generate again |
| Enrollment database unavailable | The Pi cannot complete the database transaction | Check the vitals-console and PostgreSQL journals before retrying |
| Enrollment succeeds but writes do not appear | Enrollment worked, but a selected scope, transport, or downstream health path may be failing | Check the phone's sync status and the combined Pi service journal |

## Useful verification commands

Follow the combined Pi service log:

```sh
ssh -t <pi-alias> 'sudo /usr/local/sbin/vitals-log-follow'
```

Inspect only the console journal:

```sh
ssh <pi-alias> 'sudo journalctl --since "15 minutes ago" -u vitals-console.service -o short-iso-precise --no-pager'
```

Check the public console health/version endpoints without credentials:

```sh
PI_HOST='private-address-of-pi'
curl -fsS "http://$PI_HOST:8480/api/health"
curl -fsS "http://$PI_HOST:8480/api/version"
```

Do not put the enrollment JSON, admin password, session cookie, CSRF token, private key, or device credential into any of these logs or command histories.
