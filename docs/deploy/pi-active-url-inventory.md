# Active Pi admin and monitoring URLs

**Evidence time:** 2026-09-28 09:33 UTC
**Host placeholder:** `$PI_HOST` means the Pi's configured private-LAN address. The literal address is intentionally not committed to the repository.

This is a read-only inventory of HTTP listeners and routes observed on the Pi. HTTP status codes below are observations at the evidence time, not permanent interface guarantees.

## Summary

| Base URL | Service | Bind scope | Role | Current state |
|---|---|---|---|---|
| `http://$PI_HOST:8480` | `vitals-console.service` | Pi private interface only | AnxietyWatch vitals dashboard, health, and authenticated administration | Active; HTTP only; dashboard and health routes respond |
| `http://$PI_HOST:8975` | `panel-http.service` | `0.0.0.0`, with private-LAN/loopback request allowlist | Bedside clinical panel | Active; panel root responds |
| `http://$PI_HOST:8976` | `panel-admin.service` | `0.0.0.0` | Aircraft/noise-watch administration and state | Active; no authentication implemented by this service |
| `http://$PI_HOST:80` | `lighttpd.service` | `0.0.0.0` | dump1090 aircraft map and static files | Active; map responds, aircraft JSON feeds currently return `404` |
| `http://127.0.0.1:631` | `cupsd` | Loopback only | Host print administration | Active locally; not reachable through the LAN address |

There is no HTTPS listener in this inventory. The vitals console explicitly runs with TLS termination disabled and binds only to the private interface.

## 1. Vitals console: port 8480

Service: `vitals-console.service`
Configured bind: the Pi's private address on port `8480`
Protocol: HTTP
Authentication: browser login/session for data-bearing administration routes; health and version are intentionally public.

### Browser pages

| URL | Purpose | Observed result |
|---|---|---:|
| `http://$PI_HOST:8480/console` | Console login/application shell | `200 text/html` |
| `http://$PI_HOST:8480/console/` | Console login/application shell | `200 text/html` |
| `http://$PI_HOST:8480/console/login` | Console login/application shell | `200 text/html` |
| `http://$PI_HOST:8480/dashboard` | Vitals dashboard | `200 text/html` |
| `http://$PI_HOST:8480/dashboard/` | Vitals dashboard | `200 text/html` |
| `http://$PI_HOST:8480/` | No root page is mounted | `404` |
| `http://$PI_HOST:8480/admin` | No standalone admin page is mounted | `404` |
| `http://$PI_HOST:8480/admin/` | No standalone admin page is mounted | `404` |

There is no `/admin` or `/admin/` page on the vitals console; both returned `404`. The app-wide operator entry point is `/console`, and the app-wide monitoring page is `/dashboard`. The dashboard HTML is reachable without a session, but its data-bearing API calls are protected separately.

### Public monitoring endpoints

| Method | URL | Purpose | Observed result |
|---|---|---|---:|
| `GET` | `http://$PI_HOST:8480/api/health` | Structured vitals/source health | `200 application/json` |
| `GET` | `http://$PI_HOST:8480/api/version` | Running console/API version | `200 application/json` |

### Session-protected monitoring endpoints

These routes are mounted and require the browser admin session. An unauthenticated `GET` returned `401 application/json` during this inventory.

| Method | URL | Purpose |
|---|---|---|
| `GET` | `http://$PI_HOST:8480/api/query` | Query interface |
| `GET` | `http://$PI_HOST:8480/api/latest` | Latest measurements |
| `GET` | `http://$PI_HOST:8480/api/nightly` | Nightly view/data |
| `GET` | `http://$PI_HOST:8480/api/services` | Service state |
| `GET` | `http://$PI_HOST:8480/api/host` | Host state/details |
| `GET` | `http://$PI_HOST:8480/api/display` | Display state |
| `GET` | `http://$PI_HOST:8480/api/recovery` | Recovery/deployment state |
| `GET` | `http://$PI_HOST:8480/api/clock` | Clock state |
| `GET` | `http://$PI_HOST:8480/api/attempts` | Attempt history |
| `GET` | `http://$PI_HOST:8480/api/admin/security` | Security/admin state |

### Session-protected administration endpoints

| Method | URL | Purpose |
|---|---|---|
| `POST` | `http://$PI_HOST:8480/api/admin/login` | Create the browser admin session; host validation applies |
| `POST` | `http://$PI_HOST:8480/api/admin/logout` | End the browser admin session |
| `POST` | `http://$PI_HOST:8480/api/health/mutes` | Change health mute state |
| `POST` | `http://$PI_HOST:8480/api/config/declarations` | Change source declarations |
| `POST` | `http://$PI_HOST:8480/api/config/overrides` | Change operator overrides |
| `POST` | `http://$PI_HOST:8480/api/services/:unit/:action` | Request a service operation |
| `POST` | `http://$PI_HOST:8480/api/envelope` | Submit an envelope through the console path |
| `POST` | `http://$PI_HOST:8480/api/admin/mobile/enrollment-artifacts` | Create mobile enrollment artifacts |
| `POST` | `http://$PI_HOST:8480/api/admin/notifications/apns/recipients` | Preview APNs recipients |
| `POST` | `http://$PI_HOST:8480/api/admin/notifications/apns/send` | Send an APNs notification |

The production process does not mount the plaintext test routes. `/test`, `/api/envelope/register`, and `/api/envelope/plaintext-test` are only mounted when the explicit test-route environment flag is enabled.

