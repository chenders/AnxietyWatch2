#!/usr/bin/env python3
"""Local-only Polar H10 PS-FTP session and reset tool.

This reproduces the verified Java-side PS-FTP framing from Polar Flow 7.36.2;
it does not use Polar Flow, Polar Cloud, an account, or bluetoothctl pairing.

Safe operation (session init and live observation):
    python3 h10_psftp_offline.py --operation initialize

Inspect the onboard user-id file without printing its contents:
    python3 h10_psftp_offline.py --operation read-userid

The reset operations are destructive and require an explicit acknowledgement:
    python3 h10_psftp_offline.py --operation factory-reset \
        --i-understand-data-loss

The factory-reset payload intentionally matches Flow's
sendFactoryResetNotification(false): sleep=false, ota_fwupdate=false, with
protobuf defaults for the other fields. No guessed raw characteristic write is
used; both PS-FTP notification characteristics are enabled and the message is
framed exactly as the analyzed client does it.
"""

from __future__ import annotations

import argparse
import asyncio
import sys
from dataclasses import dataclass
from typing import Any

try:
    from bleak import BleakClient, BleakScanner
except ImportError:
    BleakClient = None  # type: ignore[assignment,misc]
    BleakScanner = None  # type: ignore[assignment,misc]
    _BLEAK_IMPORT_ERROR = "Missing dependency: bleak (python3 -m pip install bleak)"
else:
    _BLEAK_IMPORT_ERROR = None

PSFTP_SERVICE = "0000feee-0000-1000-8000-00805f9b34fb"
PSFTP_MTU = "fb005c51-02e7-f387-1cad-8acd2d8df0c8"
PSFTP_D2H = "fb005c52-02e7-f387-1cad-8acd2d8df0c8"
PSFTP_H2D = "fb005c53-02e7-f387-1cad-8acd2d8df0c8"
HR_SERVICE = "0000180d-0000-1000-8000-00805f9b34fb"
HR_MEASUREMENT = "00002a37-0000-1000-8000-00805f9b34fb"

# Flow's protocol.PbPFtpHostToDevNotification values.
START_SYNC = 0
RESET = 2
INITIALIZE_SESSION = 8


@dataclass
class Candidate:
    device: Any
    advertisement: Any

    @property
    def name(self) -> str:
        return (
            getattr(self.advertisement, "local_name", None)
            or getattr(self.device, "name", None)
            or "Unknown device"
        )

    @property
    def address(self) -> str:
        return str(getattr(self.device, "address", ""))


def short_address(address: str) -> str:
    return "…" + address[-5:] if address else "<unknown address>"


def is_h10(candidate: Candidate) -> bool:
    name = candidate.name.lower()
    uuids = {
        str(value).lower()
        for value in (getattr(candidate.advertisement, "service_uuids", None) or [])
    }
    return "h10" in name or PSFTP_SERVICE in uuids or HR_SERVICE in uuids


async def scan_h10s(timeout: float) -> list[Candidate]:
    if BleakScanner is None:
        raise RuntimeError(_BLEAK_IMPORT_ERROR or "Bleak is unavailable")
    found = await BleakScanner.discover(timeout=timeout, return_adv=True)
    candidates = [
        Candidate(item[0], item[1])
        for item in found.values()
        if isinstance(item, tuple) and len(item) == 2 and is_h10(Candidate(item[0], item[1]))
    ]
    candidates.sort(key=lambda item: item.name.lower())
    return candidates


def choose_candidate(candidates: list[Candidate], address: str | None) -> Candidate:
    if address:
        for candidate in candidates:
            if candidate.address.lower() == address.lower():
                return candidate
        raise RuntimeError(f"H10 address {short_address(address)} was not found")
    if not candidates:
        raise RuntimeError("No H10 advertisement found; wet the strap and scan again")
    if len(candidates) == 1:
        return candidates[0]
    print("Multiple H10-compatible devices found:")
    for index, candidate in enumerate(candidates, 1):
        print(f"  {index}. {candidate.name} ({short_address(candidate.address)})")
    try:
        selected = int(input("Select a device number: "))
    except (ValueError, EOFError) as exc:
        raise RuntimeError("device selection was not a number") from exc
    if not 1 <= selected <= len(candidates):
        raise RuntimeError(f"device selection {selected} is outside 1..{len(candidates)}")
    return candidates[selected - 1]


