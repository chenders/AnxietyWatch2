# Migrating to a Raspberry Pi: Architectural Feasibility Verdict

This document outlines the architectural decisions and feasibility research that led to the creation of **AnxietyWatch2** — transitioning from an iOS-centric application to a headless, Rust-based Raspberry Pi backend (`vitalsd`, `vitals-core`, PostgreSQL).

## TL;DR
- **The backend (Rust + PostgreSQL) and all cloud API integrations move to a Raspberry Pi 5 cleanly.** The multi-hour BLE oximeter/chest-strap ingestion is the one genuinely risky component.
- **Recommended target:** Raspberry Pi 5 (8GB or 16GB) booting from an NVMe SSD on the official M.2 HAT+, on the official 27W 5V/5A PSU, with a UPS/watchdog HAT, an external CSR8510A10 USB BLE dongle with external antenna, and the Pi kept on 5GHz WiFi or Ethernet. **microSD as the primary DB store is disqualified** for a write-heavy 24/7 database.
- **Alerting Philosophy:** Do not treat a single Pi as the sole trusted host for a safety-relevant physiological alert. Pi OS is not real-time, SD/power-loss failure modes are real, and BLE continuity over hours is the least reliable link in the chain. Keep the phone/watch as the authoritative alerting path where possible, and treat the Pi as an ingestion/analysis server.

## Key Findings

**Hardware.** The Pi 5 (BCM2712, quad Cortex-A76 @ 2.4GHz) is comfortably adequate for PostgreSQL + Rust binaries. RAM tiers matter more than CPU: 4GB is the practical minimum, 8GB is the sweet spot.

**Storage is the make-or-break decision.** A write-heavy Postgres workload on a consumer microSD card will wear it out and corrupt on power loss. The database must reside on an NVMe via the PCIe connector and official M.2 HAT+.

**BLE is the crux and the least certain area.** The Pi's onboard Cypress/Infineon combo chip shares a single antenna between WiFi and BLE, and coexistence-induced BLE drops are extensively documented. Mitigations are well understood: use an external USB dongle with its own antenna (CSR8510A10), 5GHz WiFi or Ethernet, and modern BlueZ. The Linux collectors use `tokio` and BlueZ, with `vitalsd` remaining the single append boundary for durable samples. Two simultaneous BLE central connections (oximeter + chest strap) is within BlueZ's capability but adds instability under load.

**Apple boundary is absolute.** HealthKit has no server-side API. The Pi cannot compile or sign iOS/watchOS builds. The ecosystem is thus split: data ingestion on the Pi, presentation/client on iOS.

## Details — Verdict Per Component

### 1. `vitalsd` Backend — ✅ Feasible
Pure Rust (`vitalsd`, `vitals-core`) on arm64 Linux. A Pi 5 handles a personal-scale sync API with enormous headroom.

### 2. PostgreSQL — ✅ Feasible, ⚠️ only with NVMe storage
Postgres has first-class arm64 packages. **The caveat is storage:**
- Filesystem/reliability tuning: ext4 with journaling, mount `/tmp` as tmpfs to reduce wear, tune `shared_buffers`/`work_mem`, enable WAL, and add a UPS/watchdog HAT for graceful shutdown.

### 3. BLE device integrations (BlueZ) — ⚠️ The hard part

**EMAY SleepO2 (overnight, multi-hour)**
Proven at the protocol level (UART-style RX/TX GATT). However, multi-hour continuous-session stability on Linux requires careful handling.
- The Wellue/EMAY auto-powers-off after ~2 minutes without a BLE connection.
- The deployed EMAY is unpaired/unbonded; after a Pi or BlueZ reboot, its configured address may not recreate the transient BlueZ device object. A bounded discovery interval may be required before the address-pinned collector can reconnect. The LED is not a substitute for checking BlueZ connection and fresh sample timestamps; see [`EMAY_OXIMETER.md`](../hardware/EMAY_OXIMETER.md) and the recovery record.

**Polar H10 (HR + RR/ECG streaming)**
Streams HR and RR via `0x180D`. Overnight multi-hour drops are real and partly hardware/environmental.

**BLE radio, coexistence, and the stack**
- **Strongly recommended: external USB BLE dongle with its own external antenna on a USB extension cable, onboard BT disabled, Pi on 5GHz WiFi or Ethernet.**
- BlueZ maturity: use dbus-broker and BlueZ ≥ 5.63. Known pain points: `bluetoothd` degrading over long runtimes, requiring service restarts.

### 4. CPAP / ez Share ingestion — ✅ Feasible
The ez Share WiFi microSD adapter creates its own WiFi access point. The Pi can use its onboard WiFi to connect to the SD card while using Ethernet for the main LAN, downloading `.edf` files via a polling service (`hms-cpap`).

### 5. Multi-sensor fusion / alerting — ⚠️ Feasible as compute, ❌ not as a sole safety-critical path
The computation (HRV analysis, baseline-deviation) is light and runs fine in `vitals-core`. The problem is **trustworthiness**:
- Raspberry Pi OS is **not a real-time OS**.
- Harden it with systemd `Restart=always` and watchdog (`WatchdogSec`), hardware UPS HAT, and dead-man's switch monitoring.
