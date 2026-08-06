# Scheduled restore verification

Installed host artifacts for T-C3.

## Installation

Install `verify-restore.sh` as `/usr/local/bin/verify-restore.sh` with owner `root:root` and mode
`0755`. Install the five unit files in `/etc/systemd/system/` with owner `root:root` and mode `0644`.
Create the fixed parents before starting the service:

```sh
sudo install -d -o root -g postgres -m 0710 \
  /var/lib/vitals/restore-work /run/vitals-restore
sudo systemctl daemon-reload
# Enable only after the authorized non-vacuous fixture proof has created durable G3 evidence:
sudo systemctl daemon-reload
sudo systemctl enable --now verify-restore.timer verify-restore-staleness.timer
```

The service creates an ext4 image and mounts it in its private mount namespace. Restored PGDATA and
all remapped tablespaces remain inside that disposable filesystem. The production data directory is
read-only in the service namespace. Empty-schema runs are accepted only while retained, non-vacuous G3 evidence remains valid; they
cannot establish G3.

## Schedule

`verify-restore.timer` runs at 04:00 on the first Sunday of each month. The staleness timer runs daily
and fails if the latest result is missing, malformed, unsuccessful, future-dated by more than five
minutes, or older than 45 days.

## Result

The verifier writes `/var/lib/vitals/restore-verify.json` atomically and retains up to 13 historical
results in `/var/lib/vitals/restore-verify-history/`. A successful non-vacuous result is also
atomically preserved as `/var/lib/vitals/restore-verify-g3.json`, outside rotating history. The
empty-schema run and its daily staleness check fail unless that durable artifact remains valid. A
successful result records the fresh full backup
label, the locked production evidence LSN, table count comparisons, extension comparison, recovery
LSN check, and `pg_amcheck` completion.
