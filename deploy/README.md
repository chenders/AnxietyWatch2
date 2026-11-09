# Deployed units

Every systemd unit that runs on the Pi should have its definition here. For a long time eleven of
them did not — including the entire backup-verification path — so a change to them was a hand-edit on
one box with no review, no history, and no way to rebuild after a failure. This directory is now the
full set, captured 2026-11-09.

## Why this mattered

The gap was found while debugging two failed units. `verify-restore.service` had failed on
2026-11-05, and `verify-restore-staleness.service` — the unit whose whole job is to assert that
restoration was verified within 45 days — had been failing since. Both guard 44 GB of irreplaceable
physiological history. Neither existed in this repo.

`vitals-cold-export.service` had also failed on 2026-11-05, with a real permissions bug: it runs as
`postgres`, which owns `/var/lib/vitals/cold`, but is not in the `vitals` group and so could not
*traverse* `/var/lib/vitals` (mode `770 vitals:vitals`) to reach it. The fix is a traverse-only ACL,
matching one already present for the operator account:

```sh
setfacl -m u:postgres:--x /var/lib/vitals
```

That ACL is **not** captured by anything declarative. Neither was the operator's. If
`/var/lib/vitals` is ever recreated, the monthly cold export breaks again and says only
"Permission denied".

## What is here

| Unit | Enabled | Notes |
|---|---|---|
| `vitalsd.service` | yes | daemon |
| `vitals-console.service` | yes | HTTP console |
| `vitals-spool-drain.service` | yes | spool → Postgres |
| `vitals-spool-seal.service` + `.timer` | timer | seals the active segment so rows become visible |
| `vitals-ble-broker.service` | yes | per-adapter BLE lease broker |
| `vitals-ble-broker-air11.service` | yes | dedicated broker instance for the `AirSense` |
| `vitals-air11.service` | yes | `AirSense` 11 collector |
| `vitals-h10.service` | yes | Polar H10 collector |
| `vitals-bp.service` | yes | A&D UA-651BLE collector |
| `vitals-bp-autoconnect.service` | — | registers kernel auto-connect |
| `vitals-bp-connect.service` | — | one-shot connect helper |
| `vitals-emay.service` | yes | EMAY SleepO2 collector — **see caveats** |
| `emay-live.service` | no | earlier EMAY spike (`emay-spike`), superseded |
| `wellue-collector.service` | no | **retired device**; id 4 — see amendment 20 |
| `vitals-bt-up.service` | no | Bluetooth bring-up helper |
| `silence-detector.service` | yes | source-health evaluator |
| `verify-restore.service` + `.timer` | timer | monthly isolated pgBackRest restore verification |
| `verify-restore-staleness.service` + `.timer` | timer | asserts the above ran and passed within 45 days |
| `verify-restore-alert.service` | `OnFailure` | alert hook for both of the above |
| `vitals-cold-export.service` + `.timer` | timer | monthly immutable self-contained export |

Their scripts are in [`../scripts/pi/`](../scripts/pi/).

## Caveats recorded with the capture

**`vitals-emay.service` has no source.** Its `ExecStart=/usr/local/bin/emay-collector` is a stripped
ARM64 binary dated 2026-09-25 that is built by neither this repo nor gen-1, and `vitals-deploy` does
not install it. `emay-spike` is a different program and does not produce its log messages. The
collector feeding the clinical record cannot currently be rebuilt or patched from source.

**It also crash-loops.** When the device is not advertising it prints a correctly-named blocker —
`EMAY device <addr> was not discovered before the 15s deadline` — then exits 70. With
`Restart=always` and `RestartSec=20` that reached **NRestarts=12,045**. Between crashes
`systemctl is-active` reports `active`, so the unit looks healthy while collecting nothing: §2.1's
"running and useless is indistinguishable from working", live. A device that only advertises while
worn should report absence as a steady named state and back off, not restart every twenty seconds.

**Two units run as a human login account.** `vitals-emay.service` and `emay-live.service` use the
operator's account; every other unit here runs as `vitals` or `postgres`. Both files are therefore
sanitized templates and carry a header saying so — `check-public-info.py --fix` rewrote the account
to `pi-user` and the home directory to `$HOME`, and **systemd does not expand `$HOME`** in
`ReadWritePaths=` or `Environment=`. Deployed verbatim they would write to a directory literally
named `$HOME` and the panel status file would stop updating silently.

## Keeping this honest

These files were captured from a running host, so they are a snapshot, not a proof. Nothing currently
verifies that what runs on the Pi still matches what is here. Until something does, treat a
difference as a question about which one is wrong rather than evidence that the repo is right.