### Versioned client API

These are protocol endpoints for signed/mobile clients, not operator-facing monitoring pages:

| Method | URL |
|---|---|
| `GET` | `http://$PI_HOST:8480/api/v1/client/version` |
| `GET` | `http://$PI_HOST:8480/api/v1/client/openapi.json` |
| `GET` | `http://$PI_HOST:8480/api/v1/client/registry` |
| `POST` | `http://$PI_HOST:8480/api/v1/client/enroll` |
| `POST` | `http://$PI_HOST:8480/api/v1/client/items` |
| `POST` | `http://$PI_HOST:8480/api/v1/client/status` |
| `POST` | `http://$PI_HOST:8480/api/v1/client/notifications/apns/registration` |
| `POST` | `http://$PI_HOST:8480/api/v1/client/notifications/apns/outcomes` |
| `GET` | `http://$PI_HOST:8480/api/v1/client/projections` |

The client API uses its own device/request authentication contract where required; it is separate from the browser admin cookie.

## 2. Bedside clinical panel: port 8975

Service: `panel-http.service`
Process: `anxietywatch-panel-http.py`
Bind: `0.0.0.0:8975`
Request boundary: private-LAN and loopback allowlist; clients outside the allowlist are rejected.

| URL | Purpose | Observed result |
|---|---|---:|
| `http://$PI_HOST:8975/` | Bedside clinical panel, Direction A | `200 text/html` |
| `http://$PI_HOST:8975/index.html` | Same static panel | `200 text/html` |

This is a static panel server. The tested API-like paths (`/api/state`, `/api/health`, `/api/version`, and `/dashboard`) returned `404`; its live data is supplied through the panel's client-side integration rather than these routes.

## 3. Noise-watch administration: port 8976

Service: `panel-admin.service`
Process: `panel-admin`
Bind: `0.0.0.0:8976`
Authentication: **none implemented by this service**.

The service is LAN-facing because it binds all interfaces. Treat it as a private-LAN administrative surface, not an Internet-safe endpoint.

### Pages and read-only state

| Method | URL | Purpose | Observed result |
|---|---|---|---:|
| `GET` | `http://$PI_HOST:8976/` | Noise Watch admin page | `200 text/html` |
| `GET` | `http://$PI_HOST:8976/index.html` | Noise Watch admin page | `200 text/html` |
| `GET` | `http://$PI_HOST:8976/admin.html` | Noise Watch admin page | `200 text/html` |
| `GET` | `http://$PI_HOST:8976/settings` | Noise Watch settings page | `200 text/html` |
| `GET` | `http://$PI_HOST:8976/api/state` | Large combined aircraft/noise/settings state document | `200 application/json` |

### Mutating endpoints

| Method | URL | Purpose | Current route evidence |
|---|---|---|---|
| `POST` | `http://$PI_HOST:8976/api/settings` | Save settings | Route active; empty probe body returned `400` |
| `POST` | `http://$PI_HOST:8976/api/warning-review` | Save a warning review | Route active; empty probe body returned `400` |

The service returns `404` for unrelated paths and does not provide a login route.

## 4. dump1090 aircraft map: port 80

Service: `lighttpd.service`
Bind: `0.0.0.0:80`
Document root: `/var/www/html`

### Active map route

| URL | Purpose | Observed result |
|---|---|---:|
| `http://$PI_HOST/dump1090/` | Redirect to the aircraft map | `301` to `/dump1090/gmap.html` |
| `http://$PI_HOST/dump1090/gmap.html` | dump1090 aircraft map | `200 text/html` |

The server root `http://$PI_HOST/` returned `403`; it is not an active application landing page.

### Configured but currently unavailable data routes

Lighttpd is configured to alias these dump1090 data paths:

- `http://$PI_HOST/dump1090/data/aircraft.json`
- `http://$PI_HOST/dump1090/data/receiver.json`
- `http://$PI_HOST/dump1090/data/stats.json`

At evidence time, the corresponding runtime directory contained no files and all three requests returned `404`. Therefore the map HTML is active, but live aircraft JSON data was not available in this snapshot.

## 5. Local-only CUPS administration

CUPS was listening on loopback only:

- `http://127.0.0.1:631/` — CUPS home, observed `200`
- `http://127.0.0.1:631/admin/` — CUPS administration, observed `200`

These are not reachable through `$PI_HOST:631` and are not AnxietyWatch application URLs.

## Not HTTP URLs

The following active control/data surfaces appeared in the socket inventory but are not URLs:

- `/run/vitals/vitalsd.sock` — local Unix socket used by the vitals console and services.
- PostgreSQL on `127.0.0.1:5432` — local database listener, not LAN-facing HTTP.
- SSH on port `22` — administrative shell access, not an application URL.
- Tailscale and mDNS listeners — network/discovery services, not AnxietyWatch admin pages.

## Recheck commands

From an operator workstation, substitute the Pi's private address for `$PI_HOST`:

```sh
PI_HOST='private-address-of-pi'
curl -I "http://$PI_HOST:8480/dashboard"
curl -I "http://$PI_HOST:8975/"
curl -I "http://$PI_HOST:8976/"
curl -I "http://$PI_HOST/dump1090/"
```

For the full process/journal view, use the installed helper:

```sh
ssh -t <pi-alias> 'sudo /usr/local/sbin/vitals-log-follow'
```

The URL inventory should be refreshed after bind-address, reverse-proxy, authentication, or service changes.