def varint(value: int) -> bytes:
    if value < 0:
        raise ValueError("varint cannot encode a negative value")
    result = bytearray()
    while value >= 0x80:
        result.append((value & 0x7F) | 0x80)
        value >>= 7
    result.append(value)
    return bytes(result)


def protobuf_factory_reset(
    *,
    sleep: bool,
    do_factory_defaults: bool | None = None,
    ota_fwupdate: bool | None = None,
    hibernate: bool | None = None,
) -> bytes:
    """Encode PbPFtpFactoryResetParams, matching Wire's proto2 encoding."""
    fields = bytearray((0x08, int(sleep)))
    for tag, value in (
        (2, do_factory_defaults),
        (3, ota_fwupdate),
        (4, hibernate),
    ):
        if value is not None:
            fields.extend((tag << 3, int(value)))
    return bytes(fields)


def protobuf_get_operation(path: str) -> bytes:
    """Encode PbPFtpOperation(command=GET, path=path)."""
    encoded_path = path.encode("utf-8")
    return b"\x08\x00\x12" + varint(len(encoded_path)) + encoded_path


def user_identifier_registration_state(data: bytes) -> str:
    """Return Flow's registration gate without exposing identifier contents."""
    index = 0
    password_token_present = False
    try:
        while index < len(data):
            key, index = read_varint(data, index)
            field = key >> 3
            wire_type = key & 7
            if field == 3 and wire_type == 2:
                password_token_present = True
            index = skip_wire_value(data, index, wire_type)
        return "registered" if password_token_present else "not_registered"
    except ValueError:
        return "unknown"


def read_varint(data: bytes, index: int) -> tuple[int, int]:
    value = 0
    shift = 0
    while index < len(data) and shift < 64:
        byte = data[index]
        index += 1
        value |= (byte & 0x7F) << shift
        if byte < 0x80:
            return value, index
        shift += 7
    raise ValueError("malformed protobuf varint")


def skip_wire_value(data: bytes, index: int, wire_type: int) -> int:
    if wire_type == 0:
        _value, index = read_varint(data, index)
        return index
    if wire_type == 1:
        end = index + 8
    elif wire_type == 2:
        length, index = read_varint(data, index)
        end = index + length
    elif wire_type == 5:
        end = index + 4
    else:
        raise ValueError(f"unsupported protobuf wire type {wire_type}")
    if end > len(data):
        raise ValueError("protobuf field extends beyond payload")
    return end


def frame_message(message: bytes, mtu_payload: int) -> list[bytes]:
    """Apply BlePsFtpUtils.m36825a/m36826b framing."""
    if mtu_payload < 2:
        raise ValueError(f"MTU payload {mtu_payload} is too small")
    frames: list[bytes] = []
    offset = 0
    sequence = 0
    while offset < len(message) or not frames:
        remaining = len(message) - offset
        room = mtu_payload - 1
        final = remaining <= room
        count = remaining if final else room
        first = 0 if not frames else 1
        kind = 1 if final else 3
        header = first | (kind << 1) | (sequence << 4)
        frames.append(bytes((header,)) + message[offset : offset + count])
        offset += count
        sequence = (sequence + 1) % 16
    return frames


def decode_frame(packet: bytes) -> tuple[int, int, int, bytes]:
    if not packet:
        raise ValueError("empty PS-FTP packet")
    header = packet[0]
    return header & 1, (header >> 1) & 3, (header >> 4) & 15, packet[1:]


async def receive_message(queue: asyncio.Queue[bytes], timeout: float) -> bytes:
    payload = bytearray()
    expected_sequence = 0
    first_packet = True
    async with asyncio.timeout(timeout):
        while True:
            packet = await queue.get()
            first, kind, sequence, fragment = decode_frame(packet)
            if first_packet:
                if first != 0 or sequence != 0 or kind not in (0, 1, 3):
                    raise RuntimeError(
                        f"PS-FTP response start invalid: first={first}, kind={kind}, seq={sequence}"
                    )
                if kind == 0:
                    if len(fragment) < 2:
                        raise RuntimeError("PS-FTP error response omitted its error code")
                    error = int.from_bytes(fragment[:2], "little")
                    raise RuntimeError(f"PS-FTP response error {error}")
                first_packet = False
            elif first != 1 or sequence != expected_sequence:
                raise RuntimeError(
                    f"PS-FTP response sequence invalid: first={first}, kind={kind}, seq={sequence}, expected={expected_sequence}"
                )
            payload.extend(fragment)
            expected_sequence = (sequence + 1) % 16
            if kind == 1:
                return bytes(payload)
            if kind != 3:
                raise RuntimeError(f"PS-FTP response has unsupported frame kind {kind}")


