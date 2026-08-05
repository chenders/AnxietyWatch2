# How to Build and Flash nRF52840 Firmware (Zephyr / nRF Connect SDK)

This documents the setup and workflow used to build firmware for the nRF52840 USB Dongle,
as done for both the EMAY SleepO2 emulator and the Oura Ring BLE emulator.

## Prerequisites

Install the prerequisites and set `ZEPHYRPROJECT` to the nRF Connect SDK workspace:

| Component | Path |
|---|---|
| nRF Connect SDK (Zephyr) | `$ZEPHYRPROJECT/` |
| Zephyr SDK (ARM GCC) | Set by the nRF Connect SDK toolchain |
| Python venv (west) | `$ZEPHYRPROJECT/.venv312/` |
| nrfutil (DFU flashing) | Available on `PATH` |

## Build workflow

```bash
# 1. Activate the Zephyr environment
cd /path/to/AnxietyWatch2/firmware/polar-h10-emu
source "$ZEPHYRPROJECT/.venv312/bin/activate"
source "$ZEPHYRPROJECT/zephyr/zephyr-env.sh"

# 2. Build
west build --board raytac_mdbt50q_cx_40_dongle/nrf52840 --build-dir build-raytac .

# 3. Output
# Firmware:  build-raytac/zephyr/zephyr.hex
# ELF:       build-raytac/zephyr/zephyr.elf
```

## Flash to dongle

This hardware is a **Raytac MDBT50Q-CX-40 dongle**, not Nordic's PCA10059
`nrf52840dongle`. Use the exact Raytac board target above. Building for the
PCA10059 can package and flash successfully while producing an application that
does not boot correctly on this dongle.

### nrfutil Open DFU
```bash
# Package hex into DFU zip
nrfutil pkg generate \
    --hw-version 52 \
    --sd-req 0x00 \
    --application build-raytac/zephyr/zephyr.hex \
    --application-version 1 \
    build-raytac/dfu-package.zip

# Flash via USB serial (dongle must be in DFU mode)
nrfutil dfu usb-serial \
    --package build-raytac/dfu-package.zip \
    --port /dev/cu.usbmodemXXXX \
    --timeout 30
```

## Important: Zephyr v4 notes

The Zephyr installation is **v4.4.99** (pre-release/development). Key differences from v3:

- **GATT macros**: `BT_GATT_CCC(_changed, _perm)` not `BT_GATT_CCC_MANAGED(((struct bt_gatt_ccc_cfg[]){...}), ...)`
- **BT_GATT_CCC_CFG_INIT**: Does not exist in v4. Use `BT_GATT_CCC(changed_fn, perm)`.
- **Advertising**: `BT_LE_ADV_CONN_FAST_1` not `BT_LE_ADV_CONN_NAME`
- **PSA Crypto**: `CONFIG_PSA_CRYPTO=y` enables mbedTLS automatically via `PSA_CRYPTO_PROVIDER_MBEDTLS`
- **USB**: Board defconfig uses `device_next` CDC ACM; do NOT set `CONFIG_USB_DEVICE_STACK=y` or `CONFIG_USB_CDC_ACM=y`
- **uart_rx_enable**: Takes 4 args `(dev, buf, len, timeout)` not 3

## Notes

- Enter DFU by unplugging the dongle, holding its side button toward the USB connector, and plugging it in while holding the button. A slowly fading red LED means Open DFU is active.
- After `Device programmed`, unplug and replug normally without holding the button.
- If the build cache gets corrupted, delete `build-raytac/` and rebuild.
- Never set `CONFIG_USB_DEVICE_STACK=y` — the board defconfig handles USB
- The firmware UUIDs must match the Swift `OuraBLEProtocol.swift` UUIDs
- Default auth key: `00112233445566778899AABBCCDDEEFF` (same as `OuraBLEActorTests`)
