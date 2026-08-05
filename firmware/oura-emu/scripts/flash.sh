#!/usr/bin/env bash
# flash.sh — Build and flash the Oura Ring BLE Emulator to the Raytac nRF52840 dongle
#
# Prerequisites:
#   - nRF Connect SDK installed and `west` on PATH
#   - Raytac MDBT50Q-CX-40 dongle in Open DFU mode (hold side button while plugging in)
#
# Usage:
#   ./scripts/flash.sh              # Build + flash (DFU over USB)
#   ./scripts/flash.sh --build-only # Build only, don't flash
#   ./scripts/flash.sh --clean      # Clean build
#
# Enter DFU by unplugging the dongle, holding its side button toward the USB
# connector, and plugging it in while still holding the button. A slowly fading
# red LED and /dev/cu.usbmodem* indicate Open DFU mode.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="${PROJECT_DIR}/build"
# This hardware is the Raytac MDBT50Q-CX-40, not Nordic's PCA10059 dongle.
# The distinction matters: the Raytac board definition has the correct pinout,
# regulator, reset/UICR, flash partition, LEDs, and USB configuration.
BOARD="raytac_mdbt50q_cx_40_dongle/nrf52840"

MODE="full"

# ── Parse args ──────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --build-only) MODE="build" ;;
        --clean)      MODE="clean" ;;
        *)            echo "Unknown flag: $arg"; exit 1 ;;
    esac
done

# ── Check SDK ───────────────────────────────────────────────────────────
if ! command -v west &>/dev/null; then
    echo "ERROR: 'west' not found. Source nRF Connect SDK environment first."
    echo "  e.g.: source /opt/nordic/ncs/v2.x.x/zephyr/zephyr-env.sh"
    exit 1
fi

# ── Clean ───────────────────────────────────────────────────────────────
if [ "$MODE" = "clean" ]; then
    echo "==> Cleaning build directory..."
    rm -rf "$BUILD_DIR"
    MODE="full"
fi

# ── Build ───────────────────────────────────────────────────────────────
echo "==> Building for ${BOARD}..."
mkdir -p "$BUILD_DIR"

west build \
    --board "$BOARD" \
    --build-dir "$BUILD_DIR" \
    "$PROJECT_DIR" \
    -- \
    -DCONFIG_DEBUG=y \
    -DCONFIG_ASSERT=y

echo "==> Build complete."
echo "    zephyr.hex: ${BUILD_DIR}/zephyr/zephyr.hex"

if [ "$MODE" = "build" ]; then
    exit 0
fi

# ── Flash via DFU (nrfutil) ────────────────────────────────────────────
echo "==> Flashing via USB DFU..."

# Package hex into DFU zip
HEX_FILE="${BUILD_DIR}/zephyr/zephyr.hex"
DFU_PKG="${BUILD_DIR}/zephyr/dfu-package.zip"

if ! command -v nrfutil &>/dev/null; then
    echo "ERROR: 'nrfutil' not found. Install it:"
    echo "  pip install nrfutil"
    echo ""
    echo "Install Nordic nrfutil with the nrf5sdk-tools command and retry."
    echo "This dongle uses Open DFU over USB serial; it is not a mass-storage drive."
    exit 1
fi

nrfutil pkg generate \
    --hw-version 52 \
    --sd-req 0x00 \
    --application "$HEX_FILE" \
    --application-version 1 \
    "$DFU_PKG"

# nrfutil will try to find the dongle in DFU mode.
echo ""
echo "==> Looking for the Open DFU serial port..."
shopt -s nullglob
ports=(/dev/cu.usbmodem* /dev/tty.usbmodem* /dev/ttyACM*)
if [ ${#ports[@]} -eq 0 ]; then
    echo "ERROR: no Open DFU serial port found."
    echo "Unplug the dongle, hold its side button toward the USB connector,"
    echo "plug it in while holding the button, then rerun this script."
    exit 1
fi
port="${ports[0]}"
echo "    Using $port"

nrfutil dfu usb-serial \
    --package "$DFU_PKG" \
    --port "$port" \
    --timeout 30

echo ""
echo "==> Programmed successfully. Unplug and replug normally (without holding"
echo "    the button); the dongle will advertise as 'Oura'. Its application USB"
echo "    console should then appear as /dev/cu.usbmodem*."
