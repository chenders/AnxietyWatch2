#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
source_file=clients/apple/Packages/AWTransport/Sources/AWTransport/EnrollmentSecurity.swift
app_file=clients/apple/AnxietyWatchApp/ClientRootView.swift

for required in SecureEnclaveKeyStore KeychainCredentialStore EnrollmentCoordinator EnrollmentConfirmation; do
  rg -q "\b${required}\b" "$source_file" || {
    echo "auth verification failed: missing ${required} implementation in ${source_file}" >&2
    exit 1
  }
done
rg -q 'kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly' "$source_file" || {
  echo "auth verification failed: device-bound after-first-unlock Keychain protection is absent" >&2
  exit 1
}
rg -q 'SecureEnclave\.P256\.Signing\.PrivateKey' "$source_file" || {
  echo "auth verification failed: Secure Enclave P-256 key generation is absent" >&2
  exit 1
}
if rg -q 'UserDefaults|bearer|trustAll|NSAllowsArbitraryLoads' clients/apple/AnxietyWatchApp clients/apple/Packages; then
  echo "auth verification failed: forbidden credential or trust shortcut found in Apple client code" >&2
  exit 1
fi
rg -q 'Start recovery|not enrolled' "$app_file" || {
  echo "auth verification failed: enrollment/recovery UI is absent from the iPhone root" >&2
  exit 1
}
echo "iOS authentication checks passed"
