#!/usr/bin/env python3
"""Check the AirSense admin migration remains non-secret and bounded."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


DEPLOY_REQUIRED = (
    "CREATE TABLE air11_integration_config",
    "singleton  BOOLEAN     PRIMARY KEY DEFAULT TRUE CHECK (singleton)",
    "opted_in   BOOLEAN     NOT NULL DEFAULT FALSE",
    "VALUES (TRUE, FALSE)",
    "CREATE TABLE air11_admin_audit",
    "GENERATED ALWAYS AS IDENTITY PRIMARY KEY",
    "'opt_in', 'opt_out', 'start', 'stop', 'restart', 'credential_check'",
    "'requested', 'succeeded', 'failed', 'blocked'",
    "CREATE INDEX air11_admin_audit_created_idx",
    "CREATE TRIGGER air11_admin_audit_immutable",
)
REVERT_REQUIRED = (
    "DROP TABLE IF EXISTS air11_admin_audit",
    "DROP TABLE IF EXISTS air11_integration_config",
)
VERIFY_REQUIRED = (
    "FROM air11_integration_config",
    "FROM air11_admin_audit",
    "air11_admin_audit_created_idx",
    "air11_admin_audit_immutable",
)
FORBIDDEN_SQL_TOKENS = (
    "CREATE TABLE samples",
    "master_pair_key TEXT",
    "session_key TEXT",
    "raw_payload",
    "device_address TEXT",
    "client_id TEXT",
)


def root_dir() -> Path:
    return Path(
        subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True
        ).strip()
    )


def findings_from_text(deploy: str, revert: str, verify: str) -> list[str]:
    result: list[str] = []
    result.extend(
        f"deploy migration is missing required non-secret structure: {token}"
        for token in DEPLOY_REQUIRED
        if token not in deploy
    )
    result.extend(
        f"revert migration is missing required drop: {token}"
        for token in REVERT_REQUIRED
        if token not in revert
    )
    result.extend(
        f"verify migration is missing required check: {token}"
        for token in VERIFY_REQUIRED
        if token not in verify
    )
    result.extend(
        f"AirSense admin migration contains forbidden schema token: {token}"
        for token in FORBIDDEN_SQL_TOKENS
        if token in deploy
    )
    return result


def findings(root: Path) -> list[str]:
    paths = {
        "deploy": root / "db/deploy/air11_admin.sql",
        "revert": root / "db/revert/air11_admin.sql",
        "verify": root / "db/verify/air11_admin.sql",
    }
    result: list[str] = []
    for name, path in paths.items():
        if not path.is_file():
            result.append(f"missing AirSense admin {name} migration: {path}")
    if result:
        return result
    return findings_from_text(
        paths["deploy"].read_text(),
        paths["revert"].read_text(),
        paths["verify"].read_text(),
    )


def self_test(root: Path) -> None:
    if findings(root):
        raise AssertionError("checked-in migration does not satisfy its own gate")
    deploy = (root / "db/deploy/air11_admin.sql").read_text()
    mutated = deploy.replace("VALUES (TRUE, FALSE)", "VALUES (TRUE, TRUE)", 1)
    if not findings_from_text(
        mutated,
        (root / "db/revert/air11_admin.sql").read_text(),
        (root / "db/verify/air11_admin.sql").read_text(),
    ):
        raise AssertionError("SQL gate accepted an opted-in initial state")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = root_dir()
    if args.self_test:
        self_test(root)
        print("AirSense admin SQL self-test passed")
    result = findings(root)
    if result:
        print("AirSense admin SQL check failed:", file=sys.stderr)
        for item in result:
            print(f"  {item}", file=sys.stderr)
        return 1
    print("AirSense admin SQL structure passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
