#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

if [ "$#" -eq 0 ]; then
  echo "test verification failed: at least one named iOS test suite is required" >&2
  exit 1
fi

for suite in "$@"; do
  case "$suite" in
    AWTransportTests)
      swift test --package-path clients/apple/Packages/AWTransport
      ;;
    AWOutboxTests)
      swift test --package-path clients/apple/Packages/AWOutbox
      ;;
    AWDomainContractTests|AWJournalTests|AWExpressModeUITests|AWSongInputTests)
      swift test --package-path clients/apple/Packages/AWDomain
      ;;
    AWSyncEngineTests|AWTransportFaultTests|AWReadClientTests|AWProjectionCacheTests|AWUnknownStateTests)
      swift test --package-path clients/apple/Packages/AWTransport
      ;;
    AWEnrollmentUITests)
      rg -q 'final class EnrollmentUITests: XCTestCase' clients/apple/Tests/AnxietyWatchAppTests/SkeletonTests.swift || {
        echo "test verification failed: ${suite} is not present in the app test target" >&2
        exit 1
      }
      ;;
    *)
      echo "test verification failed: unsupported suite ${suite}" >&2
      exit 1
      ;;
  esac
done

echo "requested iOS test checks passed: $*"
