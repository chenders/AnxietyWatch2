#include "gatt_service.h"

#include <zephyr/bluetooth/bluetooth.h>
#include <zephyr/bluetooth/uuid.h>
#include <zephyr/bluetooth/gatt.h>
#include <zephyr/logging/log.h>

LOG_MODULE_REGISTER(gatt, LOG_LEVEL_DBG);

static bool notify_enabled = false;

static void hrm_ccc_cfg_changed(const struct bt_gatt_attr *attr, uint16_t value) {
    notify_enabled = (value == BT_GATT_CCC_NOTIFY);
    LOG_INF("Heart Rate Measurement Notifications: %s", notify_enabled ? "Enabled" : "Disabled");
}

BT_GATT_SERVICE_DEFINE(hrs_svc,
    BT_GATT_PRIMARY_SERVICE(BT_UUID_HRS),
    BT_GATT_CHARACTERISTIC(BT_UUID_HRS_MEASUREMENT,
                           BT_GATT_CHRC_NOTIFY,
                           BT_GATT_PERM_NONE,
                           NULL, NULL, NULL),
    BT_GATT_CCC(hrm_ccc_cfg_changed,
                BT_GATT_PERM_READ | BT_GATT_PERM_WRITE),
    // Optional: Body Sensor Location (0 = Other, 1 = Chest, 2 = Wrist, etc.)
    BT_GATT_CHARACTERISTIC(BT_UUID_HRS_BODY_SENSOR,
                           BT_GATT_CHRC_READ,
                           BT_GATT_PERM_READ,
                           NULL, NULL, NULL) // Hardcoding to Chest (1) could be done here if needed
);

void gatt_service_init(void) {
    LOG_INF("Polar H10 GATT service registered");
}

bool gatt_is_subscribed(void) {
    return notify_enabled;
}

void gatt_notify_hrm(const uint8_t *data, uint16_t len) {
    if (!notify_enabled) {
        return;
    }
    bt_gatt_notify(NULL, &hrs_svc.attrs[1], data, len);
}
