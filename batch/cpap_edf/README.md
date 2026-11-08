# CPAP EDF semantics

Decodes EDF files mirrored from a ResMed AirSense 11 SD card into canonical,
range-checked samples. It **writes nothing**: no database, no SQL, no network, no systemd. The
output is plain data for T-CP3 to insert transactionally.

Stdlib only — `dataclasses`, `hashlib`, `pathlib`, `re`, `tomllib`. See `NOTICE` for why no
third-party EDF library is used; the short form is that these bytes come from an untrusted card over
unauthenticated HTTP, and a compiled C extension in that path would reverse the property
`batch/cpap_ezshare` exists to establish.

## Modules

| | |
|---|---|
| `edf.py` | the format reader: header, signal headers, digital samples. Every field that sizes a later read is bounded *before* it is used. |
| `channels.py` | ResMed label → registry type, and EDF physical dimension → canonical unit. The only place a unit conversion happens. |
| `identity.py` | `session_id` and `seq`, the two identity contracts. |
| `clock.py` | the versioned device-clock transform, applied at read time. |
| `decode.py` | ties them together and enforces the registry range. |
| `registry.py` | the one place `vitals/registry.toml` is opened. |
| `fixtures.py` | deterministic synthetic EDF builder for tests. Never a real recording. |

## Three decisions worth knowing

**Units come from the file, not from the source.** `[source.resmed_air11]` in the registry carries
`conversion = { leak = 60.0, tidal_volume = 1000.0 }`, because the BLE wire format states no units
and a fixed factor is the only option there. EDF states a physical dimension per signal, so the file
is the authority and an unrecognised dimension is a **rejection, never a guess**. The failure this
avoids is specific: if ResMed ever writes leak in L/min where it used to write L/s, a fixed 60×
factor turns a 24 L/min leak into 1440, and a mis-scaled leak reading is indistinguishable from a
real one downstream.

A pressure in hPa is rejected for the same reason even though it is a legitimate quantity — the
cmH₂O-per-hPa factor would be a number this project had not verified against a device, and an
unverified conversion on a therapy pressure is worse than a refusal that names itself.

**`session_id` groups a night, not a file.** One ResMed night is several files — `BRP`, `PLD`,
`SAD` — in one `DATALOG/<date>` directory. Keying on the file hash would split one night into four
and make a cross-channel query impossible, so the id derives from the directory and the recording
start date, both immutable for a given recording.

The assumption — *one directory and date holds one session* — is written down so it can be
falsified, and `identity.session_conflicts()` is the check that falsifies it: it reports files
sharing a session key whose start times are further apart than a night plausibly spans. T-CP3 should
run it against real card data before trusting the grouping.

**`seq` is milliseconds from the session start, not a sample ordinal.** A running ordinal breaks as
soon as two files in one session carry the same type: `MaskPres` appears in both `BRP` and `PLD` at
different rates, and two independent ordinals would both start at 0 while describing different
instants. A time-derived ordinal collides only for samples of the same type at the same instant,
which *are* the same measurement and *should* deduplicate.

## Clock correction is metadata, and "uncorrected" is a version

Raw device time is stored as read. The correction is a versioned transform applied at read time, and
the transform currently in force is the **identity** — because the AirSense has no network time
source and no device-versus-reference observation has been taken yet. That is T-CP6's physical gate.

It is deliberately version 1 rather than a null transform. A stored sample must always name the rule
that interpreted it, or a later reader cannot distinguish *no correction was needed* from *none was
recorded*. And the correction cannot be applied at write time: `samples` is append-only, so baking a
−94 s offset into stored timestamps could never be undone when the offset turns out to be wrong.

## Out of range contributes nothing

A value outside the registry range is **never clamped**. It is dropped and the rejection names the
digital value, the scaled value, the unit and the bounds. A clamped reading looks exactly like a
real one downstream, which is the whole reason the rule exists.

Likewise, unmapped channel labels (`Ti`, `Te`, `TgMV`, `Resp Event` — no registry type exists for
them yet) are **reported**, not silently skipped, so a firmware rename shows up as a named
observation rather than a quietly shorter night.

### One honest gap

EDF carries **no in-band sentinel** for "no data" — unlike the IEEE-11073 SFLOAT reserved values the
BLE decoders handle. So absence here is detectable only as an out-of-range value after scaling. If
the AirSense turns out to write a specific digital marker for absent samples, that is a finding for
T-CP3 against real data, and it would need its own rejection path. The current behaviour is
conservative: an unrecognisable value contributes nothing either way.

## Tests

```sh
python3 -m unittest discover -s batch/cpap_edf/tests
```

68 tests. Expected physical values are **derived by hand from the EDF scaling formula**, never from
this package's own output — a golden file produced by the parser under test would agree with any bug
the parser has. Fixtures use asymmetric digital bounds (digital 0..100 onto physical 0..50) so every
expectation is exactly `digital × 0.5` and can be checked mentally. Float comparisons use a
tolerance, never equality.
