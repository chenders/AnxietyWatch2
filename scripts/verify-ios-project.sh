#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
apple=clients/apple
project="$apple/AnxietyWatch.xcodeproj/project.pbxproj"

required=(
  "$project"
  "$apple/AnxietyWatch.xcodeproj/xcshareddata/xcschemes/AnxietyWatch.xcscheme"
  "$apple/AnxietyWatch.xcodeproj/xcshareddata/xcschemes/AnxietyWatch-CI.xcscheme"
  "$apple/AnxietyWatch.xcodeproj/xcshareddata/xcschemes/AnxietyWatchWatch.xcscheme"
  "$apple/Config/Shared.xcconfig"
  "$apple/Config/iOS.xcconfig"
  "$apple/Config/watchOS.xcconfig"
  "$apple/AnxietyWatchApp/PrivacyInfo.xcprivacy"
  "$apple/AnxietyWatchWatch/PrivacyInfo.xcprivacy"
  "$apple/Dependencies.lock.json"
  "$apple/Entitlements.allowlist.json"
  "$apple/PROVENANCE.md"
  "$apple/SUPPORT.md"
  "$apple/Fixtures/Public/skeleton-state.json"
)
for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "ios-project: FAIL — missing $path" >&2
    exit 1
  fi
done

plutil -lint "$project" \
  "$apple/AnxietyWatchApp/PrivacyInfo.xcprivacy" \
  "$apple/AnxietyWatchWatch/PrivacyInfo.xcprivacy" \
  "$apple/AnxietyWatchApp/AnxietyWatchApp.entitlements" \
  "$apple/AnxietyWatchWatch/AnxietyWatchWatch.entitlements" >/dev/null

for target in AnxietyWatchApp AnxietyWatchWatch AnxietyWatchAppTests; do
  if ! grep -q "name = $target;" "$project"; then
    echo "ios-project: FAIL — Xcode target missing: $target" >&2
    exit 1
  fi
done
if ! grep -q $'^\tobjectVersion = 77;$' "$project" ||
  ! grep -q $'^\t\t\tpreferredProjectObjectVersion = 77;$' "$project"; then
  echo "ios-project: FAIL — project and preferred object versions must both be 77" >&2
  exit 1
fi
if rg -n 'Platforms/.+SDKs/.+\.sdk|sourceTree = DEVELOPER_DIR' "$project"; then
  echo "ios-project: FAIL — system frameworks must resolve relative to the active SDK" >&2
  exit 1
fi
if [[ $(grep -c 'path = System/Library/Frameworks/Foundation.framework; sourceTree = SDKROOT;' "$project") -ne 2 ]]; then
  echo "ios-project: FAIL — iOS and watchOS Foundation references must use SDKROOT" >&2
  exit 1
fi

packages=(AWClientCore AWTransport AWOutbox AWHealthKit AWDomain)
for package in "${packages[@]}"; do
  manifest="$apple/Packages/$package/Package.swift"
  if [[ ! -f "$manifest" ]] || ! grep -q "name: \"$package\"" "$manifest"; then
    echo "ios-project: FAIL — package manifest missing or mislabeled: $package" >&2
    exit 1
  fi
  if ! grep -q "relativePath = Packages/$package;" "$project"; then
    echo "ios-project: FAIL — Xcode project does not reference local package: $package" >&2
    exit 1
  fi
  swift package --package-path "$apple/Packages/$package" dump-package >/dev/null
done

if ! grep -qx 'IPHONEOS_DEPLOYMENT_TARGET = 18.0' "$apple/Config/iOS.xcconfig"; then
  echo "ios-project: FAIL — iPhone deployment target must remain 18.0" >&2
  exit 1
fi
if ! grep -qx 'WATCHOS_DEPLOYMENT_TARGET = 11.0' "$apple/Config/watchOS.xcconfig"; then
  echo "ios-project: FAIL — watch deployment target must remain 11.0" >&2
  exit 1
fi
if grep -q 'TARGETED_DEVICE_FAMILY = "1,2"' "$project" || \
   ! grep -qx 'TARGETED_DEVICE_FAMILY = 1' "$apple/Config/iOS.xcconfig" || \
   ! grep -qx 'TARGETED_DEVICE_FAMILY = 4' "$apple/Config/watchOS.xcconfig"; then
  echo "ios-project: FAIL — device families must be iPhone=1 and Watch=4" >&2
  exit 1
