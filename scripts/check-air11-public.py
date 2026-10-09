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
    "After=bluetooth.service vitalsd.service vitals-ble-broker-air11.service",
    "Requires=bluetooth.service vitalsd.service vitals-ble-broker-air11.service",
    "EnvironmentFile=/etc/vitals/ble-broker-air11.env",
    "User=vitals",
    "Group=vitals",
    "UMask=0077",
    "ExecStart=/usr/local/bin/resmed-air11-collector",
    "--credentials %d/air11-credentials.json",
    "--adapter-selector ${VITALS_BLE_ADAPTER_SELECTOR}",
    "--broker-socket ${VITALS_BLE_BROKER_SOCKET}",
    "--state-dir /var/lib/vitals/air11",
    "--socket /run/vitals/vitalsd.sock",
    "--status /run/vitals-air11/status.json",
    "LoadCredential=air11-credentials.json:/var/lib/vitals/.air11/air11-credentials.json",
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
REQUIRED_BROKER_SERVICE_LINES = (
    "After=bluetooth.service vitalsd.service",
    "Requires=bluetooth.service",
    "Before=vitals-air11.service",
    "ExecStart=/usr/local/bin/vitals-ble-broker",
    "Restart=always",
    "RestartSec=3",
    "EnvironmentFile=/etc/vitals/ble-broker-air11.env",
    "Environment=VITALS_BLE_BROKER_SOCKET=/run/vitals/ble-broker-air11.sock",
    "Environment=VITALS_BLE_BROKER_METRICS_PATH=/run/vitals/ble-broker-air11.prom",
    "User=vitals",
    "Group=vitals",
    "UMask=0007",
    "OOMScoreAdjust=-1000",
    "MemoryMin=64M",
    "MemoryHigh=192M",
    "NoNewPrivileges=true",
    "PrivateTmp=true",
    "ProtectSystem=strict",
    "ReadWritePaths=/run/vitals",
    "ProtectHome=true",
    "RestrictAddressFamilies=AF_UNIX AF_NETLINK",
    "RestrictNamespaces=true",
    "LockPersonality=true",
    "MemoryDenyWriteExecute=true",
    "SystemCallArchitectures=native",
)
REQUIRED_INSTALLER_TOKENS = (
    "broker-unit)",
    'local_path="deploy/vitals-ble-broker-air11.service"',
    'remote_path="/etc/systemd/system/vitals-ble-broker-air11.service"',
)
SKIP_PATHS = {"scripts/check-public-info.py"}

