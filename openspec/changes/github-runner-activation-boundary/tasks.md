# Tasks — GitHub runner activation write boundary

**Status:** Proposed; no task is complete until its exact `Verify:` command exits 0.

**Amends:** `docs/specs/AMENDMENTS.md`; the accepted GitHub Actions delivery design and the
owner-approved OpenSpec authoring guidance. This change does not alter physiological, storage,
alerting, or database requirements.

- [x] T-GRAB-1 — Specify the sandbox-compatible activation boundary · documentation · host either
- [x] T-GRAB-2 — Add the narrowly allowlisted runner write paths · implement-standard · host either
- [x] T-GRAB-3 — Install, verify, and exercise the boundary on `minidude` · human · Pi

## T-GRAB-1 — Specify the sandbox-compatible activation boundary

**Reads:** `AGENTS.md`, `docs/specs/AMENDMENTS.md`, `openspec/config.yaml`,
`openspec/changes/pi-github-actions-deployment/{proposal,design,tasks}.md`,
`deploy/github-runner/{README.md,github-vitals-runner.service,github-vitals-deploy.sudoers}`,
`crates/vitals-deploy/src/main.rs`

**Creates:** this proposal, design, capability delta, and task contract.

**Verify:** `openspec validate github-runner-activation-boundary --type change --strict --no-interactive && git diff --check`

## T-GRAB-2 — Add the narrowly allowlisted runner write paths

**Depends:** T-GRAB-1 · **Review required:** yes

**Reads:** `openspec/changes/github-runner-activation-boundary/{proposal,design,specs/github-runner-activation-boundary/spec}.md`,
`deploy/github-runner/github-vitals-runner.service`, `crates/vitals-deploy/src/main.rs`

**Modifies:** `deploy/github-runner/github-vitals-runner.service`

**Do:** Add only the five existing deployment target roots to `ReadWritePaths`. Do not weaken
`ProtectSystem`, add a Docker socket, widen sudoers, or include the spool, PostgreSQL, boot, or
credential paths.

**Verify:** `python3 - <<'PY'
from pathlib import Path
p = Path('deploy/github-runner/github-vitals-runner.service').read_text()
assert 'ProtectSystem=full' in p
assert 'ProtectHome=yes' in p
assert 'PrivateTmp=yes' in p
line = next(line for line in p.splitlines() if line.startswith('ReadWritePaths='))
expected = {
    '/usr/local/lib/vitals',
    '/usr/local/bin',
    '/usr/local/share/vitals-console/assets',
    '/etc/systemd/system',
    '/etc/udev/rules.d',
}
assert set(line.split('=', 1)[1].split()) == expected
for forbidden in ('/var/lib/vitals/spool', '/var/lib/postgresql', '/boot', '/var/run/docker.sock'):
    assert forbidden not in line, forbidden
PY
git diff --check`

## T-GRAB-3 — Install, verify, and exercise the boundary on `minidude`

**Depends:** T-GRAB-2 · **Review required:** yes

**Reads:** `deploy/github-runner/github-vitals-runner.service`,
`deploy/github-runner/github-vitals-deploy.sudoers`,
`.github/workflows/pi-deploy.yml`, and the T-GRAB-2 evidence.

**Do:** Install only the checked-in runner unit, reload/restart the runner, run the trusted
workflow activation against a fresh backup gate, retain its receipt, and restart
`vitals-spool-seal.timer` after the maintenance window. Do not claim success from a green build-only
job.

**Verify:** `ssh "$PI_HOST" 'set -e; systemctl is-active --quiet github-vitals-runner; sudo systemd-analyze verify /etc/systemd/system/github-vitals-runner.service; sudo jq -e ".activated == true and .preflight_ok == true and .health_ok == true and .spool_unchanged == true and .backup_gate == true" /var/lib/vitals/recovery/github-deploy-latest.json >/dev/null; systemctl is-active --quiet vitals-spool-seal.timer'`
