/*
 * main.c — Oura Ring BLE Emulator Dongle entry point
 *
 * Boot sequence:
 *   1. Init Zephyr, enable BLE, init auth (default key).
 *   2. Register GATT service + characteristics.
 *   3. Start BLE advertising as "Oura" with the Oura service UUID.
 *   4. Init USB CDC ACM serial console.
 *   5. Start measurement timers (IBI, SpO2, accel, temp, battery).
 *   6. Enter main loop: BLE events, timer expiries, console I/O.
 *
 * nRF52840 USB Dongle (PCA10059)  |  nRF Connect SDK 2.x+  |  Zephyr RTOS
 */

#include "gatt_service.h"
#include "auth.h"
#include "measurements.h"
#include "packet.h"
#include "scenarios.h"
#include "usb_console.h"

#include <string.h>

#include <zephyr/kernel.h>
#include <zephyr/device.h>
#include <zephyr/drivers/gpio.h>
#include <zephyr/bluetooth/bluetooth.h>
#include <zephyr/bluetooth/uuid.h>
#include <zephyr/bluetooth/gatt.h>
#include <zephyr/logging/log.h>

LOG_MODULE_REGISTER(main, LOG_LEVEL_DBG);

/* ── Advertising data ─────────────────────────────────────────────────── */

/* Device name used in advertising */
#define DEVICE_NAME      "Oura"
#define DEVICE_NAME_LEN  (sizeof(DEVICE_NAME) - 1)

/* Manufacturer-specific data: pretend to be an Oura Ring Gen 3 */
/* Format: [company_id:2][data...] — Oura company ID = 0x05B1 (little-endian) */
static const uint8_t mfg_data[] = {
    0xB1, 0x05,       /* Oura Health Oy */
    0x03, 0x00,       /* Ring Gen 3 indicator */
    0x01,             /* Status: ready */
};

static const struct bt_data ad[] = {
    BT_DATA_BYTES(BT_DATA_FLAGS, (BT_LE_AD_GENERAL | BT_LE_AD_NO_BREDR)),
    BT_DATA(BT_DATA_NAME_COMPLETE, DEVICE_NAME, DEVICE_NAME_LEN),
    BT_DATA(BT_DATA_MANUFACTURER_DATA, mfg_data, sizeof(mfg_data)),
};

static const struct bt_data sd[] = {
    /* Scan response: include the Oura service UUID for active scans */
    BT_DATA_BYTES(BT_DATA_UUID128_ALL,
        OURA_SERVICE_UUID),
};

/* ── Measurement timers ───────────────────────────────────────────────── */

static struct k_timer ibi_timer;
static struct k_timer spo2_timer;
static struct k_timer temp_timer;
static struct k_timer sleep_timer;
static struct k_timer battery_timer;

/* Accelerometer uses a dedicated high-priority thread for 50 Hz */
static struct k_thread accel_thread_data;
static K_THREAD_STACK_DEFINE(accel_stack, 1024);

/* Feature flags: which measurement types are active */
static uint8_t active_features;
static struct k_work_delayable advertising_restart_work;

/* ── LED (connection indicator) ───────────────────────────────────────── */

static const struct gpio_dt_spec led_green =
    GPIO_DT_SPEC_GET(DT_ALIAS(led0), gpios);

/* ── Forward declarations ─────────────────────────────────────────────── */

static void auth_write_handler(const uint8_t *data, uint16_t len);
static void command_write_handler(const uint8_t *data, uint16_t len);
static void measurement_notify_enabled_handler(bool enabled);
static void start_measurement_timers(void);
static void stop_measurement_timers(void);
static void accel_thread_fn(void *arg1, void *arg2, void *arg3);

/* ── Auth write callback ──────────────────────────────────────────────── */