fi

if rg -n '^\s*import (SwiftUI|WatchConnectivity|SwiftData|CoreData|CoreBluetooth|SQLite3)\b' \
    "$apple/Packages" --glob '*.swift'; then
  echo "ios-project: FAIL — pure package imports an Apple UI/device/persistence framework" >&2
  exit 1
fi
if rg -n '^\s*import HealthKit\b' \
    "$apple/Packages/AWClientCore" "$apple/Packages/AWTransport" "$apple/Packages/AWOutbox" \
    "$apple/Packages/AWDomain" --glob '*.swift'; then
  echo "ios-project: FAIL — a non-HealthKit package imports HealthKit" >&2
  exit 1
fi
if rg -n '\.package\s*\(\s*url:' "$apple/Packages" --glob 'Package.swift'; then
  echo "ios-project: FAIL — unreviewed remote Swift dependency found" >&2
  exit 1
fi
if rg -n '\b(CoreBluetooth|CBPeripheral|Polar|EMAY|Oura|SwiftData|SQLite3|HealthSnapshot|ReportGenerator|HLC)\b' \
    "$apple/AnxietyWatchApp" "$apple/AnxietyWatchWatch" --glob '*.swift'; then
  echo "ios-project: FAIL — forbidden generation-one BLE/analysis/storage code compiled" >&2
  exit 1
fi
if rg -n 'NSAllowsArbitraryLoads|NSBluetooth|NSLocation|NSCamera|NSMicrophone|NSUserTracking' "$apple" \
    --glob '*.xcconfig' --glob '*.plist' --glob '*.pbxproj'; then
  echo "ios-project: FAIL — skeleton requests a forbidden network or privacy capability" >&2
  exit 1
fi

python3 - "$apple" <<'PY'
import json
import plistlib
import sys
from pathlib import Path

apple = Path(sys.argv[1])
expected_entitlements = {
    "AnxietyWatchApp/AnxietyWatchApp.entitlements": {"com.apple.developer.healthkit": True},
    "AnxietyWatchWatch/AnxietyWatchWatch.entitlements": {},
}
for relative in expected_entitlements:
    with (apple / relative).open("rb") as handle:
        entitlements = plistlib.load(handle)
    if entitlements != expected_entitlements[relative]:
        raise SystemExit(f"ios-project: FAIL — unexpected entitlements: {relative}")
for relative in (
    "AnxietyWatchApp/PrivacyInfo.xcprivacy",
    "AnxietyWatchWatch/PrivacyInfo.xcprivacy",
):
    with (apple / relative).open("rb") as handle:
        manifest = plistlib.load(handle)
    expected = {
        "NSPrivacyAccessedAPITypes": [],
        "NSPrivacyCollectedDataTypes": [],
        "NSPrivacyTracking": False,
        "NSPrivacyTrackingDomains": [],
    }
    if manifest != expected:
        raise SystemExit(f"ios-project: FAIL — unexpected privacy declaration: {relative}")
lock = json.loads((apple / "Dependencies.lock.json").read_text())
if lock.get("schema_version") != 1 or lock.get("third_party_dependencies") != []:
    raise SystemExit("ios-project: FAIL — dependency lock must explicitly contain no third-party code")
allowlist = json.loads((apple / "Entitlements.allowlist.json").read_text())
if allowlist.get("targets") != {
    "AnxietyWatchApp": ["com.apple.developer.healthkit"],
    "AnxietyWatchWatch": [],
}:
    raise SystemExit("ios-project: FAIL — entitlement allowlist differs from HealthKit-only target entitlements")
fixture = json.loads((apple / "Fixtures/Public/skeleton-state.json").read_text())
if fixture.get("fixture") is not True or fixture.get("contains_personal_data") is not False:
    raise SystemExit("ios-project: FAIL — public skeleton fixture is not explicitly synthetic")
PY

printf 'ios-project: PASS — iPhone/watch targets, 5 packages, schemes, privacy, support, and HealthKit-only capabilities verified\n'
