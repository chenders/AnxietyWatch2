# Security policy

AnxietyWatch2 is an experimental, single-household prototype. It runs on a Raspberry Pi on a
private LAN and is **not a medical device** (see the warning in [README.md](README.md)). There is
no hosted service, no release channel and no other deployment to patch.

## Reporting a vulnerability

Please use GitHub's **private vulnerability reporting** for this repository (*Security* tab →
*Report a vulnerability*). Do not open a public issue or pull request for a vulnerability.

Reports are read by the single maintainer. There is no response-time commitment, and no bounty.

## In scope

- Decoders that parse bytes from Bluetooth devices (`crates/vitals-decode`) — this input is
  attacker-controlled by design and is fuzzed.
- The daemon, spool and storage path, the HTTP API and admin console, and the deployment scripts.
- The CI workflows, in particular anything that could let a pull request reach the self-hosted
  runner. Workflow runs from external pull requests either cannot reach a self-hosted runner or
  need maintainer approval before they run, and `scripts/check-workflow-runners.py` additionally
  keeps default-branch workflows that a fork can trigger off it.

## Known, accepted gaps

The prototype deliberately defers some hardening while it is LAN-only. Those gaps are tracked in
the open, with reasons, in
[`openspec/changes/prototype-hardening-backlog`](openspec/changes/prototype-hardening-backlog/) —
reporting one of them again is welcome only if it shows the gap is reachable in a way that change
does not describe.