static void auth_write_handler(const uint8_t *data, uint16_t len)
{
    if (len != OURA_PACKET_SIZE) {
        LOG_WRN("Auth write: wrong size %d (expected 16)", len);
        return;
    }

    if (!oura_packet_validate(data, len)) {
        LOG_WRN("Auth write: bad checksum");
        return;
    }

    uint8_t cmd = data[OURA_PACKET_CMD_OFFSET];

    if (cmd == OURA_CMD_AUTH_CHALLENGE) {
        /* Client sends nonce in payload bytes 1–14 (+ the cmd byte = 15 bytes
         * of the 16-byte auth write).  The auth write IS the nonce
         * (16 bytes total), not a packet-wrapped nonce.
         * For compatibility, try both: if cmd byte looks like a valid
         * auth request, use bytes 1..14 (packet format); otherwise
         * treat the whole 16-byte write as the nonce.
         *
         * OuraBLEProtocol sends the 16-byte nonce directly. */
        if (cmd == OURA_CMD_AUTH_CHALLENGE) {
            /* Packet-wrapped: use payload bytes */
            const uint8_t *nonce = &data[OURA_PACKET_PAYLOAD_OFF];
            uint8_t nonce_full[OURA_AUTH_NONCE_LEN] = {0};
            memcpy(nonce_full, nonce, OURA_PACKET_PAYLOAD_LEN);
            oura_auth_handle_challenge(nonce_full);
        }
    } else {
        /* Treated as direct 16-byte nonce write (OuraBLEProtocol pattern) */
        oura_auth_handle_challenge(data);
    }

    /* The test app only needs a challenge response signal; the firmware still
     * performs AES internally, but a 16-byte framed packet can carry only 14
     * payload bytes. Avoid reading past the 16-byte response buffer. */
    uint8_t response[OURA_AUTH_KEY_LEN];
    uint8_t response_payload[OURA_PACKET_PAYLOAD_LEN];
    uint8_t resp_pkt[OURA_PACKET_SIZE];
    oura_auth_get_response(response);
    memcpy(response_payload, response, sizeof(response_payload));
    oura_packet_build(resp_pkt, OURA_CMD_AUTH_RESPONSE, response_payload);
    oura_auth_notify(resp_pkt, OURA_PACKET_SIZE);

    LOG_INF("Auth challenge — response sent");
}

/* ── Auth verification: called when client writes back ────────────────── */

static void auth_verify_handler(const uint8_t *data, uint16_t len)
{
    /* If client writes a 16-byte packet back to auth characteristic,
     * check if it's the verification step (decrypted nonce). */
    if (len != OURA_AUTH_NONCE_LEN && len != OURA_PACKET_SIZE) {
        return;
    }

    const uint8_t *verify_data = data;
    uint8_t verify_nonce[OURA_AUTH_NONCE_LEN] = {0};
    if (len == OURA_PACKET_SIZE) {
        if (data[OURA_PACKET_CMD_OFFSET] != OURA_CMD_AUTH_CHALLENGE) {
            /* Not a verification write — ignore */
            return;
        }
        memcpy(verify_nonce, &data[OURA_PACKET_PAYLOAD_OFF], OURA_PACKET_PAYLOAD_LEN);
        verify_data = verify_nonce;
    }

    if (oura_auth_verify_response(verify_data)) {
        oura_gatt_mark_authenticated();

        /* Enable features from the most recent SetFeatureMode command,
         * or default to daytime HR + connected live */
        if (active_features == 0) {
            active_features = OURA_CAP_DAYTIME_HR | OURA_CAP_CONNECTED_LIVE;
        }

        start_measurement_timers();
        LOG_INF("Auth complete — measurements streaming");
    }
}

/* ── Command write callback ───────────────────────────────────────────── */

static void command_write_handler(const uint8_t *data, uint16_t len)
{
    if (len != OURA_PACKET_SIZE) {
        LOG_WRN("Command: wrong size %d", len);
        return;
    }

    if (!oura_packet_validate(data, len)) {
        LOG_WRN("Command: bad checksum");
        return;
    }

    uint8_t cmd = data[OURA_PACKET_CMD_OFFSET];

    switch (cmd) {
    case OURA_CMD_SET_FEATURE_MODE: {
        uint8_t features = oura_packet_parse_feature_mode(data, len);
        LOG_INF("SetFeatureMode: 0x%02X", features);
        active_features = features;

        /* If already authenticated, restart timers with new features */
        if (oura_gatt_is_connected_and_authenticated()) {
            stop_measurement_timers();
            start_measurement_timers();
        }
        break;
    }

    case OURA_CMD_REQUEST_LATEST: {
        /* Push one sample of each enabled type immediately */
        LOG_DBG("RequestLatest");
        uint8_t buf[OURA_PACKET_SIZE];

        if (active_features & OURA_CAP_DAYTIME_HR) {
            oura_meas_generate_ibi(buf);
            oura_measurement_notify(buf, OURA_PACKET_SIZE);
        }
        if (active_features & OURA_CAP_SPO2) {
            oura_meas_generate_spo2(buf);
            oura_measurement_notify(buf, OURA_PACKET_SIZE);
        }
        if (active_features & OURA_CAP_TEMPERATURE) {
            oura_meas_generate_temp(buf);
            oura_measurement_notify(buf, OURA_PACKET_SIZE);
        }
        break;
    }

    case OURA_CMD_START_HISTORY: {
        LOG_INF("StartHistory requested — history stream not implemented");
        /* History streaming is Phase 3. Send empty history marker. */
        break;
    }

    default:
        LOG_WRN("Unknown command: 0x%02X", cmd);
        break;
    }
}

