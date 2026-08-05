/*
 * gatt_service.c — Oura BLE GATT service registration (Zephyr v4+)
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

#include "gatt_service.h"

#include <string.h>
#include <errno.h>

#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>

#include <zephyr/bluetooth/bluetooth.h>
#include <zephyr/bluetooth/conn.h>
#include <zephyr/bluetooth/uuid.h>
#include <zephyr/bluetooth/gatt.h>

LOG_MODULE_REGISTER(gatt, LOG_LEVEL_DBG);

/* ── GATT UUID declarations ──────────────────────────────────────────── */

/* Primary service UUID */
#define OURA_SVC_UUID_VAL \
    BT_UUID_128_ENCODE(0x00001523, 0x1212, 0xefde, 0x1523, 0x785feabcd123)

/* Characteristic UUIDs */
#define OURA_AUTH_UUID_VAL \
    BT_UUID_128_ENCODE(0x00001524, 0x1212, 0xefde, 0x1523, 0x785feabcd123)
#define OURA_CMD_UUID_VAL \
    BT_UUID_128_ENCODE(0x00001525, 0x1212, 0xefde, 0x1523, 0x785feabcd123)
#define OURA_MEAS_UUID_VAL \
    BT_UUID_128_ENCODE(0x00001526, 0x1212, 0xefde, 0x1523, 0x785feabcd123)
#define OURA_DEVINFO_UUID_VAL \
    BT_UUID_128_ENCODE(0x00001527, 0x1212, 0xefde, 0x1523, 0x785feabcd123)
#define OURA_HIST_UUID_VAL \
    BT_UUID_128_ENCODE(0x00001528, 0x1212, 0xefde, 0x1523, 0x785feabcd123)

/* Static UUID declarations */
static const struct bt_uuid_128 ourasvc_uuid  = BT_UUID_INIT_128(OURA_SVC_UUID_VAL);
static const struct bt_uuid_128 auth_uuid     = BT_UUID_INIT_128(OURA_AUTH_UUID_VAL);
static const struct bt_uuid_128 cmd_uuid      = BT_UUID_INIT_128(OURA_CMD_UUID_VAL);
static const struct bt_uuid_128 meas_uuid     = BT_UUID_INIT_128(OURA_MEAS_UUID_VAL);
static const struct bt_uuid_128 devinfo_uuid  = BT_UUID_INIT_128(OURA_DEVINFO_UUID_VAL);
static const struct bt_uuid_128 hist_uuid     = BT_UUID_INIT_128(OURA_HIST_UUID_VAL);

/* ── User data structs ───────────────────────────────────────────────── */

/* Auth characteristic user data: tracks notification state */
struct oura_auth_ud {
    bool notify_enabled;
};

/* Measurement characteristic user data: tracks CCC state */
struct oura_meas_ud {
    bool notify_enabled;
};

/* History characteristic user data */
struct oura_hist_ud {
    bool notify_enabled;
};

/* Device info value */
struct oura_devinfo_value {
    uint8_t  battery_pct;
    uint8_t  flags;       /* bit 0 = charging */
    uint32_t fw_version;  /* e.g. 0x00000100 = 0.1.0 */
};

/* ── Static state ────────────────────────────────────────────────────── */

static struct oura_auth_ud    auth_ud;
static struct oura_meas_ud    meas_ud;
static struct oura_hist_ud    hist_ud;
static struct oura_devinfo_value devinfo_val = {
    .battery_pct = 100,
    .flags = 0,
    .fw_version = 0x00000100,  /* 0.1.0 */
};

static struct bt_conn *current_conn;
static bool is_authenticated;
static bool meas_notify_enabled;
static bool auth_notify_enabled;

/* Callbacks */
static oura_auth_write_cb_t              auth_write_cb;
static oura_command_write_cb_t           cmd_write_cb;
static oura_measurement_notify_enabled_cb_t meas_notify_cb;
static oura_history_notify_enabled_cb_t  hist_notify_cb;
static oura_connection_cb_t             conn_cb;

/* ── GATT read/write callbacks ───────────────────────────────────────── */

/* Auth characteristic: write-only (client → ring) */
static ssize_t on_auth_write(struct bt_conn *conn,
    const struct bt_gatt_attr *attr, const void *buf,
    uint16_t len, uint16_t offset, uint8_t flags)
{
    (void)conn;
    (void)attr;
    (void)offset;
    (void)flags;

    if (auth_write_cb) {
        auth_write_cb((const uint8_t *)buf, len);
    }
    return len;
}

