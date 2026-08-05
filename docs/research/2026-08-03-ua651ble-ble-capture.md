# A&D UA-651BLE — live BLE capture

**Date:** 2026-08-03
**Hardware:** A&D UA-651BLE (`00:09:1F:XX:XX:XX`), Sena UD100-G03 (CSR8510) on the Pi
**Kernel:** `6.12.96+rpt-rpi-2712` · **BlueZ:** 5.66
**Feeds:** T-D6 (`bp-collector`) · **produced amendment 06**, which supersedes A05 §5 and extends §4.3

The first real physiological data captured by this system. Four measurements drained from the
cuff's internal buffer over one connection, decoded live.

> **Note on identifiers and values.** The device address is redacted to its A&D OUI
> (`00:09:1F:XX:XX:XX`) — the vendor half is not identifying, the device half is, and it is
> rediscoverable with `sudo btmgmt find` in seconds, so recording it bought nothing.
>
> The byte layout is the hardware's, kept verbatim, and the readings are substituted: a fixture whose provenance is "I made
> it up from the spec" is exactly what produced the `0xF14A → 7.4` error in T-D2's brief. They carry
> no identity on their own. If this repo goes public, swap the mantissas and keep the flags and
> lengths — the byte *layout* is what the tests exercise.

---

## The frames

All four are 11 bytes, `flags = 0x14`:

| # | Raw hex | Systolic | Diastolic | MAP | Pulse |
|---|---|---|---|---|---|
| 1 | `1476004c005a003e000000` | 118 | 76 | 90 | 62 |
| 2 | `147a004e005d0040000000` | 122 | 78 | 93 | 64 |
| 3 | `1484005600650044000000` | 132 | 86 | 101 | 68 |
| 4 | `14780050005d0046000000` | 120 | 80 | 93 | 70 |

`status` is a 16-bit field (body movement, cuff fit, irregular pulse, among others); the fixtures use `0x0000`.

### Flags byte

```
0x14 = 0b0001_0100
  bit0  units             0  → mmHg              (never saw kPa)
  bit1  timestamp         0  → ABSENT
  bit2  pulse rate        1  → present
  bit3  user id           0  → ABSENT
  bit4  measurement status 1 → present
```

Layout confirmed against the SIG spec: flags, then systolic/diastolic/MAP as three SFLOATs,
then the optional fields in order. With `0x14` the payload is
`1 + 6 + 2 (pulse) + 2 (status) = 11` bytes, matching exactly, with zero trailing bytes.

**The SFLOATs are all exponent 0** (`0x0076` = mantissa 118, exponent 0). No negative exponent
appeared, so the exponent path is unexercised by this capture — it still needs synthetic coverage.

---

## Findings that change the spec

### 1. There is no measurement sequence number · **contradicts T-D6 and A05 §5**

Amendment 05 §5 says T-D6 should take `seq` "from the cuff's own measurement sequence, not
arrival." **The cuff transmits no such sequence.** No timestamp, no user ID, no counter. Nothing
in the payload distinguishes these four readings from a retransmission of the same four.

The identity has to come from the transport instead — see finding 2. **Resolved in amendment 06
§1:** `seq` is the drain index within one connection, and A05 §1.1's "never from host timing" rule
is narrowed to its actual scope (streaming sources), because a *confirmed* store-and-forward
delivery cannot be redelivered.

**Worth testing before accepting this:** A&D devices expose a clock, and the timestamp bit may be
zero only because the clock is unset. If writing the current time (Date Time `0x2A08`, or the
vendor service `233bf000-5a34-1b6d-975c-000d5690abe4`) causes bit1 to flip, the identity problem
dissolves and the collector gets real event times instead of arrival estimates. **Not yet tested.**

### 2. Indication *is* the dedup mechanism — and it hands us a durability gap

An indication requires an ATT Handle Value Confirmation, and the cuff deletes a stored record only
once confirmed. The snoop shows 4 indications and 4 confirmations. So delivery is exactly-once and
identity is not needed in the payload.

The hazard: **BlueZ sends the confirmation from the kernel, before the application has durably
written anything.** A crash between the kernel's ACK and the spool write destroys the reading
permanently — it is already gone from the cuff.

Every other source in this system is reconstructible (HealthKit re-read, sensor re-derivation).
This one is not. Same class as the `QuantityHealthSample` restore hole: app-only data with nothing
upstream to rebuild it.

`bp-collector` must therefore treat the confirmation as a commit point it does not control.

**Resolved in amendment 06 §2.1.** The obvious instruction — "fsync the spool segment on every
frame" — is unbuildable: `vitalsd` is the single writer and owns the spool, and the collector's only
channel is the CBOR socket. So `append` gains `sync: true`, which makes `vitalsd` fsync before
replying, set by `bp-collector` and by nothing else. A night of RR is ~250 MB and ~100k appends;
fsyncing those would destroy the ingest path to protect data that is already reconstructible.
Durability cost is paid where loss is unrecoverable and nowhere else. `bp-collector` also journals
the raw bytes before decoding, so a decoder bug is replayable even though a lost frame is not.