async def write_framed(client: BleakClient, characteristic: str, message: bytes) -> None:
    # Flow starts at an ATT MTU of 23, therefore a safe 20-byte GATT payload.
    # The reset and initialization messages are single-frame, but the framing
    # remains general for the USERID read request.
    mtu = max(23, int(getattr(client, "mtu_size", 23)))
    mtu_payload = max(20, min(244, mtu - 3))
    for frame in frame_message(message, mtu_payload):
        await client.write_gatt_char(characteristic, frame, response=False)


async def send_notification(client: BleakClient, notification_id: int, payload: bytes = b"") -> None:
    await write_framed(client, PSFTP_H2D, bytes((notification_id,)) + payload)


async def read_userid(client: BleakClient, responses: asyncio.Queue[bytes]) -> None:
    operation = protobuf_get_operation("/U/0/USERID.BPB")
    request = len(operation).to_bytes(2, "little") + operation
    await write_framed(client, PSFTP_MTU, request)
    response = await receive_message(responses, 10.0)
    # C8306a.m36856A exposes successful response payload bytes directly; the
    # Java operation wrapper adds its own four zero status bytes only after
    # receiving them. The wire response therefore contains the USERID.BPB
    # protobuf, not a four-byte status prefix.
    state = user_identifier_registration_state(response)
    print(
        f"USERID.BPB read succeeded; payload bytes={len(response)}; "
        f"registration_state={state}"
    )


def print_hr(data: bytearray) -> None:
    if not data:
        return
    flags = data[0]
    if flags & 1:
        if len(data) < 3:
            print(f"HR packet rejected: short UINT16 packet ({len(data)} bytes)")
            return
        bpm = int.from_bytes(data[1:3], "little")
        index = 3
    elif len(data) >= 2:
        bpm = data[1]
        index = 2
    else:
        print("HR packet rejected: short UINT8 packet")
        return
    print(f"HR notification: bpm={bpm}, raw_len={len(data)}, flags=0x{flags:02x}")
    if flags & 0x10:
        if flags & 0x08:
            index += 2
        rr = []
        while index + 1 < len(data):
            rr.append(int.from_bytes(data[index : index + 2], "little") / 1024.0)
            index += 2
        print(f"  RR intervals (seconds): {rr}")