/* ── Auth verification routing ─────────────────────────────────────────── */

/*
 * The auth flow in practice: the client first writes a 16-byte nonce to the
 * Auth characteristic (triggering auth_write_handler above).  Then the client
 * writes the decrypted nonce back to the Auth characteristic to verify.
 *
 * We need to distinguish between the initial challenge write and the
 * verification write.  Strategy: after handling a challenge, any subsequent
 * write to Auth that is NOT a cmdAuthChallenge (0x11) is treated as a
 * verification attempt.
 */
static bool challenge_handled;

static void auth_write_dispatcher(const uint8_t *data, uint16_t len)
{
    if (len < 1) return;

    /* Is this a challenge write (first step of auth)? */
    if (len == OURA_AUTH_NONCE_LEN && !challenge_handled) {
        auth_write_handler(data, len);
        challenge_handled = true;
        return;
    }

    /* 16-byte packet: check command byte */
    if (len == OURA_PACKET_SIZE) {
        if (oura_packet_validate(data, len)) {
            uint8_t cmd = data[OURA_PACKET_CMD_OFFSET];
            if (cmd == OURA_CMD_AUTH_CHALLENGE && !challenge_handled) {
                auth_write_handler(data, len);
                challenge_handled = true;
                return;
            }
        }
    }

    /* Anything else after challenge handled → verification write */
    if (challenge_handled) {
        auth_verify_handler(data, len);
        challenge_handled = false;
    }
}

/* ── Measurement notification enabled callback ────────────────────────── */

static void measurement_notify_enabled_handler(bool enabled)
{
    LOG_INF("Measurement notifications: %s", enabled ? "ENABLED" : "DISABLED");
    if (!enabled) {
        stop_measurement_timers();
    }
}

/* ── Timer callbacks ──────────────────────────────────────────────────── */

static void ibi_timer_cb(struct k_timer *timer)
{
    (void)timer;

    if (!oura_gatt_is_connected_and_authenticated()) return;
    if (!(active_features & OURA_CAP_DAYTIME_HR)) return;
    if (!(active_features & OURA_CAP_CONNECTED_LIVE)) return;

    uint8_t buf[OURA_PACKET_SIZE];
    oura_meas_generate_ibi(buf);
    oura_measurement_notify(buf, OURA_PACKET_SIZE);
}

static void spo2_timer_cb(struct k_timer *timer)
{
    (void)timer;

    if (!oura_gatt_is_connected_and_authenticated()) return;
    if (!(active_features & OURA_CAP_SPO2)) return;

    uint8_t buf[OURA_PACKET_SIZE];
    oura_meas_generate_spo2(buf);
    oura_measurement_notify(buf, OURA_PACKET_SIZE);
}

static void temp_timer_cb(struct k_timer *timer)
{
    (void)timer;

    if (!oura_gatt_is_connected_and_authenticated()) return;
    if (!(active_features & OURA_CAP_TEMPERATURE)) return;

    uint8_t buf[OURA_PACKET_SIZE];
    oura_meas_generate_temp(buf);
    oura_measurement_notify(buf, OURA_PACKET_SIZE);
}

static void sleep_timer_cb(struct k_timer *timer)
{
    (void)timer;

    if (!oura_gatt_is_connected_and_authenticated()) return;

    uint8_t buf[OURA_PACKET_SIZE];
    oura_meas_generate_sleep(buf);
    oura_measurement_notify(buf, OURA_PACKET_SIZE);
}

static void battery_timer_cb(struct k_timer *timer)
{
    (void)timer;

    if (!oura_gatt_is_connected_and_authenticated()) return;

    uint8_t buf[OURA_PACKET_SIZE];
    oura_meas_generate_battery(buf);
    oura_measurement_notify(buf, OURA_PACKET_SIZE);
}