FORBIDDEN_SERVICE_TOKENS = (
    "Environment=",
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
AIR11_TRANSPORT_SOURCES = (
    "crates/vitals-collectors/src/as11_broker.rs",
    "crates/vitals-collectors/src/as11_common.rs",
    "crates/vitals-collectors/src/bin/resmed_air11_collector.rs",
)
DIRECT_AIR11_TRANSPORT = re.compile(r"(?:^|::)bluer|\bhci[0-9]+\b")
CONSOLE_REQUIRED_LINES = (
    "ExecStart=/usr/local/bin/vitals-console",
    "User=vitals",
    "Group=vitals",
    "SupplementaryGroups=systemd-journal",
    "Environment=VITALS_CONSOLE_BIND=0.0.0.0:8480",
    "Environment=VITALS_CONSOLE_ALLOWED_NETWORKS=127.0.0.0/8,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,fd00::/8,::1/128",
    "Environment=VITALS_CONSOLE_AIR11_ACTIONS=1",
    "Environment=VITALS_CONSOLE_ADMIN_TOKEN_FILE=/etc/vitals/console-admin-token",
    "LoadCredential=air11-credentials.json:/var/lib/vitals/.air11/air11-credentials.json",
    "Environment=AIR11_CREDENTIALS_PATH=%d/air11-credentials.json",
    "ProtectHome=true",
    "NoNewPrivileges=true",
    "ProtectSystem=strict",
    "RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6",
)
CONSOLE_FORBIDDEN_TOKENS = (
    "EnvironmentFile=",
    "AF_BLUETOOTH",
    "ExecStart=/bin/sh",
    "ExecStart=/bin/bash",
)
ACCEPTANCE_REQUIRED = (
    "--require-postgres",
    "--require-h10",
    "--require-emay",
    "--require-ezshare-unchanged",
    "AIR11-LIVE-",
    'systemctl start "$AIR11_UNIT"',
    'systemctl restart "$AIR11_UNIT"',
    'systemctl stop "$AIR11_UNIT"',
    "--no-password",
    '"durability_unknown"',
    '"received_envelopes"',
    '"decoded_frames"',
    '"published_samples"',
    "distinct session_ids=",
)
FORBIDDEN_ACCEPTANCE_TOKENS = (
    "bluetoothctl pair",
    "bluetoothctl scan",
    "bluetoothctl trust",
    "bluetoothctl remove",
    "systemctl enable",
    "systemctl disable",
    "ip route add",
    "ip route del",
    "ip route replace",
    "dtoverlay",
    "StartStream",
    "StopStream",
)


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
    environment_files = [
        line for line in text.splitlines() if line.startswith("EnvironmentFile=")
    ]
    findings.extend(
        f"unexpected service environment file: {line}"
        for line in environment_files
        if line != "EnvironmentFile=/etc/vitals/ble-broker-air11.env"
    )
    exec_lines = [line for line in text.splitlines() if line.startswith("ExecStart=")]
    if len(exec_lines) != 1:
        findings.append(f"expected one collector ExecStart, found {len(exec_lines)}")
    elif "--credentials %d/air11-credentials.json" not in exec_lines[0]:
        findings.append("collector ExecStart does not use the private systemd credential path")
    return findings


def broker_service_findings(text: str) -> list[str]:
    findings = [
        f"missing required AirSense broker directive: {line}"
        for line in REQUIRED_BROKER_SERVICE_LINES
        if line not in text
    ]
    if re.search(r"\bhci[0-9]+\b", text):
        findings.append("AirSense broker service uses a runtime hci alias")
    return findings


def installer_findings(text: str) -> list[str]:
    return [
        f"AirSense installer missing fixed broker-unit token: {token}"
        for token in REQUIRED_INSTALLER_TOKENS
        if token not in text
    ]


def air11_transport_findings(root: Path) -> list[str]:
    findings: list[str] = []
    for relative in AIR11_TRANSPORT_SOURCES:
        path = root / relative
        try:
            text = path.read_text()
        except OSError as error:
            findings.append(f"cannot read AirSense transport source {relative}: {error}")
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            if DIRECT_AIR11_TRANSPORT.search(line):
                findings.append(
                    f"direct AirSense BlueZ/runtime-alias access in {relative}:{line_number}"
                )
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


def console_service_findings(text: str) -> list[str]:
    findings = [
        f"console service missing required directive: {line}"
        for line in CONSOLE_REQUIRED_LINES
        if line not in text
    ]
    findings.extend(
        f"forbidden console service token: {token}"
        for token in CONSOLE_FORBIDDEN_TOKENS
        if token in text
    )
    exec_lines = [line for line in text.splitlines() if line.startswith("ExecStart=")]
    if len(exec_lines) != 1:
        findings.append(f"expected one console ExecStart, found {len(exec_lines)}")
    return findings


def acceptance_findings_for_text(text: str) -> list[str]:
    findings = [
        f"acceptance harness missing required guard: {token}"
        for token in ACCEPTANCE_REQUIRED
        if token not in text
    ]
    findings.extend(
        f"acceptance harness contains forbidden action: {token}"
        for token in FORBIDDEN_ACCEPTANCE_TOKENS
        if token in text
    )
    return findings


def acceptance_findings(root: Path) -> list[str]:
    path = root / "scripts/air11-pi-acceptance.sh"
    if not path.is_file():
        return [f"missing bounded Pi acceptance harness: {path.relative_to(root)}"]
    if not path.stat().st_mode & 0o111:
        return [f"bounded Pi acceptance harness is not executable: {path.relative_to(root)}"]
    try:
        text = path.read_text()
    except (UnicodeDecodeError, OSError) as error:
        return [f"cannot read bounded Pi acceptance harness: {error}"]
    return acceptance_findings_for_text(text)


def scan(root: Path) -> list[str]:
    service = root / "deploy/vitals-air11.service"
    broker_service = root / "deploy/vitals-ble-broker-air11.service"
    installer = root / "scripts/install-air11-pi-artifact.sh"
    console = root / "deploy/vitals-console.service"
    findings = acceptance_findings(root)
    findings.extend(air11_transport_findings(root))
    try:
        findings.extend(service_findings(service.read_text()))
    except OSError as error:
        findings.append(f"cannot read {service.relative_to(root)}: {error}")
    try:
        findings.extend(broker_service_findings(broker_service.read_text()))
    except OSError as error:
        findings.append(f"cannot read {broker_service.relative_to(root)}: {error}")
    try:
        findings.extend(installer_findings(installer.read_text()))
    except OSError as error:
        findings.append(f"cannot read {installer.relative_to(root)}: {error}")
    try:
        findings.extend(console_service_findings(console.read_text()))
    except OSError as error:
        findings.append(f"cannot read {console.relative_to(root)}: {error}")
    for path in tracked_files(root):
        findings.extend(file_findings(root, path))
        findings.extend(copied_aircannect_findings(root, path))
    return findings


def self_test(root: Path) -> None:
    good = root / "deploy/vitals-air11.service"
    service = good.read_text()
    if service_findings(service):
        raise AssertionError("checked-in AirSense service does not satisfy its own gate")
    broker = root / "deploy/vitals-ble-broker-air11.service"
    broker_text = broker.read_text()
    if broker_service_findings(broker_text):
        raise AssertionError("checked-in AirSense broker service does not satisfy its own gate")
    if air11_transport_findings(root):
        raise AssertionError("checked-in AirSense worker contains direct Bluetooth access")
    if not DIRECT_AIR11_TRANSPORT.search("bluer::Session hci0"):
        raise AssertionError("transport gate self-test setup failed")
    mutated = service.replace("ProtectHome=true", "ProtectHome=false", 1)
    if not service_findings(mutated):
        raise AssertionError("service gate did not reject a weakened ProtectHome")
    weakened_broker = broker_text.replace("ReadWritePaths=/run/vitals", "", 1)
    if not broker_service_findings(weakened_broker):
        raise AssertionError("broker service gate did not reject a missing writable runtime path")
    installer = root / "scripts/install-air11-pi-artifact.sh"
    installer_text = installer.read_text()
    if installer_findings(installer_text):
        raise AssertionError("AirSense installer does not satisfy its own broker-unit gate")
    weakened_installer = installer_text.replace(
        'remote_path="/etc/systemd/system/vitals-ble-broker-air11.service"',
        'remote_path="/etc/systemd/system/vitals-ble-broker.service"',
        1,
    )
    if not installer_findings(weakened_installer):
        raise AssertionError("installer gate did not reject an unexpected broker-unit destination")
    console = root / "deploy/vitals-console.service"
    console_text = console.read_text()
    if console_service_findings(console_text):
        raise AssertionError("console service does not satisfy its own confinement gate")
    weakened_console = console_text.replace("ProtectHome=true", "ProtectHome=false", 1)
    if not console_service_findings(weakened_console):
        raise AssertionError("console service gate did not reject a weakened ProtectHome")
    acceptance = root / "scripts/air11-pi-acceptance.sh"
    acceptance_text = acceptance.read_text()
    if acceptance_findings(root):
        raise AssertionError("bounded Pi acceptance harness does not satisfy its own gate")
    weakened_acceptance = acceptance_text.replace(
        'systemctl start "$AIR11_UNIT"', 'systemctl enable "$AIR11_UNIT"', 1
    )
    if not acceptance_findings_for_text(weakened_acceptance):
        raise AssertionError("acceptance gate did not reject a service enable action")
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