/* Command characteristic: write-only */
static ssize_t on_cmd_write(struct bt_conn *conn,
    const struct bt_gatt_attr *attr, const void *buf,
    uint16_t len, uint16_t offset, uint8_t flags)
{
    (void)conn;
    (void)attr;
    (void)offset;
    (void)flags;

    if (cmd_write_cb) {
        cmd_write_cb((const uint8_t *)buf, len);
    }
    return len;
}

/* Device Info: read-only */
static ssize_t on_devinfo_read(struct bt_conn *conn,
    const struct bt_gatt_attr *attr, void *buf,
    uint16_t len, uint16_t offset)
{
    (void)conn;

    const struct oura_devinfo_value *val = attr->user_data;
    return bt_gatt_attr_read(conn, attr, buf, len, offset,
        val, sizeof(*val));
}

/* ── CCC changed callbacks ───────────────────────────────────────────── */

static void meas_ccc_cfg_changed(const struct bt_gatt_attr *attr, uint16_t value)
{
    (void)attr;
    bool enabled = (value == BT_GATT_CCC_NOTIFY);
    meas_ud.notify_enabled = enabled;
    meas_notify_enabled = enabled;
    LOG_INF("Measurement CCC: %s", enabled ? "notify ON" : "notify OFF");
    if (meas_notify_cb) {
        meas_notify_cb(enabled);
    }
}

static void auth_ccc_cfg_changed(const struct bt_gatt_attr *attr, uint16_t value)
{
    (void)attr;
    bool enabled = (value == BT_GATT_CCC_NOTIFY);
    auth_ud.notify_enabled = enabled;
    auth_notify_enabled = enabled;
    LOG_INF("Auth CCC: %s", enabled ? "notify ON" : "notify OFF");
}

static void hist_ccc_cfg_changed(const struct bt_gatt_attr *attr, uint16_t value)
{
    (void)attr;
    bool enabled = (value == BT_GATT_CCC_NOTIFY);
    hist_ud.notify_enabled = enabled;
    LOG_INF("History CCC: %s", enabled ? "notify ON" : "notify OFF");
    if (hist_notify_cb) {
        hist_notify_cb(enabled);
    }
}

/* ── GATT Service Definition ────────────────────────────────────────────
 *
 * Layout:
 *   Primary Service (0x1523)
 *   ├── Auth characteristic (0x1524) — Write, Notify
 *   │   └── CCC descriptor
 *   ├── Command characteristic (0x1525) — Write
 *   ├── Measurement characteristic (0x1526) — Notify
 *   │   └── CCC descriptor
 *   ├── Device Info characteristic (0x1527) — Read, Notify
 *   │   └── CCC descriptor
 *   └── History characteristic (0x1528) — Notify
 *       └── CCC descriptor
 */

BT_GATT_SERVICE_DEFINE(oura_svc,
    /* ── Primary Service ── */
    BT_GATT_PRIMARY_SERVICE(&ourasvc_uuid),

    /* ── Auth characteristic (write + notify) ── */
    BT_GATT_CHARACTERISTIC(&auth_uuid.uuid,
        BT_GATT_CHRC_WRITE | BT_GATT_CHRC_NOTIFY,
        BT_GATT_PERM_WRITE,
        NULL, on_auth_write, &auth_ud),
    BT_GATT_CCC(auth_ccc_cfg_changed, BT_GATT_PERM_READ | BT_GATT_PERM_WRITE),

    /* ── Command characteristic (write-only) ── */
    BT_GATT_CHARACTERISTIC(&cmd_uuid.uuid,
        BT_GATT_CHRC_WRITE,
        BT_GATT_PERM_WRITE,
        NULL, on_cmd_write, NULL),

    /* ── Measurement characteristic (notify-only) ── */
    BT_GATT_CHARACTERISTIC(&meas_uuid.uuid,
        BT_GATT_CHRC_NOTIFY,
        BT_GATT_PERM_NONE,
        NULL, NULL, &meas_ud),
    BT_GATT_CCC(meas_ccc_cfg_changed, BT_GATT_PERM_READ | BT_GATT_PERM_WRITE),

    /* ── Device Info characteristic (read + notify) ── */
    BT_GATT_CHARACTERISTIC(&devinfo_uuid.uuid,
        BT_GATT_CHRC_READ | BT_GATT_CHRC_NOTIFY,
        BT_GATT_PERM_READ,
        on_devinfo_read, NULL, &devinfo_val),
    BT_GATT_CCC(NULL, BT_GATT_PERM_READ | BT_GATT_PERM_WRITE),

    /* ── History characteristic (notify-only) ── */
    BT_GATT_CHARACTERISTIC(&hist_uuid.uuid,
        BT_GATT_CHRC_NOTIFY,
        BT_GATT_PERM_NONE,
        NULL, NULL, &hist_ud),
    BT_GATT_CCC(hist_ccc_cfg_changed, BT_GATT_PERM_READ | BT_GATT_PERM_WRITE),
);

