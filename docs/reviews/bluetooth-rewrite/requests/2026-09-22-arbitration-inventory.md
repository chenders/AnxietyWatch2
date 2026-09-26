# Librarian request: BlueZ arbitration inventory

`LIBRARIAN.md` is absent. Please inventory the following with exact repository-relative file and line references, marking each unknown rather than inferring it:

1. Every Linux Bluetooth entry point in `AnxietyWatch2`, `aw2-air11`, and `aw2-wellue-fix` (BlueZ D-Bus, `bluetoothctl`, `btmgmt`, raw sockets, kernel/autoconnect configuration, and subprocesses).
2. All BlueZ object/API usage: adapter discovery/start/stop, device connect/disconnect, pair/trust, GATT/ATT, agent, and property subscriptions.
3. Adapter/controller identity assumptions, including MAC/address, hci index, Sena/CSR/USB references, udev rules, systemd units, and deployment scripts.
4. Service ownership and startup/order dependencies for BP, Wellue, H10, AirSense, and SleepO2; identify every process that can issue discovery or connect.
5. Evidence for BP kernel autoconnect and the exact boundary that prohibits `Device1.Connect` in the BP collector.
6. Evidence for the reported Wellue discovery contention and the observed BP recovery after Wellue stopped, distinguishing code evidence from live/runtime evidence.
7. Existing tests and missing test seams for adapter arbitration, discovery leases, connect retries, and failure isolation.

Scope: inventory only; do not change Rust, deployment, service, udev, firmware, or runtime configuration. The arbitration architecture report will proceed with clearly marked unknowns until this is answered.
