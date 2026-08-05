# Oura Ring BLE Emulator Dongle — nRF52840 USB Dongle (PCA10059)
# nRF Connect SDK / Zephyr RTOS v4
cmake_minimum_required(VERSION 3.20.0)

find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})
project(oura_emu)

# ── Sources ───────────────────────────────────────────────────────────
target_sources(app PRIVATE
  src/main.c
  src/gatt_service.c
  src/auth.c
  src/measurements.c
  src/usb_console.c
  src/scenarios.c
)

# PSA Crypto is enabled via Kconfig (CONFIG_PSA_CRYPTO=y).
# Zephyr links it automatically when those symbols are set.
