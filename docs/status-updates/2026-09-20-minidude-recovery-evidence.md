# Minidude recovery evidence — 2026-09-20

## Recovery artifact

Direct-to-Mac recovery directory:

`~/minidude-reboot-evidence/20260920T073718Z/recovery/`

Captured artifacts:

- encrypted PostgreSQL globals;
- encrypted custom-format PostgreSQL 16 logical dump;
- encrypted WAL/recovery-boundary evidence;
- encrypted pending spool archive;
- checksums and local readability evidence.

The database archive contains 754 `pg_restore --list` entries when checked with the PostgreSQL 16
client. The PostgreSQL 15 client on the Mac initially rejected the PostgreSQL 16 archive; this was a
client-version mismatch, not a dump failure. The spool archive lists successfully. The backup is a
logical recovery set, not a verified physical pgBackRest restore.

## Immediate host evidence

- Current boot began at `22:17:01` host time and the first capture occurred at `22:22:23`.
- `last -x` shows a prior `crash` session and a rapid reboot history.
- Root is on `/dev/nvme0n1p1`, with 3.4 TB available and 2% used.
- Temperature was `30.7'C`; `throttled=0x0`.
- PostgreSQL was reachable and had only three connections after the incident controls.
- The watchdog service reported inactive/infinity; this is evidence to investigate, not permission to
  disable hardware watchdog protection.
- The journal reported unclean/corrupted journal replacement after the prior crash.

## Confirmed restart storms

The prior boot journal shows repeated service failure/restart activity:

- `vitals-console.service`: approximately 16,975 restart attempts; it repeatedly failed to bind
  `192.168.1.48:8480` with `Cannot assign requested address`.
- user `cage-status.service`: approximately 24,390 restart attempts.
- `vitals-emay.service`: restart counter approximately 2,708; EMAY target was absent/removed.
- `vitals-bt-up.service`: repeated failures approximately every 30 seconds.
- `panel-kiosk.service`: repeated failures because no connected DSI connector was present.
- `nvmf-autoconnect.service`: failed at boot.
- `panel-weather-poll.service`: failed.

This is substantially stronger evidence for service-thrash/resource amplification than temperature,
NVMe capacity, or database volume alone.

## Controls applied

After evidence capture and recovery streaming, the following were stopped and runtime-masked to
prevent reactivation during the incident:

- collectors, `vitalsd`, and spool drainer;
- `vitals-console`, panel HTTP/admin/kiosk/status services;
- aircraft/noise/FR24/weather/panel timers and services;
- EMAY recovery timer/service;
- `cage-status` user service.

No schema migration or destructive database cleanup has run.

## Database size findings

The largest relation is:

- `public.samples`: approximately 6.97 GiB, estimated 15.97 million rows;
- `public.spool_status`: approximately 67 MiB.

Plane/noise tables are already effectively empty (`n_live_tup = 0`) and together occupy only about
3–4 MiB of relation/index space. Removing them would not address the 7 GiB database or the restart
storm.

The dominant sample source is `source = 5`, `resmed_air11`:

- type 10 `patient_flow`: 7,378,313 rows;
- type 11 `mask_pressure`: 7,378,311 rows;
- other Air11 types add approximately 0.85 million rows.

The source-5 Air11 data is therefore the actual database-size candidate. It is CPAP data and is
represented in the broader CPAP/dashboard design as well as being relevant to the CNS-warning design;
it cannot be classified as CNS-exclusive solely from its source name. A destructive predicate must
be chosen deliberately if this data is to be removed.

A read-only attempt to group all 16 million samples by source/type was cancelled after it spawned a
parallel scan and remained on `DataFileRead`; no data was changed. Exact source-5 counts were then
obtained using the indexed predicate.

## Critical storage finding

After the recovery capture, the kernel reported a real NVMe controller failure event:

- multiple read timeouts and 4 KiB write timeouts;
- controller reset;
- repeated `I/O error, dev nvme0n1` messages;
- NVMe link currently downgraded to `5GT/s x1` rather than the device's x4 capability;
- ASPM L1 enabled;
- kernel explicitly suggested investigating power-saving mode;
- root filesystem remained mounted `rw` after recovery;
- SMART currently reports 31°C, 0% used, 0 media errors, and 0 persistent NVMe error-log entries;
- unsafe shutdown count is 311.

This changes the immediate diagnosis. The live instability is not primarily database size. The Pi's
boot disk/controller path is timing out under I/O and resetting. The service restart storms amplified
the damage, but deleting rows now would generate additional NVMe writes on a controller that has
already failed I/O. PostgreSQL has been stopped cleanly and all optional/collector services remain
stopped and runtime-masked.

No delete, vacuum-full, migration, filesystem repair, NVMe reset, power-management change, or reboot
has been attempted after this finding. The next action is to preserve the NVMe/PCIe/power evidence
and move the recovery decision to a known-good storage path. A second NVMe may be a replacement or
boot-disk migration requirement, not merely a load-balancing optimization.

## Current next action

Keep the incident services and PostgreSQL stopped. Do not perform database cleanup on the failing
root device. The direct recovery artifacts are present on the Mac, but the source-5 Air11 rows should
not be treated as the immediate cause until the NVMe path is stable or the database is moved to a
known-good device.