### 3. `bluetoothctl connect` issues no LE Create Connection at all

Confirmed against `btmon`: the cuff produced **269 advertising reports** while the ATT trace stayed
**completely empty** and no `LE Create Connection` was ever sent. Every `connect` was a silent no-op.

The cuff advertises in ~5–10 second bursts after a measurement and is dark otherwise. A userspace
poll cannot reliably catch that window. The fix is to register intent with the kernel:

```sh
btmgmt add-device -a 2 -t 1 00:09:1F:XX:XX:XX   # action 2 = auto-connect
```

The kernel then background-scans and connects the instant the device advertises. **T-D6's "connect
to the cuff over BlueZ" is wrong** — it assumes a connectable-on-demand peripheral. A collector
built on `connect()` works in testing, where you trigger the cuff deliberately, and captures
nothing unattended.

### 4. Reconnection to an existing bond fails on this kernel

```
[hh:mm:ss] Bluetooth: hci0: unexpected SMP command 0x0b from 00:09:1F:XX:XX:XX
```

SMP `0x0B` is Security Request — a bonded peripheral asking the central to resume encryption from
the stored LTK. Kernel 6.12.96 rejects it as arriving in an invalid state, so encryption never
starts, so GATT never opens. Known issue: [bluez#581](https://github.com/bluez/bluez/issues/581),
[Victron writeup](https://community.victronenergy.com/t/kernel-fix-fix-for-unexpected-smp-command-0x0b-bluetooth-le-connection-failures/51883).

The bond was **not** at fault — `/var/lib/bluetooth/<adapter>/<cuff>/info` held `[LongTermKey]`,
`[PeripheralLongTermKey]`, `[IdentityResolvingKey]` and `[ConnectionParameters]`.

Removing the bond and pairing fresh sidesteps the path entirely, because a fresh pairing
establishes encryption as part of the pairing procedure and never sends a Security Request.

**This adds a third device state to amendment 05 §4.3**, which distinguished only "device not found"
(pairing fault) from "device out of range" (normal). The third is **bonded but unable to resume
encryption** — indistinguishable from a healthy idle system until you notice no data has arrived for
days. **Specified in amendment 06 §3**: a named degraded state, detected as "bonded device reached
`Connected` but `ServicesResolved` never fired within 10 s."

### 5. The IRK is stored, so A05 §4.3 is satisfiable today

`LegacyPairing: no` — bonded via LE Secure Connections, and `[IdentityResolvingKey]` is present.
The cuff advertises a **public** address (A&D OUI `00:09:1F`) and does not rotate it, but the
collector should still resolve by IRK. "It doesn't rotate" is a property of this firmware revision,
not of the protocol.

### 6. Store-and-forward buffering works as briefed

Four measurements taken across ~25 minutes were all delivered on one later connection, in order.
T-D6's "the cuff buffers ~30 readings internally, so the collector drains its memory on connect
rather than requiring presence at measurement time" is confirmed.

---

## Working procedure

What actually captures data, after several approaches that did not:

```sh
# 1. Fresh bond (avoids the SMP 0x0b reconnect path)
bluetoothctl remove 00:09:1F:XX:XX:XX
#    hold START on the cuff until the display reads "Pr" — 60-second window
/tmp/pair-cuff.sh 00:09:1F:XX:XX:XX

# 2. Register auto-connect so the kernel catches the advertising burst
btmgmt add-device -a 2 -t 1 00:09:1F:XX:XX:XX

# 3. Watch via the BlueZ D-Bus API; StartNotify() picks the CCCD bit from
#    the characteristic's own flags. Frames are fsynced per frame to
#    BP_CAPTURE_LOG (default ~/bp-capture.jsonl unprivileged) BEFORE printing --
#    the kernel has already ACKed and the cuff has already deleted its copy, so
#    stdout is not storage.
BP_CAPTURE_LOG=~/bp-capture.jsonl python3 -u tools/bp_capture.py 00:09:1F:XX:XX:XX

# 4. Take a measurement. Readings drain automatically.
```

`bluetoothctl` was unusable for this device and failed three distinct ways: empty scan output when
piped, `connect` that issued no HCI command, and `menu gatt` dropping into the main menu. The
D-Bus client replaced it and is the shape `bp-collector` should take.

---

## Still open

- **Does setting the cuff's clock enable timestamps?** Highest-value follow-up — it would give
  real event times and resolve finding 1.
- The vendor service `233bf000-5a34-1b6d-975c-000d5690abe4` is unexplored.
- `0x2A36` Intermediate Cuff Pressure was never observed; the cuff may not implement it.
- kPa frames, negative SFLOAT exponents and measurement-status bits all need synthetic
  fixtures; a capture cannot be relied on to produce them.