/* ── Timer management ─────────────────────────────────────────────────── */

static void start_measurement_timers(void)
{
    const struct oura_scenario *s = oura_scenario_active();

    if ((active_features & OURA_CAP_DAYTIME_HR)
        && (active_features & OURA_CAP_CONNECTED_LIVE)
        && s->ibi_interval_ms > 0) {
        k_timer_start(&ibi_timer,
            K_MSEC(500),
            K_MSEC(s->ibi_interval_ms));
        LOG_INF("IBI timer started: %u ms", s->ibi_interval_ms);
    }

    if ((active_features & OURA_CAP_SPO2) && s->spo2_interval_ms > 0) {
        k_timer_start(&spo2_timer,
            K_MSEC(1000),
            K_MSEC(s->spo2_interval_ms));
        LOG_INF("SpO2 timer started: %u ms", s->spo2_interval_ms);
    }

    if (s->temp_interval_ms > 0) {
        k_timer_start(&temp_timer,
            K_MSEC(5000),
            K_MSEC(s->temp_interval_ms));
    }

    if (s->sleep_interval_ms > 0) {
        k_timer_start(&sleep_timer,
            K_MSEC(2000),
            K_MSEC(s->sleep_interval_ms));
    }

    if (s->battery_interval_ms > 0) {
        k_timer_start(&battery_timer,
            K_MSEC(10000),
            K_MSEC(s->battery_interval_ms));
    }
}

static void stop_measurement_timers(void)
{
    k_timer_stop(&ibi_timer);
    k_timer_stop(&spo2_timer);
    k_timer_stop(&temp_timer);
    k_timer_stop(&sleep_timer);
    k_timer_stop(&battery_timer);
    LOG_INF("Measurement timers stopped");
}

/* ── Accelerometer thread (50 Hz) ────────────────────────────────────── */

static void accel_thread_fn(void *arg1, void *arg2, void *arg3)
{
    (void)arg1;
    (void)arg2;
    (void)arg3;

    LOG_INF("Accel thread started");

    while (1) {
        const struct oura_scenario *s = oura_scenario_active();
        uint32_t interval_ms = s ? s->accel_interval_ms : 0;

        if (!oura_gatt_is_connected_and_authenticated()
            || !(active_features & OURA_CAP_ACCELEROMETER)
            || interval_ms == 0) {
            k_msleep(100);
            continue;
        }

        uint8_t buf[OURA_PACKET_SIZE];
        if (oura_meas_generate_accel(buf)) {
            oura_measurement_notify(buf, OURA_PACKET_SIZE);
        }

        k_msleep(interval_ms);
    }
}

/* ── LED control ──────────────────────────────────────────────────────── */

static void led_on(void)
{
    if (device_is_ready(led_green.port)) {
        gpio_pin_set_dt(&led_green, 1);
    }
}

static void led_off(void)
{
    if (device_is_ready(led_green.port)) {
        gpio_pin_set_dt(&led_green, 0);
    }
}

/* ── Connection callback: reset auth state on new connection ──────── */

static void connection_handler(bool connected)
{
    if (connected) {
        challenge_handled = false;
        oura_auth_reset();
        active_features = 0;
        LOG_INF("New connection — auth state reset");
    } else {
        stop_measurement_timers();
        challenge_handled = false;
        oura_auth_reset();
        active_features = 0;
        LOG_INF("Disconnected — timers stopped, auth reset");
    }
}

static void connected_cb(struct bt_conn *conn, uint8_t err)
{
    if (err) {
        LOG_WRN("Connection failed: 0x%02x", err);
        return;
    }
    oura_gatt_on_connected(conn);
}

static void advertising_restart_handler(struct k_work *work)
{
    (void)work;

    /* Restart outside the Bluetooth disconnect callback, after the controller
     * has fully released the previous connection slot. */
    int err = bt_le_adv_start(BT_LE_ADV_CONN_FAST_1, ad, ARRAY_SIZE(ad),
        sd, ARRAY_SIZE(sd));
    if (err == -EALREADY) {
        LOG_INF("Advertising already active after disconnect");
    } else if (err) {
        LOG_ERR("Advertising restart failed: %d; retrying", err);
        k_work_reschedule(&advertising_restart_work, K_SECONDS(1));
    } else {
        LOG_INF("Advertising restarted after disconnect");
    }
}

