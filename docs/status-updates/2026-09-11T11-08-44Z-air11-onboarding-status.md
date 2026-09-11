# AirSense onboarding status — 2026-09-11 11:08:44Z

## Decision

The owner approved the AirSense onboarding amendment in the conversation on
2026-09-11, and the repository proposal is now marked `Accepted`. This does not
authorize collector activation or live acceptance.

## Current position

- T-A11-ONBOARD-1 host implementation is complete and its Verify command has
  exited 0.
- No physical scan, pairing attempt, service activation, or credential mutation
  has occurred.
- A bounded remote-scan attempt did not reach `bluetoothctl`: the SSH heredoc
  was interpreted as an interactive Python session and timed out. It was
  stopped without starting a scanner.
- The existing production collector and its read-only RPC boundary remain
  unchanged.
- `crates/vitals-collectors/src/as11_config.rs` now validates reusable
  application credentials without requiring a Linux mapping, writes new pairing
  credentials without replacing an existing path, and updates mappings through
  private atomic replacement; synthetic tests pass.
- The pre-existing unrelated `.idea/modules.xml` worktree change remains
  untouched.

## Scope correction

Implementation will stay narrow: one separate, finite onboarding command that
reuses the reviewed SRP implementation and exact FD56 GATT path. It will not
redesign the admin console, widen the production RPC allowlist, alter systemd
activation, or change H10/EMAY/EzShare behavior.

The multiple-candidate wording now requires an exact operator-selected
address, and the user guide distinguishes application pairing from the
steady-state collector's pre-existing BlueZ-bond requirement. The remaining
steps are a Linux/Pi operator-authorized discovery or first-pair run, followed
by the separate bounded AirSense acceptance command.
