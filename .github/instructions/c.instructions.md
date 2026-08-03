---
applyTo: "**/*.{c,h}"
---

# C review rules — AnxietyWatch2

This code runs unattended all night and is the only thing standing between a desaturation and an
alarm. Review it that way.

## Radio decoders — the highest-risk code here

Any function parsing bytes off Bluetooth (`0x2A35` blood pressure, `0x2A37` heart rate, EMAY frames,
Oura frames) sees **attacker-controlled input**. A malformed advertisement or truncated GATT
indication is not hypothetical — it is what BlueZ hands you when a device browns out mid-packet.

Required shape, flag any deviation:

```c
bool decode_x(const uint8_t *buf, size_t len, result_t *out);
```

- **Pure.** No allocation, no I/O, no global or static state, no logging.
- **Every read bounds-checked against `len` before it happens.** Flag any indexing that assumes a
  minimum length without checking it first, including the flags byte.
- **Length checks before each optional field**, not just once at the top. Optional-field layouts
  mean the required length depends on the flags byte.
- **Returns `false` on malformed input** and leaves `*out` untouched. Never partially populated.
- **Has a libFuzzer target** in `vitals/fuzz/`. A new decoder without one is incomplete.

## Numeric hazards

**IEEE-11073 SFLOAT is not IEEE-754 half-float.** 4-bit signed exponent in the high nibble, 12-bit
signed mantissa below. Flag any `_Float16` cast, `memcpy` into a float, or bit-reinterpretation — it
produces plausible-looking wrong numbers, and 118/74 versus 141/92 are both believable.

**Sign-extension.** 4-bit and 12-bit signed fields must be explicitly sign-extended. Flag arithmetic
on a raw masked field without it.

**Reserved sentinels are absent, not zero.** `0x07FF` NaN, `0x0800` NRes, `0x07FE`/`0x0802` ±INF,
`0x0801` Reserved. Flag any path where one reaches `*out` as a value.

**Never compare floats with `==`.** Tests use `fabs(a - b) < 1e-9`. Flag exact equality on `double`.

**`-Wconversion` is on.** Flag implicit narrowing that was silenced with a cast rather than fixed.

## Integer and pointer safety

- Flag `int` where the value comes from a wire field. Use sized types.
- Flag arithmetic on `size_t` that could underflow — `len - 2` when `len` might be 0 or 1.
- Flag any `strcpy`, `sprintf`, `strcat`, `alloca`, or unbounded `scanf`.
- Flag missing `NULL` checks on out-parameters in public functions.

## Concurrency

`vitalsd` is a **single-writer, single-threaded epoll loop**. That is the design, and it is why
there is no locking. Flag:

- Any thread creation in the daemon.
- Any shared mutable state introduced between collectors and the store.
- Any assumption that two collectors can write the database directly — they append through the
  socket; `vitalsd` alone touches SQLite.

## Portability

Two libraries, and the boundary is load-bearing:

| Library | Contains | Builds on |
|---|---|---|
| `vitals_core` | decoders, HLC — pure C11 | macOS **and** Linux |
| `vitals_linux` | SQLite, `vitalsd`, epoll, BlueZ, systemd | Linux only |

Flag any Linux header (`sys/epoll.h`, `systemd/sd-daemon.h`, `sys/random.h`, `dbus/`) reaching a
`vitals_core` source. Flag any `#ifdef __APPLE__` shim that fakes a Linux API — a mocked BlueZ tests
the mock. Linux-only work belongs in `vitals_linux` and runs on the Pi.

## Storage

- No `UPDATE` or `DELETE` against `samples`. Append-only; removal is a tombstone.
- `INSERT ... ON CONFLICT DO NOTHING` so append is idempotent on `(source, type, timestamp)`.
- Range validation against the registry happens **before** insert. Out-of-range goes to quarantine
  with a reason, never clamped, never dropped silently.
- Flag any SQL string built by concatenation. Prepared statements with bound parameters only.

## Time

- `timestamp` is a `double` holding **UTC epoch seconds**. Flag `time_t` arithmetic assuming local
  time, `localtime()`, or any timezone conversion below presentation.
- Flag monotonic and wall clocks being mixed. HLC physical time is wall-clock milliseconds;
  durations use a monotonic source.

## Error handling

- Flag any ignored return value from `sqlite3_*`, `read`, `write`, `close`, or `malloc`.
- Flag a caught-and-logged error that then proceeds as though it succeeded.
- Flag `assert()` used for runtime validation of external input — asserts compile out in release.
  Validate and return an error instead.

## Style

`.clang-format` handles layout; do not flag it. Do flag:

- Comments that explain *what* rather than *why*.
- A doc comment that no longer matches the code — especially after a threshold, filter, or sort
  order changes.
- Magic numbers used in two or more places. They belong in `vitals/registry.toml` and reach C
  through generated constants.
