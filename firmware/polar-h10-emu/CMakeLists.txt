# Polar H10 BLE Emulator Dongle
# nRF Connect SDK / Zephyr RTOS
cmake_minimum_required(VERSION 3.20.0)

find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})
project(polar_h10_emu)

# ── Sources ───────────────────────────────────────────────────────────
target_sources(app PRIVATE
  src/main.c
  src/gatt_service.c
  src/measurements.c
  src/usb_console.c
  src/scenarios.c
)
