/*
 * gatt_service.h — Oura BLE GATT service & characteristic registration
 *
 * Registers the primary Oura GATT service with characteristics matching
 * the real Oura Ring 3/4/5 layout:
 *   - Auth (write + notify)        — AES-ECB nonce challenge
 *   - Command (write)              — feature enable, request latest, start history
 *   - Measurement (notify)         — live IBI / SpO2 / accel / temp / sleep / battery
 *   - Device Info (read + notify)  — battery %, firmware version
 *   - History (notify)             — event dump stream
 *
 * UUIDs follow the open_oura / ringverse convention:
 *   Service:        00001523-1212-efde-1523-785feabcd123
 *   Auth:           00001524-1212-efde-1523-785feabcd123
 *   Command:        00001525-1212-efde-1523-785feabcd123
 *   Measurement:    00001526-1212-efde-1523-785feabcd123
 *   Device Info:    00001527-1212-efde-1523-785feabcd123
 *   History:        00001528-1212-efde-1523-785feabcd123
 */

#ifndef GATT_SERVICE_H
#define GATT_SERVICE_H

#include <zephyr/bluetooth/bluetooth.h>
#include <zephyr/bluetooth/gatt.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── GATT UUIDs ──────────────────────────────────────────────────────── */

/* Primary service: 00001523-1212-efde-1523-785feabcd123 */
#define OURA_SERVICE_UUID \
    BT_UUID_128_ENCODE(0x00001523, 0x1212, 0xefde, 0x1523, 0x785feabcd123)

/* Auth characteristic: 00001524-… */
#define OURA_AUTH_CHRC_UUID \
    BT_UUID_128_ENCODE(0x00001524, 0x1212, 0xefde, 0x1523, 0x785feabcd123)

/* Command characteristic: 00001525-… */
#define OURA_COMMAND_CHRC_UUID \
    BT_UUID_128_ENCODE(0x00001525, 0x1212, 0xefde, 0x1523, 0x785feabcd123)

/* Measurement characteristic: 00001526-… */
#define OURA_MEASUREMENT_CHRC_UUID \
    BT_UUID_128_ENCODE(0x00001526, 0x1212, 0xefde, 0x1523, 0x785feabcd123)

/* Device Info characteristic: 00001527-… */
#define OURA_DEVICE_INFO_CHRC_UUID \
    BT_UUID_128_ENCODE(0x00001527, 0x1212, 0xefde, 0x1523, 0x785feabcd123)

/* History characteristic: 00001528-… */
#define OURA_HISTORY_CHRC_UUID \
    BT_UUID_128_ENCODE(0x00001528, 0x1212, 0xefde, 0x1523, 0x785feabcd123)

/* ── Callback typedefs ────────────────────────────────────────────────── */

/** Called when the client writes a 16-byte nonce to the auth characteristic. */
typedef void (*oura_auth_write_cb_t)(const uint8_t *data, uint16_t len);

/** Called when the client writes a command packet to the command characteristic. */
typedef void (*oura_command_write_cb_t)(const uint8_t *data, uint16_t len);

/** Called when the client enables notifications on the measurement characteristic. */
typedef void (*oura_measurement_notify_enabled_cb_t)(bool enabled);

/** Called when the client enables notifications on the history characteristic. */
typedef void (*oura_history_notify_enabled_cb_t)(bool enabled);

/** Called when a BLE client connects or disconnects. */
typedef void (*oura_connection_cb_t)(bool connected);

/* ── API ──────────────────────────────────────────────────────────────── */

/**
 * Initialize the GATT service and register all characteristics.
 *
 * Must be called once before bt_le_adv_start().  If any callback is NULL
 * that handler is simply not invoked.
 */
int oura_gatt_init(
    oura_auth_write_cb_t              auth_write_cb,
    oura_command_write_cb_t           command_write_cb,
    oura_measurement_notify_enabled_cb_t meas_notify_cb,
    oura_history_notify_enabled_cb_t     hist_notify_cb,
    oura_connection_cb_t                conn_cb
);

/**
 * Send a notification on the measurement characteristic.
 *
 * @param data  Pointer to 16-byte packet.
 * @param len   Must be 16.
 * @return 0 on success, negative errno otherwise.
 */
int oura_measurement_notify(const uint8_t *data, uint16_t len);

/**
 * Send a notification on the auth characteristic (challenge response).
 *
 * @param data  Pointer to 16-byte encrypted nonce.
 * @param len   Must be 16.
 * @return 0 on success, negative errno otherwise.
 */
int oura_auth_notify(const uint8_t *data, uint16_t len);

/**
 * Send a notification on the history characteristic.
 *
 * @param data  Pointer to packet buffer.
 * @param len   Packet length (≤16).
 * @return 0 on success, negative errno otherwise.
 */
int oura_history_notify(const uint8_t *data, uint16_t len);

/**
 * Update the Device Info characteristic value (battery, firmware version).
 *
 * The value is a 6-byte structure:
 *   [0]  Battery percent (0–100)
 *   [1]  Flags: bit 0 = charging
 *   [2:5] Firmware version as uint32 LE (e.g. 0x00010000 = 1.0.0)
 */
int oura_device_info_update(uint8_t battery_pct, bool charging, uint32_t fw_version);

/**
 * Call from main when a BLE connection is established or dropped.
 * Controls advertisement restart and LED indication.
 */
void oura_gatt_on_connected(struct bt_conn *conn);
void oura_gatt_on_disconnected(struct bt_conn *conn);

/**
 * Returns true if at least one BLE client is connected and authenticated.
 */
bool oura_gatt_is_connected_and_authenticated(void);

/**
 * Mark the current connection as authenticated (called after successful
 * nonce challenge).
 */
void oura_gatt_mark_authenticated(void);

/**
 * Returns the current connection reference (or NULL).
 */
struct bt_conn *oura_gatt_current_conn(void);

#ifdef __cplusplus
}
#endif

#endif /* GATT_SERVICE_H */
