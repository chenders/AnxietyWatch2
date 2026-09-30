#!/usr/bin/env python3
"""Check the Rust console's JavaScript assets, including the kiosk page."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path


JS_ASSETS = (
    "crates/vitals-console/assets/console.js",
    "crates/vitals-console/assets/dir-a.js",
    "crates/vitals-console/assets/dir-b.js",
    "crates/vitals-console/assets/dir-c.js",
)
KIOSK = "crates/vitals-console/assets/air11-kiosk.html"
BEDSIDE = "crates/vitals-console/assets/air11-bedside.html"


def root_dir() -> Path:
    return Path(
        subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True
        ).strip()
    )


def node_check(path: Path, *, cwd: Path) -> list[str]:
    try:
        result = subprocess.run(
            ["node", "--check", str(path)],
            cwd=cwd,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as error:
        return [f"cannot execute node for {path}: {error}"]
    if result.returncode:
        detail = (result.stderr or result.stdout).strip()
        return [f"JavaScript syntax failed for {path}: {detail}"]
    return []


def kiosk_script(text: str) -> str | None:
    matches = re.findall(r"<script>(.*?)</script>", text, re.DOTALL)
    if len(matches) != 1:
        return None
    return matches[0]


def check(root: Path) -> list[str]:
    findings: list[str] = []
    for relative in JS_ASSETS:
        path = root / relative
        if not path.is_file():
            findings.append(f"missing console JavaScript asset: {relative}")
            continue
        findings.extend(node_check(path, cwd=root))

    kiosk = root / KIOSK
    if not kiosk.is_file():
        findings.append(f"missing kiosk page: {KIOSK}")
        return findings
    text = kiosk.read_text()
    script = kiosk_script(text)
    if script is None:
        findings.append(f"kiosk page must contain exactly one inline script: {KIOSK}")
    else:
        with tempfile.NamedTemporaryFile(
            mode="w", prefix="air11-kiosk-", suffix=".js", delete=False
        ) as handle:
            handle.write(script)
            script_path = Path(handle.name)
        try:
            findings.extend(node_check(script_path, cwd=root))
        finally:
            script_path.unlink(missing_ok=True)
    for token in ("/api/air11", "Read-only therapy stream", "no therapy controls"):
        if token not in text:
            findings.append(f"kiosk page is missing its read-only contract token: {token}")

    bedside = root / BEDSIDE
    if not bedside.is_file():
        findings.append(f"missing bedside page: {BEDSIDE}")
    else:
        bedside_text = bedside.read_text()
        bedside_script = kiosk_script(bedside_text)
        if bedside_script is None:
            findings.append(f"bedside page must contain exactly one inline script: {BEDSIDE}")
        else:
            with tempfile.NamedTemporaryFile(
                mode="w", prefix="air11-bedside-", suffix=".js", delete=False
            ) as handle:
                handle.write(bedside_script)
                script_path = Path(handle.name)
            try:
                findings.extend(node_check(script_path, cwd=root))
            finally:
                script_path.unlink(missing_ok=True)
        for token in ("/api/air11", "NIGHT FIELD", "AHI is intentionally not derived here"):
            if token not in bedside_text:
                findings.append(f"bedside page is missing its visual/read-only contract token: {token}")
    return findings


def self_test(root: Path) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w", prefix="air11-console-invalid-", suffix=".js", delete=False
    ) as handle:
        handle.write("const = invalid;")
        invalid = Path(handle.name)
    try:
        if not node_check(invalid, cwd=root):
            raise AssertionError("JavaScript syntax gate accepted invalid input")
    finally:
        invalid.unlink(missing_ok=True)
    findings = check(root)
    if findings:
        raise AssertionError("console asset check failed its own repository: " + "; ".join(findings))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = root_dir()
    if args.self_test:
        self_test(root)
        print("console asset syntax self-test passed")
    findings = check(root)
    if findings:
        print("console asset check failed:", file=sys.stderr)
        for finding in findings:
            print(f"  {finding}", file=sys.stderr)
        return 1
    print("console JavaScript and kiosk checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