/* ── Notification helpers ────────────────────────────────────────────── */

/* Find the attribute index for a given characteristic UUID */
static int find_attr_idx(const struct bt_uuid *uuid)
{
    for (int i = 0; i < ARRAY_SIZE(attr_oura_svc); i++) {
        const struct bt_gatt_attr *a = &attr_oura_svc[i];
        if (!bt_uuid_cmp(a->uuid, uuid)) {
            return i;
        }
    }
    return -1;
}

int oura_measurement_notify(const uint8_t *data, uint16_t len)
{
    if (!current_conn || !is_authenticated || !meas_notify_enabled) {
        return -ENOTCONN;
    }

    int idx = find_attr_idx(&meas_uuid.uuid);
    if (idx < 0) return -ENOENT;

    return bt_gatt_notify(current_conn, &attr_oura_svc[idx], data, len);
}

int oura_auth_notify(const uint8_t *data, uint16_t len)
{
    if (!current_conn || !auth_notify_enabled) {
        return -ENOTCONN;
    }

    int idx = find_attr_idx(&auth_uuid.uuid);
    if (idx < 0) return -ENOENT;

    return bt_gatt_notify(current_conn, &attr_oura_svc[idx], data, len);
}

int oura_history_notify(const uint8_t *data, uint16_t len)
{
    if (!current_conn || !is_authenticated || !hist_ud.notify_enabled) {
        return -ENOTCONN;
    }

    int idx = find_attr_idx(&hist_uuid.uuid);
    if (idx < 0) return -ENOENT;

    return bt_gatt_notify(current_conn, &attr_oura_svc[idx], data, len);
}

int oura_device_info_update(uint8_t battery_pct, bool charging, uint32_t fw_version)
{
    devinfo_val.battery_pct = battery_pct;
    devinfo_val.flags = charging ? 1 : 0;
    devinfo_val.fw_version = fw_version;

    /* Notify if connected */
    if (current_conn && is_authenticated) {
        int idx = find_attr_idx(&devinfo_uuid.uuid);
        if (idx >= 0) {
            bt_gatt_notify(current_conn, &attr_oura_svc[idx],
                &devinfo_val, sizeof(devinfo_val));
        }
    }
    return 0;
}

/* ── Connection tracking ─────────────────────────────────────────────── */

void oura_gatt_on_connected(struct bt_conn *conn)
{
    current_conn = bt_conn_ref(conn);
    is_authenticated = false;
    meas_notify_enabled = false;
    auth_notify_enabled = false;

    LOG_INF("BLE client connected");
    if (conn_cb) conn_cb(true);
}

void oura_gatt_on_disconnected(struct bt_conn *conn)
{
    if (current_conn) {
        bt_conn_unref(current_conn);
    }
    current_conn = NULL;
    is_authenticated = false;
    meas_notify_enabled = false;
    auth_notify_enabled = false;

    LOG_INF("BLE client disconnected");
    if (conn_cb) conn_cb(false);
}

/* ── Init ─────────────────────────────────────────────────────────────── */

int oura_gatt_init(
    oura_auth_write_cb_t              _auth_write_cb,
    oura_command_write_cb_t           _cmd_write_cb,
    oura_measurement_notify_enabled_cb_t _meas_notify_cb,
    oura_history_notify_enabled_cb_t     _hist_notify_cb,
    oura_connection_cb_t                _conn_cb)
{
    auth_write_cb  = _auth_write_cb;
    cmd_write_cb   = _cmd_write_cb;
    meas_notify_cb = _meas_notify_cb;
    hist_notify_cb = _hist_notify_cb;
    conn_cb        = _conn_cb;

    /* Force the service definition to be included */
    __ASSERT(ARRAY_SIZE(attr_oura_svc) > 0, "GATT service has no attributes");

    LOG_INF("GATT service initialized (%d attributes)", (int)ARRAY_SIZE(attr_oura_svc));
    return 0;
}

/* ── Accessors ────────────────────────────────────────────────────────── */

bool oura_gatt_is_connected_and_authenticated(void)
{
    return current_conn != NULL && is_authenticated;
}

void oura_gatt_mark_authenticated(void)
{
    is_authenticated = true;
    LOG_INF("Connection marked as authenticated");
}

struct bt_conn *oura_gatt_current_conn(void)
{
    return current_conn;
}
