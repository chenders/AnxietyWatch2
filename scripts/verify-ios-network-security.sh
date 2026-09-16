#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
source_file=clients/apple/Packages/AWTransport/Sources/AWTransport/EnrollmentSecurity.swift
config=clients/apple/Config/iOS.xcconfig

rg -q 'url\.scheme\?\.lowercased\(\) == "https"' "$source_file" || {
  echo "network verification failed: HTTPS origin validation is absent" >&2
  exit 1
}
rg -q 'url\.port == nil \|\| url\.port == 443' "$source_file" || {
  echo "network verification failed: port 443 validation is absent" >&2
  exit 1
}
rg -q 'RedirectRejectingDelegate' "$source_file" || {
  echo "network verification failed: enrollment redirect rejection is absent" >&2
  exit 1
}
if rg -n 'NSAllowsArbitraryLoads|didReceive challenge|serverTrust|accept.*certificate|URLSessionDelegate' clients/apple "$config"; then
  echo "network verification failed: cleartext or trust override found" >&2
  exit 1
fi
if rg -n 'http://' clients/apple/AnxietyWatchApp clients/apple/Packages \
  --glob '*.swift' --glob '!**/Tests/**'; then
  echo "network verification failed: cleartext URL found in production client code" >&2
  exit 1
fi
rg -q 'TLS is delegated to URLSession.*system trust' "$source_file" || {
  echo "network verification failed: system trust policy is not documented at the transport seam" >&2
  exit 1
}
echo "iOS network security checks passed"
