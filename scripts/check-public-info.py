#!/usr/bin/env python3
"""Reject personal or machine-specific information from tracked public files."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Rule:
    name: str
    pattern: re.Pattern[str]
    message: str


def rx(parts: tuple[str, ...], flags: int = re.IGNORECASE) -> re.Pattern[str]:
    """Build patterns in pieces so this scanner does not match its own source."""
    return re.compile("".join(parts), flags)


RULES = (
    Rule(
        "mac-home",
        rx((r"/", "Users", r"/[A-Za-z0-9._-]+/")),
        "absolute macOS user path",
    ),
    Rule(
        "linux-home",
        rx((r"/", "home", r"/(?!user(?:/|\b)|pi-user(?:/|\b))[A-Za-z0-9._-]+/")),
        "absolute Linux user path; use $HOME, $PI_PATH, /opt, or a placeholder",
    ),
    Rule(
        "literal-ssh-target",
        rx((r"\b", "ssh", r"\s+(?!\"?\$)[A-Za-z0-9._-]+@[A-Za-z0-9._-]+")),
        "literal SSH user/host; use ssh \"$PI_HOST\"",
    ),
    Rule(
        "private-ip-url",
        rx((r"https?://(?:10\.|192[.]168[.]|172[.](?:1[6-9]|2[0-9]|3[01])[.])(?:[0-9]{1,3}[.]){1,2}[0-9]{1,3}")),
        "private-network URL; use a documented hostname placeholder",
    ),
    Rule(
        "full-device-address",
        rx((r"\b(?:[0-9A-F]{2}:){5}[0-9A-F]{2}\b")),
        "full Bluetooth/MAC address; redact the device-specific octets",
    ),
    Rule(
        "personal-device-suffix",
        rx((r"\b(?:SleepO2-2_|ResMed\s+)[0-9]{4,}\b")),
        "personalised device name or serial suffix",
    ),
    Rule(
        "known-private-host",
        rx((r"\bmini", "dude", r"\b")),
        "private hostname",
    ),
    Rule(
        "setup-user",
        rx((r"\b(?:User=|id\s+-u\s+)", "chris", r"\b")),
        "personal account name in deployment configuration",
    ),
    Rule(
        "personal-plugin-id",
        rx((r"\b", "chris", r"[.]opsx-dev-layout\b")),
        "personal plugin identifier",
    ),
)

# These are intentionally public authorship/ownership metadata, not private setup.
ALLOW_PATHS = {"LICENSE", "Cargo.toml", "README.md", "db/sqitch.plan"}
SKIP_PATHS = {"scripts/check-public-info.py"}
PLACEHOLDER_MACS = {"AA:BB:CC:DD:EE:FF", "AA:BB:CC:DD:EE:FE"}


def tracked_files(root: Path) -> list[Path]:
    output = subprocess.check_output(["git", "ls-files", "-z"], cwd=root)
    return [root / item.decode() for item in output.split(b"\0") if item]


def findings_for(path: str, text: str) -> list[str]:
    if path in SKIP_PATHS:
        return []
    findings: list[str] = []
    for number, line in enumerate(text.splitlines(), 1):
        for rule in RULES:
            for match in rule.pattern.finditer(line):
                value = match.group(0)
                if rule.name == "full-device-address" and (
                    "XX" in value.upper() or value.upper() in PLACEHOLDER_MACS
                ):
                    continue
                findings.append(f"{path}:{number}: {rule.message}: {value}")
    return findings


def replacement(rule_name: str, value: str) -> str:
    """Return a deterministic public placeholder for a scanner match."""
    if rule_name in {"mac-home", "linux-home"}:
        return "$HOME/"
    if rule_name == "literal-ssh-target":
        return 'ssh "$PI_HOST"'
    if rule_name == "private-ip-url":
        scheme = value.split("://", 1)[0]
        return f"{scheme}://pi-host"
    if rule_name == "full-device-address":
        if "XX" in value.upper() or value.upper() in PLACEHOLDER_MACS:
            return value
        return f"{value[:8]}XX:XX:XX"
    if rule_name == "personal-device-suffix":
        if value.lower().startswith("sleepo2"):
            return "SleepO2-2_XXXX"
        return "ResMed device"
    if rule_name == "known-private-host":
        return "pi-host"
    if rule_name == "setup-user":
        if value.startswith("User="):
            return "User=pi-user"
        return "id -u pi-user"
    if rule_name == "personal-plugin-id":
        return "project.opsx-dev-layout"
    raise ValueError(f"no public replacement for rule: {rule_name}")


def fix_text(path: str, text: str) -> str:
    if path in SKIP_PATHS:
        return text
    fixed = text
    for rule in RULES:
        fixed = rule.pattern.sub(lambda match: replacement(rule.name, match.group(0)), fixed)
    return fixed


def scan(root: Path) -> list[str]:
    findings: list[str] = []
    for file_path in tracked_files(root):
        relative = file_path.relative_to(root).as_posix()
        if relative in ALLOW_PATHS or relative in SKIP_PATHS:
            continue
        try:
            text = file_path.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        findings.extend(findings_for(relative, text))
    return findings


def self_test() -> None:
    bad = {
        "a.md": "ssh jane@bedroom-pi true",
        "b.md": "/Users/jane/Source/project",
        "c.md": "/home/jane/private/file",
        "d.md": "http://192.168.50.7:4000/ui",
        "e.md": "sensor 11:22:33:44:55:66",
        "f.md": "SleepO2-2_1234",
        "g.md": "private host pi-host",
        "h.md": "User=pi-user",
        "i.md": "plugin project.opsx-dev-layout",
    }
    for path, text in bad.items():
        if not findings_for(path, text):
            raise AssertionError(f"scanner missed fixture: {text}")

    good = (
        'ssh "$PI_HOST" true\n'
        "$PI_PATH/file\n"
        "/home/user/project\n"
        "http://pi-host:4000/ui\n"
        "11:22:33:XX:XX:XX\n"
        "AA:BB:CC:DD:EE:FF\n"
    )
    findings = findings_for("good.md", good)
    if findings:
        raise AssertionError(f"scanner rejected placeholders: {findings}")

    fixed = fix_text("bad.md", "ssh jane@bedroom-pi true /Users/jane/x http://192.168.50.7/ui 11:22:33:44:55:66")
    if findings_for("bad.md", fixed):
        raise AssertionError(f"fixer left findings: {findings_for('bad.md', fixed)}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--fix",
        action="store_true",
        help="replace deterministic setup identifiers with public placeholders",
    )
    args = parser.parse_args()
    root = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())

    if args.self_test:
        self_test()
        print("public-info scanner self-test passed")

    if args.fix:
        changed: list[str] = []
        for file_path in tracked_files(root):
            relative = file_path.relative_to(root).as_posix()
            if relative in SKIP_PATHS:
                continue
            try:
                text = file_path.read_text()
            except (UnicodeDecodeError, OSError):
                continue
            fixed = fix_text(relative, text)
            if fixed != text:
                file_path.write_text(fixed)
                changed.append(relative)
        print(f"public-info fixer updated {len(changed)} tracked files")
        for relative in changed:
            print(f"  {relative}")

    findings = scan(root)
    if findings:
        print("public-info scan failed:", file=sys.stderr)
        for finding in findings:
            print(f"  {finding}", file=sys.stderr)
        return 1
    print("tracked files contain no private setup identifiers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
