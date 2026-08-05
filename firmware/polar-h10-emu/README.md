# Polar H10 BLE Emulator Dongle

Firmware for the **Raytac MDBT50Q-CX-40 nRF52840 USB-C dongle** that advertises as a Polar H10 and streams realistic synthetic RR interval data over BLE using the standard Bluetooth SIG Heart Rate profile. Pairs with the AnxietyWatch2 Linux daemon (and iOS app) to validate the full pipeline without owning a real chest strap.

## Hardware

| Item | Detail |
|------|--------|
| Target | Raytac MDBT50Q-CX-40 dongle (`raytac_mdbt50q_cx_40_dongle/nrf52840`) |
| SoC | nRF52840 — ARM Cortex-M4F @ 64 MHz, 1 MB Flash, 256 kB RAM |
| Connectivity | Bluetooth 5.4 LE (peripheral), USB 2.0 (power + CDC ACM serial) |
| Cost | ~$10–15 |

## Architecture

```
                    BLE                          USB CDC ACM
Pi ◄─────────────► nRF52840 Dongle ◄──────────► Host Computer
(vitalsd)         (advertises as "Polar H10")   (serial console)

GATT Service:     Standard Heart Rate           Commands:
                   Measurement (notify)           scenario <name>
                   0x2A37                         hr min <bpm> max <bpm>
                                                  jitter <ms>
                                                  status
                                                  help
```

## Building & Flashing

### Prerequisites

- [nRF Connect SDK](https://www.nordicsemi.com/Products/Development-software/nrf-connect-sdk) v2.4+
- `west` on PATH (source `zephyr-env.sh` from your SDK install)
- Python 3.8+ with `nrfutil` (`pip install nrfutil`)

### Build

```bash
# From the firmware/polar-h10-emu directory:
./scripts/flash.sh --build-only
```

### Flash

1. Unplug the dongle.
2. Hold the side button **toward the USB connector**, plug it in while holding the button, then release it. A slowly fading red LED and `/dev/cu.usbmodem*` indicate Open DFU mode.
3. Run:
   ```bash
   ./scripts/flash.sh
   ```
4. After `Device programmed`, unplug and replug normally **without** holding the button. It begins advertising as "Polar H10 XXXX".

This dongle uses Open DFU over USB serial; it does not expose a drag-and-drop mass-storage drive.

## Serial Console

The dongle presents a USB CDC ACM virtual serial port:

| OS | Device |
|----|--------|
| Linux | `/dev/ttyACM0` |
| macOS | `/dev/cu.usbmodem*` |

Connect at **115200 baud**:

```bash
screen /dev/ttyACM0 115200
# or
minicom -D /dev/ttyACM0
```

### Commands

```
scenario normal|sleep|exercise|afib         Switch data pattern
hr min <bpm> max <bpm>                      Set heart rate range
jitter <ms>                                 Set HRV jitter
status                                      Print config & uptime
help                                        List commands
```

## Scenarios

| Scenario | HR (BPM) | HRV Jitter (ms) | Use case |
|----------|----------|-----------------|----------|
| `normal` | 60–80 | 40 | Healthy adult baseline |
| `sleep` | 45–55 | 80 | Resting/High HRV |
| `exercise` | 120–160 | 10 | Low HRV under strain |
| `afib` | 80–140 | 250 | Massive erratic RR intervals |

## GATT Layout

Uses standard Bluetooth SIG UUIDs:

| Service / Characteristic | UUID |
|--------------------------|------|
| Primary Service (Heart Rate) | `0x180D` |
| HR Measurement (Notify) | `0x2A37` |
| Body Sensor Location (Read)| `0x2A38` |


## Project Structure

```
firmware/polar-h10-emu/
├── CMakeLists.txt
├── prj.conf                    # Zephyr config: BLE, USB CDC
├── src/
│   ├── main.c                  # Entry point, BLE init, timer loop
│   ├── gatt_service.c/h        # GATT service (SIG HRS)
│   ├── measurements.c/h        # RR-interval & HR generation
│   ├── usb_console.c/h         # CDC ACM command parser
│   └── scenarios.c/h           # Preset scenario definitions
├── scripts/
│   └── flash.sh                # Build + flash via nrfutil
└── README.md
```

## Testing

1. Flash the dongle, connect serial console.
2. Set scenario: `scenario normal`
3. Connect using standard Bluetooth Heart Rate testing tools (e.g. nRF Connect, LightBlue) or start `vitalsd`.
4. Observe the 0x180D (Heart Rate Service) stream of notifications on 0x2A37.

## License

Internal development tool — part of the AnxietyWatch2 project. DEBUG / TEST use only.
