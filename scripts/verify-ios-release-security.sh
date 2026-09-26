#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

keychain=clients/apple/Packages/AWTransport/Sources/AWTransport/EnrollmentSecurity.swift
outbox=clients/apple/Packages/AWOutbox/Sources/AWOutbox/DurableOutbox.swift
privacy=clients/apple/AnxietyWatchApp/PrivacyInfo.xcprivacy
entitlements=clients/apple/AnxietyWatchApp/AnxietyWatchApp.entitlements
lock=clients/apple/Dependencies.lock.json

rg -q 'kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly' "$keychain" || {
  echo "release security verification failed: device-only Keychain protection is absent" >&2
  exit 1
}
rg -q 'completeUntilFirstUserAuthentication' "$outbox" || {
  echo "release security verification failed: outbox Data Protection is absent" >&2
  exit 1
}
rg -q 'isExcludedFromBackup' "$outbox" || {
  echo "release security verification failed: outbox backup exclusion is absent" >&2
  exit 1
}
rg -Uq '<key>NSPrivacyTracking</key>[[:space:]]*<false/>' "$privacy" || {
  echo "release security verification failed: privacy manifest tracking declaration is absent" >&2
  exit 1
}
rg -Uq '<key>com\.apple\.developer\.healthkit</key>[[:space:]]*<true/>' "$entitlements" || {
  echo "release security verification failed: iPhone HealthKit entitlement is absent" >&2
  exit 1
}
rg -q '"third_party_dependencies": \[\]' "$lock" || {
  echo "release security verification failed: dependency inventory is not empty and reviewed" >&2
  exit 1
}
if rg -n 'UserDefaults' clients/apple/Packages/AWTransport/Sources/AWTransport/EnrollmentSecurity.swift; then
  echo "release security verification failed: credentials must not use UserDefaults" >&2
  exit 1
fi
if rg -n 'NSAllowsArbitraryLoads|serverTrust|trustAll|\.p12|\.mobileprovision' clients/apple --glob '*.swift' --glob '*.plist' --glob '*.xcconfig' --glob '*.json'; then
  echo "release security verification failed: insecure platform setting, secret storage, or trust override found" >&2
  exit 1
fi

echo "iOS release security checks passed"
