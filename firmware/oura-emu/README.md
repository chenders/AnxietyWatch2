# Oura Ring BLE Emulator Dongle

Firmware for the **Raytac MDBT50Q-CX-40 nRF52840 USB-C dongle** that advertises as an Oura Ring and streams realistic synthetic physiological data over BLE. Pairs with the AnxietyWatch iOS app (DEBUG build) to validate the full `OuraBLEActor` → `SensorRouter` → display pipeline end-to-end — without owning a real Oura Ring.

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
iPhone ◄────────► nRF52840 Dongle ◄──────────► Host Computer
(DEBUG build)     (advertises as "Oura")       (serial console)

GATT Service:     Auth (AES-ECB nonce)         Commands:
                   Command (feature enable)      scenario <name>
                   Measurement (notify)           hr / spo2 / ibi / accel
                   Device Info (read)             temp / sleep / battery
                   History (notify)               key / status / help
```

## Building & Flashing

### Prerequisites

- [nRF Connect SDK](https://www.nordicsemi.com/Products/Development-software/nrf-connect-sdk) v2.4+
- `west` on PATH (source `zephyr-env.sh` from your SDK install)
- Python 3.8+ with `nrfutil` (`pip install nrfutil`)

### Build

```bash
# From the firmware/oura-emu directory:
./scripts/flash.sh --build-only
```

### Flash

1. Unplug the dongle.
2. Hold the side button **toward the USB connector**, plug it in while holding the button, then release it. A slowly fading red LED and `/dev/cu.usbmodem*` indicate Open DFU mode.
3. Run:
   ```bash
   ./scripts/flash.sh
   ```
4. After `Device programmed`, unplug and replug normally **without** holding the button. It begins advertising as "Oura".

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
scenario normal|desat|sleep|exercise|idle   Switch data pattern
hr min <bpm> max <bpm>                      Set heart rate range
spo2 min <%> max <%>                        Set SpO2 range
ibi mean <ms> jitter <ms>                   Set IBI distribution
accel on|off                                Toggle accelerometer (50 Hz)
temp deviation <°C>                         Set temperature deviation
sleep <0-3|auto>                            Force sleep stage
battery <pct>                               Set battery %
key <32-char-hex>                           Change shared auth key
status                                      Print config & uptime
help                                        List commands
```

## Scenarios

| Scenario | HR (BPM) | SpO2 (%) | Accel | Sleep | Use case |
|----------|----------|----------|-------|-------|----------|
| `normal` | 60–80 | 96–99 | Off | Awake | Healthy adult baseline |
| `desat` | 65–90 | 88–94 | Off | Auto | Moderate desaturation testing |
| `sleep` | 48–62 | 90–98 | 50 Hz | Cycling | Overnight sleep with movement |
| `exercise` | 110–155 | 96–99 | 50 Hz | Awake | Running/workout simulation |
| `idle` | — | — | Off | Awake | Ring-off-finger (no data) |

## GATT Layout

All UUIDs follow the `open_oura` / `ringverse` convention:

| Service / Characteristic | UUID |
|--------------------------|------|
| Primary Service | `00001523-1212-efde-1523-785feabcd123` |
| Auth (Write, Notify) | `00001524-1212-efde-1523-785feabcd123` |
| Command (Write) | `00001525-1212-efde-1523-785feabcd123` |
| Measurement (Notify) | `00001526-1212-efde-1523-785feabcd123` |
| Device Info (Read, Notify) | `00001527-1212-efde-1523-785feabcd123` |
| History (Notify) | `00001528-1212-efde-1523-785feabcd123` |

## Auth Flow

1. Client writes 16-byte nonce to Auth characteristic.
2. Dongle encrypts nonce with AES-ECB using the shared 16-byte key.
3. Dongle notifies encrypted response.
4. Client decrypts and writes back the original nonce.
5. Dongle verifies → auth complete, measurements begin streaming.

**Default key:** `00112233445566778899AABBCCDDEEFF` (matches `OuraBLEActorTests`).

Change with: `key <32-char-hex>` on the serial console.

## Packet Framing

Every packet is exactly 16 bytes:

```
[command:1][payload:14][checksum:1]
```

Checksum = XOR of bytes 0–14.

### Measurement Tags

| Tag | Name | Payload |
|-----|------|---------|
| `0x2F` | Live HR (IBI) | sub-tag `40`, IBI ms (uint16 LE), reserved |
| `0x07` | SpO2 | SpO2% (uint8), quality (nibble), reserved |
| `0x06` | Accelerometer | X, Y, Z (int16 LE, mg), reserved |
| `0x08` | Temperature | deviation °C × 100 (int16 LE), reserved |
| `0x09` | Sleep Stage | stage (uint8: 0=awake, 1=light, 2=deep, 3=REM) |
| `0x0A` | Battery | % (uint8), flags (charging=bit0), reserved |

## LED Indicators

The green LED on the dongle shows connection state:

| LED Pattern | State |
|-------------|-------|
| Solid on | Advertising (no connection) |
| Fast blink (~5 Hz) | Connected, not authenticated |
| Slow blink (~0.5 Hz) | Authenticated + streaming |
| Off briefly then on | Disconnected — re-advertising |

## Project Structure

```
firmware/oura-emu/
├── CMakeLists.txt
├── prj.conf                    # Zephyr config: BLE, USB CDC, mbedTLS
├── boards/
│   └── nrf52840dongle.overlay            # legacy PCA10059-only overlay (not used by Raytac build)
├── src/
│   ├── main.c                  # Entry point, BLE init, timer loop, LED control
│   ├── gatt_service.c/h        # GATT service + characteristic registration
│   ├── auth.c/h                # AES-ECB nonce challenge (mbedTLS)
│   ├── measurements.c/h        # Synthetic data generators (random walk)
│   ├── packet.c/h              # 16-byte packet framing + XOR checksum
│   ├── usb_console.c/h         # CDC ACM command parser
│   └── scenarios.c/h           # Preset scenario definitions
├── scripts/
│   └── flash.sh                # Build + flash via nrfutil
└── README.md
```

## Testing with iOS

1. Flash the dongle, connect serial console.
2. Set scenario: `scenario normal`
3. On iPhone: build AnxietyWatch in DEBUG with `-ouraBLEDemo` launch argument.
4. The app discovers "Oura", connects, authenticates, and streams live data.
5. Use `OuraBLEDemoView` to observe IBI, SpO2, accel, temp, sleep stage, battery.

**No real user data is written to SwiftData, HealthKit, or the server.**

## Troubleshooting

| Problem | Fix |
|---------|-----|
| Dongle not found by iPhone | Double-tap RESET, verify green LED is solid. Toggle Bluetooth off/on on iPhone. |
| Auth fails | Ensure key matches on both sides. Default key: `00112233445566778899AABBCCDDEEFF`. |
| No measurements after auth | Send `SetFeatureMode` with `CAP_DAYTIME_HR \| CAP_CONNECTED_LIVE` (0x81). |
| Serial console garbled | Set baud to 115200, 8N1. |
| USB CDC ACM not appearing | Wait 2–3 seconds after plugging in. Check `dmesg \| grep ttyACM` or `ls /dev/cu.usbmodem*`. |

## License

Internal development tool — part of the AnxietyWatch project. DEBUG / TEST use only.
