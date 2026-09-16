#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

if rg -n 'HealthKit|HKQuantity|HKCategory|CoreML|CreateML|MLModel|VNRecognize|PDFDocument|CGContext' \
  clients/apple/Packages/AWTransport/Sources/AWTransport/ReadClient.swift \
  clients/apple/AnxietyWatchApp --glob '*.swift'; then
  echo "analysis verification failed: local HealthKit, analysis, or report generation found in read client" >&2
  exit 1
fi
rg -q 'ProjectionReadState' clients/apple/Packages/AWTransport/Sources/AWTransport/ReadClient.swift || {
  echo "analysis verification failed: server projection read state is absent" >&2
  exit 1
}
echo "iOS no-local-analysis checks passed"