async def run(args: argparse.Namespace, candidate: Candidate) -> None:
    responses: asyncio.Queue[bytes] = asyncio.Queue()
    events: asyncio.Queue[bytes] = asyncio.Queue()

    def on_response(_characteristic: Any, data: bytearray) -> None:
        responses.put_nowait(bytes(data))

    def on_event(_characteristic: Any, data: bytearray) -> None:
        events.put_nowait(bytes(data))

    def on_hr(_characteristic: Any, data: bytearray) -> None:
        print_hr(data)

    if BleakClient is None:
        raise RuntimeError(_BLEAK_IMPORT_ERROR or "Bleak is unavailable")
    print(f"Connecting to {candidate.name} ({short_address(candidate.address)})…")
    async with BleakClient(candidate.device, timeout=args.connect_seconds) as client:
        if not client.is_connected:
            raise RuntimeError("BLE connection did not become ready")
        print("Connected without Polar Flow or Polar Cloud.")
        await client.start_notify(PSFTP_MTU, on_response)
        await client.start_notify(PSFTP_D2H, on_event)
        hr_available = any(
            str(service.uuid).lower() == HR_SERVICE
            and any(str(char.uuid).lower() == HR_MEASUREMENT for char in service.characteristics)
            for service in client.services
        )
        if hr_available:
            await client.start_notify(HR_MEASUREMENT, on_hr)
        print("PS-FTP notifications enabled on FB005C51 and FB005C52.")

        if args.operation == "initialize":
            print("Sending INITIALIZE_SESSION notification id=8…")
            await send_notification(client, INITIALIZE_SESSION)
        elif args.operation == "read-userid":
            await read_userid(client, responses)
        elif args.operation == "factory-reset":
            payload = protobuf_factory_reset(sleep=False, ota_fwupdate=args.ota_fwupdate)
            print(f"Sending Flow-equivalent FACTORY RESET id=2 payload={payload.hex()}…")
            await send_notification(client, RESET, payload)
        elif args.operation == "reset":
            payload = protobuf_factory_reset(
                sleep=args.sleep,
                do_factory_defaults=False,
                ota_fwupdate=False,
                hibernate=args.sleep,
            )
            print(f"Sending Flow-equivalent RESET id=2 payload={payload.hex()}…")
            await send_notification(client, RESET, payload)

        if args.observe_seconds > 0:
            print(f"Observing HR/PS-FTP output for {args.observe_seconds:g}s…")
            deadline = asyncio.get_running_loop().time() + args.observe_seconds
            while asyncio.get_running_loop().time() < deadline:
                remaining = deadline - asyncio.get_running_loop().time()
                try:
                    packet = await asyncio.wait_for(events.get(), timeout=remaining)
                except asyncio.TimeoutError:
                    break
                try:
                    first, kind, sequence, fragment = decode_frame(packet)
                    print(
                        f"PS-FTP D2H event: first={first}, kind={kind}, seq={sequence}, bytes={len(fragment)}"
                    )
                except ValueError as exc:
                    print(f"PS-FTP D2H event rejected: {exc}")
        print("Disconnecting.")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--operation",
        choices=("initialize", "read-userid", "factory-reset", "reset"),
        default="initialize",
    )
    parser.add_argument("--address", help="Exact H10 address from a prior scan")
    parser.add_argument("--scan-seconds", type=float, default=15.0)
    parser.add_argument("--connect-seconds", type=float, default=20.0)
    parser.add_argument("--observe-seconds", type=float, default=15.0)
    parser.add_argument(
        "--ota-fwupdate",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="Factory-reset ota_fwupdate field; default matches Flow's false value",
    )
    parser.add_argument(
        "--sleep",
        action="store_true",
        help="For non-factory reset, request sleep/hibernate=true",
    )
    parser.add_argument(
        "--i-understand-data-loss",
        action="store_true",
        help="Required for factory-reset and reset; the sensor may erase unsynced data",
    )
    return parser.parse_args()


def validate(args: argparse.Namespace) -> None:
    if args.scan_seconds <= 0 or args.connect_seconds <= 0 or args.observe_seconds < 0:
        raise ValueError("scan/connect timeouts must be positive and observe time cannot be negative")
    if args.operation in {"factory-reset", "reset"} and not args.i_understand_data_loss:
        raise ValueError(
            f"{args.operation} blocked: --i-understand-data-loss was not supplied"
        )


def self_test() -> None:
    assert protobuf_factory_reset(sleep=False, ota_fwupdate=False) == bytes.fromhex("08001800")
    assert protobuf_factory_reset(
        sleep=False, do_factory_defaults=False, ota_fwupdate=False, hibernate=False
    ) == bytes.fromhex("0800100018002000")
    assert protobuf_get_operation("/U/0/USERID.BPB") == bytes.fromhex(
        "0800120f2f552f302f5553455249442e425042"
    )
    assert user_identifier_registration_state(b"\x08\x01") == "not_registered"
    assert user_identifier_registration_state(b"\x1a\x01\x00") == "registered"
    assert user_identifier_registration_state(b"\x1a") == "unknown"
    assert frame_message(bytes(range(25)), 20) == [
        bytes.fromhex("06000102030405060708090a0b0c0d0e0f101112"),
        bytes.fromhex("13131415161718"),
    ]


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        self_test()
        print("self-test: ok")
        raise SystemExit(0)
    arguments = parse_args()
    try:
        validate(arguments)
        candidates = asyncio.run(scan_h10s(arguments.scan_seconds))
        selected = choose_candidate(candidates, arguments.address)
        if arguments.operation in {"factory-reset", "reset"}:
            print(
                f"WARNING: {arguments.operation} may erase unsynced H10 data; "
                "no Polar account or cloud operation will be performed."
            )
        asyncio.run(run(arguments, selected))
    except KeyboardInterrupt:
        print("\nStopped.")
        raise SystemExit(130)
    except (RuntimeError, ValueError, OSError) as exc:
        print(f"h10-psftp-offline: {exc}", file=sys.stderr)
        raise SystemExit(1)
