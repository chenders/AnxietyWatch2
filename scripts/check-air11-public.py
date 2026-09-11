#!/usr/bin/env python3
"""Fail closed on private AirSense artifacts and an under-constrained unit."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path


FULL_MAC = re.compile(r"\b(?:[0-9A-F]{2}:){5}[0-9A-F]{2}\b", re.IGNORECASE)
PRIVATE_ARTIFACT = re.compile(
    r"(?i)(?:air11|resmed|capture|snoop|hci|dump).*\.(?:jsonl|pcap|pcapng|snoop|hci|bin)$"
)
PRIVATE_CREDENTIAL = re.compile(
    r'(?i)"(?:master[_-]?pair[_-]?key|master[_-]?key)"\s*:\s*"[^"]+"'
)
PRIVATE_DEVICE = re.compile(r"(?i)\b(?:SleepO2-2_|ResMed\s+)[0-9]{4,}\b")
RAW_EXTENSIONS = {".pcap", ".pcapng", ".snoop", ".hci"}

REQUIRED_SERVICE_LINES = (
    "After=bluetooth.service vitalsd.service",
    "Requires=bluetooth.service vitalsd.service",
    "User=vitals",
    "Group=vitals",
    "UMask=0077",
    "ExecStart=/usr/local/bin/resmed-air11-collector",
    "--credentials %d/air11-credentials.json",
    "--state-dir /var/lib/vitals/air11",
    "--socket /run/vitals/vitalsd.sock",
    "--status /run/vitals-air11/status.json",
    "LoadCredential=air11-credentials.json:%h/.air11/air11-credentials.json",
    "OOMScoreAdjust=-1000",
    "MemoryMin=64M",
    "MemoryHigh=192M",
    "Restart=always",
    "RestartSec=5",
    "StartLimitIntervalSec=300",
    "StartLimitBurst=5",
    "NoNewPrivileges=true",
    "PrivateTmp=true",
    "PrivateDevices=true",
    "ProtectSystem=strict",
    "ProtectHome=true",
    "MemoryDenyWriteExecute=true",
    "SystemCallArchitectures=native",
    "CapabilityBoundingSet=",
    "AmbientCapabilities=",
    "RestrictAddressFamilies=AF_UNIX AF_NETLINK AF_BLUETOOTH",
    "StateDirectory=vitals/air11",
    "RuntimeDirectory=vitals-air11",
)
SKIP_PATHS = {"scripts/check-public-info.py"}

FORBIDDEN_SERVICE_TOKENS = (
    "Environment=",
    "EnvironmentFile=",
    "DATABASE_URL",
    "postgres://",
    "AF_INET",
    "AF_INET6",
    "masterPairKey",
    "sessionKey",
    "StartKeyExchange",
    "ConfirmKeyExchange",
    "StopStream",
)
UNIT_START_LIMITS = ("StartLimitIntervalSec=300", "StartLimitBurst=5")


def root_dir() -> Path:
    return Path(
        subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True
        ).strip()
    )


def tracked_files(root: Path) -> list[Path]:
    output = subprocess.check_output(["git", "ls-files", "-z"], cwd=root)
    return [root / item.decode() for item in output.split(b"\0") if item]


def service_findings(text: str) -> list[str]:
    findings = [
        f"missing required service directive: {line}"
        for line in REQUIRED_SERVICE_LINES
        if line not in text
    ]
    unit_match = re.search(r"(?ms)^\[Unit\]\n(.*?)(?=^\[|\Z)", text)
    unit_text = unit_match.group(1) if unit_match else ""
    findings.extend(
        f"start limit must be in [Unit]: {line}"
        for line in UNIT_START_LIMITS
        if line not in unit_text
    )
    findings.extend(
        f"forbidden service token: {token}"
        for token in FORBIDDEN_SERVICE_TOKENS
        if token in text
    )
    exec_lines = [line for line in text.splitlines() if line.startswith("ExecStart=")]
    if len(exec_lines) != 1:
        findings.append(f"expected one collector ExecStart, found {len(exec_lines)}")
    elif "--credentials %d/air11-credentials.json" not in exec_lines[0]:
        findings.append("collector ExecStart does not use the private systemd credential path")
    return findings


def file_findings(root: Path, path: Path) -> list[str]:
    relative = path.relative_to(root).as_posix()
    if relative in SKIP_PATHS:
        return []
    findings: list[str] = []
    suffix = path.suffix.lower()
    if PRIVATE_ARTIFACT.search(relative) or suffix in RAW_EXTENSIONS:
        findings.append(f"private AirSense artifact is tracked: {relative}")
        return findings
    if re.search(r"(?i)(?:credentials?|secrets?|password|passwd)(?:[._-]|$)", relative):
        findings.append(f"credential or secret artifact is tracked: {relative}")
        return findings
    try:
        text = path.read_text()
    except (UnicodeDecodeError, OSError):
        return findings
    for number, line in enumerate(text.splitlines(), 1):
        for match in FULL_MAC.finditer(line):
            value = match.group(0).upper()
            if value != "AA:BB:CC:DD:EE:FF" and "XX" not in value:
                findings.append(f"{relative}:{number}: device MAC is tracked: {value}")
        if PRIVATE_DEVICE.search(line):
            findings.append(f"{relative}:{number}: personalised device identifier is tracked")
        if suffix == ".json" and PRIVATE_CREDENTIAL.search(line):
            findings.append(f"{relative}:{number}: credential material is tracked")
    return findings


def copied_aircannect_findings(root: Path, path: Path) -> list[str]:
    relative = path.relative_to(root).as_posix()
    if not (
        relative.startswith("crates/vitals-collectors/src/")
        and (
            "air11" in relative.lower()
            or "as11" in relative.lower()
            or relative.endswith("/resmed_air11_collector.rs")
        )
    ):
        return []
    try:
        text = path.read_text()
    except (UnicodeDecodeError, OSError):
        return []
    forbidden = ("#include", "Arduino", "BLEDevice", "CAN.begin", "esp_", "NimBLE")
    findings = [
        f"{relative}: copied AirCANnect/runtime token tracked: {token}"
        for token in forbidden
        if token in text
    ]
    if re.search(r"\bCAN\b", text):
        findings.append(f"{relative}: copied AirCANnect/runtime token tracked: CAN")
    return findings


def scan(root: Path) -> list[str]:
    service = root / "deploy/vitals-air11.service"
    findings = []
    try:
        findings.extend(service_findings(service.read_text()))
    except OSError as error:
        findings.append(f"cannot read {service.relative_to(root)}: {error}")
    for path in tracked_files(root):
        findings.extend(file_findings(root, path))
        findings.extend(copied_aircannect_findings(root, path))
    return findings


def self_test(root: Path) -> None:
    good = root / "deploy/vitals-air11.service"
    service = good.read_text()
    if service_findings(service):
        raise AssertionError("checked-in AirSense service does not satisfy its own gate")
    mutated = service.replace("ProtectHome=true", "ProtectHome=false", 1)
    if not service_findings(mutated):
        raise AssertionError("service gate did not reject a weakened ProtectHome")
    if not file_findings(root, root / "air11-night-20260906.jsonl"):
        raise AssertionError("artifact gate missed a raw-capture filename")
    masked = "11:22:" + "33:44:55:66"
    if FULL_MAC.search(masked) is None:
        raise AssertionError("artifact gate self-test setup failed")
    if FULL_MAC.search("AA:BB:CC:DD:EE:FF") is None:
        raise AssertionError("artifact gate placeholder setup failed")
    source_dir = root / "crates/vitals-collectors/src"
    with tempfile.NamedTemporaryFile(
        mode="w", prefix="as11_synthetic_", suffix=".rs", dir=source_dir, delete=False
    ) as handle:
        handle.write("// no Arduino runtime\n")
        synthetic = Path(handle.name)
    try:
        if not copied_aircannect_findings(root, synthetic):
            raise AssertionError("runtime-copy gate missed a synthetic Arduino token")
    finally:
        synthetic.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = root_dir()
    if args.self_test:
        self_test(root)
        print("AirSense public/service gate self-test passed")
    findings = scan(root)
    if findings:
        print("AirSense public/service gate failed:", file=sys.stderr)
        for finding in findings:
            print(f"  {finding}", file=sys.stderr)
        return 1
    print("AirSense tracked artifacts and service confinement passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
