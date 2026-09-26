#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
committed=api/openapi/client-v1.json
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

cargo run --quiet -p vitals-console --bin generate_client_openapi >"$tmp"
if ! cmp -s "$committed" "$tmp"; then
  echo "openapi: FAIL — $committed is stale; regenerate with generate_client_openapi" >&2
  diff -u "$committed" "$tmp" || true
  exit 1
fi

python3 - <<'PY'
import json
from pathlib import Path

openapi_path = Path("api/openapi/client-v1.json")
doc = json.loads(openapi_path.read_text(encoding="utf-8"))
expected_paths = {
    "/api/v1/client/enroll",
    "/api/v1/client/items",
    "/api/v1/client/status",
    "/api/v1/client/openapi.json",
    "/api/v1/client/projections",
    "/api/v1/client/registry",
    "/api/v1/client/version",
    "/api/v1/client/notifications/apns/registration",
    "/api/v1/client/notifications/apns/outcomes",
}
if set(doc.get("paths", {})) != expected_paths:
    raise SystemExit("openapi: FAIL — explicit client path set changed")
if doc.get("openapi") != "3.1.0":
    raise SystemExit("openapi: FAIL — expected OpenAPI 3.1.0")

implemented = {
    "/api/v1/client/version",
    "/api/v1/client/registry",
    "/api/v1/client/openapi.json",
    "/api/v1/client/enroll",
    "/api/v1/client/items",
    "/api/v1/client/status",
    "/api/v1/client/notifications/apns/registration",
    "/api/v1/client/notifications/apns/outcomes",
    "/api/v1/client/projections",
}
gated = expected_paths - implemented
for path in implemented:
    statuses = {
        status
        for operation in doc["paths"][path].values()
        for status in operation.get("responses", {})
    }
    expected_success = "201" if path == "/api/v1/client/enroll" else "200"
    if expected_success not in statuses:
        raise SystemExit(
            f"openapi: FAIL — implemented operation has no {expected_success}: {path}"
        )
for path in gated:
    statuses = {
        status
        for operation in doc["paths"][path].values()
        for status in operation.get("responses", {})
    }
    if any(status.startswith("2") for status in statuses) or "503" not in statuses:
        raise SystemExit(f"openapi: FAIL — gated operation advertises success or lacks 503: {path}")

signed_paths = {
    "/api/v1/client/items",
    "/api/v1/client/status",
    "/api/v1/client/projections",
    "/api/v1/client/notifications/apns/registration",
    "/api/v1/client/notifications/apns/outcomes",
}
required_signature_headers = {
    "X-AnxietyWatch-API-Version",
    "X-AnxietyWatch-Client-ID",
    "X-AnxietyWatch-Key-ID",
    "X-AnxietyWatch-Request-ID",
    "Signature-Input",
    "Signature",
}
for path in signed_paths:
    operation = next(iter(doc["paths"][path].values()))
    header_names = {
        parameter["name"]
        for parameter in operation.get("parameters", [])
        if parameter.get("in") == "header"
    }
    if not required_signature_headers.issubset(header_names):
        raise SystemExit(
            f"openapi: FAIL — signed path lacks Amendment 09 headers: {path}: "
            f"{sorted(required_signature_headers - header_names)}"
        )
    statuses = set(operation.get("responses", {}))
    if not {"401", "403", "409"}.issubset(statuses):
        raise SystemExit(f"openapi: FAIL — signed path lacks stable auth/replay errors: {path}")

schemas = doc.get("components", {}).get("schemas", {})
required_schemas = {
    "ClientAuthHeaders",
    "ClientErrorResponse",
    "ClientWriteBatch",
    "ClientWriteResponse",
    "EnrollmentClaimRequest",
    "EnrollmentCredentialResponse",
    "HealthBeaconRequest",
    "ApnsRegistrationRequest",
    "ApnsRegistrationResponse",
    "ApnsOutcomeRequest",
    "ApnsOutcomeResponse",
    "ItemAcknowledgement",
    "ProjectionPage",
    "RegistryDiscoveryResponse",
}
if not required_schemas.issubset(schemas):
    raise SystemExit(f"openapi: FAIL — missing schemas: {sorted(required_schemas - set(schemas))}")
if any(name.startswith(("VitalsType", "VitalsSource")) for name in schemas):
    raise SystemExit("openapi: FAIL — physiological registry enum leaked into OpenAPI schemas")

registry = json.loads(Path("vitals/generated/registry.json").read_text(encoding="utf-8"))
registry_names = set(registry["type"]) | set(registry["source"])

def enum_values(value):
    if isinstance(value, dict):
        for key, nested in value.items():
            if key == "enum" and isinstance(nested, list):
                yield from (item for item in nested if isinstance(item, str))
            yield from enum_values(nested)
    elif isinstance(value, list):
        for nested in value:
            yield from enum_values(nested)

leaked = registry_names & set(enum_values(doc))
if leaked:
    raise SystemExit(f"openapi: FAIL — registry-owned values emitted as OpenAPI enums: {sorted(leaked)}")

fixture_dir = Path("api/fixtures/client-v1")
expected_fixtures = {
    "enrollment-request.json",
    "error-authentication-required.json",
    "error-request-replayed.json",
    "error-unavailable.json",
    "error-version-incompatible.json",
    "health-beacon-background-delivery.json",
    "health-beacon-denied-type.json",
    "health-beacon-locked.json",
    "health-beacon-no-wake.json",
    "health-beacon-reboot.json",
    "health-beacon-request.json",
    "health-beacon-stale.json",
    "health-beacon-unknown.json",
    "healthkit-batch-request.json",
    "projection-response.json",
    "write-batch-ack-response.json",
    "write-batch-request.json",
    "apns-registration-request.json",
    "apns-registration-response.json",
    "apns-outcome-request.json",
    "apns-outcome-response.json",
}
actual_fixtures = {path.name for path in fixture_dir.glob("*.json")}
if actual_fixtures != expected_fixtures:
    raise SystemExit("openapi: FAIL — shared fixture set changed without verifier update")
for path in fixture_dir.glob("*.json"):
    text = path.read_text(encoding="utf-8")
    json.loads(text)
    lowered = text.lower()
    for forbidden in ("password", "bearer ", "https://", "/users/", "private/var/mobile"):
        if forbidden in lowered:
            raise SystemExit(f"openapi: FAIL — non-synthetic fixture material in {path}: {forbidden}")
print("openapi: contract paths, gated statuses, schemas, registry boundary, and fixtures verified")
PY

swift test --package-path clients/apple/Packages/AWTransport >/dev/null
printf 'openapi: PASS — generated document is current and shared Rust/Swift fixtures pass\n'