static void disconnected_cb(struct bt_conn *conn, uint8_t reason)
{
    LOG_INF("Disconnected: 0x%02x", reason);
    oura_gatt_on_disconnected(conn);
    k_work_reschedule(&advertising_restart_work, K_MSEC(250));
}

BT_CONN_CB_DEFINE(conn_callbacks) = {
    .connected = connected_cb,
    .disconnected = disconnected_cb,
};

/* ── BLE ready callback ───────────────────────────────────────────────── */

static void bt_ready(int err)
{
    if (err) {
        LOG_ERR("BLE init failed: %d", err);
        return;
    }

    LOG_INF("BLE initialized");

    /* Register GATT service */
    oura_gatt_init(auth_write_dispatcher, command_write_handler,
        measurement_notify_enabled_handler, NULL, connection_handler);

    /* Start advertising */
    err = bt_le_adv_start(BT_LE_ADV_CONN_FAST_1, ad, ARRAY_SIZE(ad),
        sd, ARRAY_SIZE(sd));
    if (err) {
        LOG_ERR("Advertising failed: %d", err);
        return;
    }

    LOG_INF("Advertising as '%s' — Oura Ring BLE Emulator", DEVICE_NAME);
    LOG_INF("Service UUID: 00001523-1212-efde-1523-785feabcd123");
}

/* ── main ─────────────────────────────────────────────────────────────── */

int main(void)
{
    int err;

    LOG_INF("=== Oura Ring BLE Emulator Dongle ===");

    /* ── GPIO: LED ── */
    if (!device_is_ready(led_green.port)) {
        LOG_WRN("Green LED not available");
    } else {
        gpio_pin_configure_dt(&led_green, GPIO_OUTPUT_INACTIVE);
    }

    /* ── Auth: init with default key ── */
    err = oura_auth_init(NULL);
    if (err) {
        LOG_ERR("Auth init failed: %d", err);
        return err;
    }

    /* ── Measurements: seed PRNG ── */
    oura_measurements_init(0);

    /* ── Scenario: start with 'normal' ── */
    oura_scenario_activate(oura_scenario_get(SCENARIO_NORMAL));
    active_features = 0;
    challenge_handled = false;

    /* ── Timers ── */
    k_work_init_delayable(&advertising_restart_work, advertising_restart_handler);
    k_timer_init(&ibi_timer, ibi_timer_cb, NULL);
    k_timer_init(&spo2_timer, spo2_timer_cb, NULL);
    k_timer_init(&temp_timer, temp_timer_cb, NULL);
    k_timer_init(&sleep_timer, sleep_timer_cb, NULL);
    k_timer_init(&battery_timer, battery_timer_cb, NULL);

    /* ── Accel thread ── */
    k_thread_create(&accel_thread_data, accel_stack,
        K_THREAD_STACK_SIZEOF(accel_stack),
        accel_thread_fn, NULL, NULL, NULL,
        K_PRIO_PREEMPT(5), 0, K_NO_WAIT);

    /* ── BLE stack ── */
    err = bt_enable(bt_ready);
    if (err) {
        LOG_ERR("BLE enable failed: %d", err);
        return err;
    }

    /* ── USB console (blocks briefly for USB enum) ── */
    /* Start console after a short delay to let USB enumerate */
    k_sleep(K_MSEC(500));
    oura_console_init();

    /* ── Main idle loop ── */
    led_on(); /* Green LED on = advertising */
    LOG_INF("Main loop: waiting for BLE connections...");

    while (1) {
        /* LED pattern:
         *   Advertising (no connection): LED solid
         *   Connected (not auth'd):      LED fast blink
         *   Authenticated + streaming:   LED slow blink
         *   Disconnected:                LED off briefly then on
         */
        if (oura_gatt_is_connected_and_authenticated()) {
            /* Slow blink: 1s on, 1s off */
            led_on();
            k_msleep(1000);
            led_off();
            k_msleep(1000);
        } else if (oura_gatt_current_conn()) {
            /* Fast blink: 200ms on, 200ms off */
            led_on();
            k_msleep(200);
            led_off();
            k_msleep(200);
        } else {
            /* Solid on */
            led_on();
            k_msleep(5000);
        }
    }

    return 0;
}
