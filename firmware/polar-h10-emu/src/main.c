#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>
#include <zephyr/bluetooth/bluetooth.h>
#include <zephyr/bluetooth/uuid.h>
#include <zephyr/bluetooth/conn.h>

#include "gatt_service.h"
#include "measurements.h"
#include "usb_console.h"

LOG_MODULE_REGISTER(main, LOG_LEVEL_DBG);

static const struct bt_data ad[] = {
    BT_DATA_BYTES(BT_DATA_FLAGS, (BT_LE_AD_GENERAL | BT_LE_AD_NO_BREDR)),
    BT_DATA_BYTES(BT_DATA_UUID16_ALL, 0x0D, 0x18) /* 0x180D (Heart Rate Service) in little-endian */
};

static const struct bt_data sd[] = {
    BT_DATA(BT_DATA_NAME_COMPLETE, "Polar H10 XXXX", 14),
};

static void connected(struct bt_conn *conn, uint8_t err) {
    if (err) {
        LOG_ERR("Connection failed (err 0x%02x)", err);
    } else {
        LOG_INF("Connected");
        polar_console_print("[BLE] Connected");
    }
}

static void adv_start(void)
{
    int err = bt_le_adv_start(BT_LE_ADV_CONN_FAST_2, ad, ARRAY_SIZE(ad), sd, ARRAY_SIZE(sd));
    if (err && err != -EALREADY) {
        LOG_ERR("advertising start failed (%d)", err);
    } else {
        LOG_INF("advertising as \"%s\"", CONFIG_BT_DEVICE_NAME);
    }
}

static void adv_work_handler(struct k_work *work)
{
    ARG_UNUSED(work);
    adv_start();
}
static K_WORK_DEFINE(adv_work, adv_work_handler);

static void disconnected(struct bt_conn *conn, uint8_t reason) {
    LOG_INF("Disconnected (reason 0x%02x)", reason);
    polar_console_print("[BLE] Disconnected");

    k_work_submit(&adv_work);
}

BT_CONN_CB_DEFINE(conn_callbacks) = {
    .connected = connected,
    .disconnected = disconnected,
};

int main(void) {
    int err;

    k_sleep(K_SECONDS(2));
    polar_console_init();

    LOG_INF("Starting Polar H10 Emulator");

    err = bt_enable(NULL);
    if (err) {
        LOG_ERR("Bluetooth init failed (err %d)", err);
        return 0;
    }
    LOG_INF("Bluetooth initialized");

    measurements_init();
    gatt_service_init();

    adv_start();
    polar_console_print("[BLE] Advertising started");

    measurements_start();

    return 0;
}
