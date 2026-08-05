#include "measurements.h"
#include "scenarios.h"
#include "gatt_service.h"
#include <zephyr/kernel.h>
#include <zephyr/random/random.h>
#include <zephyr/logging/log.h>
#include <stdlib.h>

LOG_MODULE_REGISTER(measurements, LOG_LEVEL_DBG);

static struct k_timer meas_timer;
static struct k_work meas_work;

static uint32_t residual_ms = 0;
static uint16_t current_hr = 60;

static void meas_work_handler(struct k_work *work) {
    if (!gatt_is_subscribed()) {
        return;
    }

    const struct polar_scenario *scen = polar_scenario_active();

    // Smoothly drift HR towards bounds
    int32_t hr_step = (sys_rand32_get() % 3) - 1; // -1, 0, or 1
    current_hr = (uint16_t)((int32_t)current_hr + hr_step);

    if (current_hr < scen->hr_min) current_hr = scen->hr_min;
    if (current_hr > scen->hr_max) current_hr = scen->hr_max;
    if (current_hr == 0) current_hr = 60; // Failsafe

    uint32_t rr_target_ms = 60000 / current_hr;
    uint32_t ms_passed = 1000 + residual_ms;

    uint8_t payload[20];
    payload[0] = 0x10; // Flags: RR present, 8-bit HR format
    payload[1] = (uint8_t)current_hr;

    int rr_count = 0;
    int offset = 2;

    while (ms_passed >= rr_target_ms && rr_count < 8 && offset < 18) {
        // Add random jitter
        int32_t jitter = 0;
        if (scen->hrv_jitter_ms > 0) {
            jitter = (sys_rand32_get() % (scen->hrv_jitter_ms * 2 + 1)) - scen->hrv_jitter_ms;
        }

        uint32_t actual_rr_ms = rr_target_ms;
        if ((int32_t)actual_rr_ms + jitter > 0) {
            actual_rr_ms += jitter;
        } else {
            actual_rr_ms = 1;
        }

        // Convert ms to 1/1024s (ms * 1024 / 1000)
        uint16_t rr_1024 = (uint16_t)((actual_rr_ms * 1024) / 1000);

        payload[offset++] = (uint8_t)(rr_1024 & 0xFF);
        payload[offset++] = (uint8_t)(rr_1024 >> 8);

        // We only consume the nominal rr_target_ms so average HR remains correct,
        // even though individual beats are jittered.
        if (ms_passed > rr_target_ms) {
            ms_passed -= rr_target_ms;
        } else {
            ms_passed = 0;
        }
        rr_count++;
    }

    residual_ms = ms_passed;
    gatt_notify_hrm(payload, offset);
}

static void meas_timer_handler(struct k_timer *dummy) {
    k_work_submit(&meas_work);
}

void measurements_init(void) {
    k_work_init(&meas_work, meas_work_handler);
    k_timer_init(&meas_timer, meas_timer_handler, NULL);

    // Initialize default scenario
    polar_scenario_activate(polar_scenario_get(SCENARIO_NORMAL));
}

void measurements_start(void) {
    residual_ms = 0;
    current_hr = polar_scenario_active()->hr_min;
    k_timer_start(&meas_timer, K_SECONDS(1), K_SECONDS(1));
    LOG_INF("Measurements started");
}

void measurements_stop(void) {
    k_timer_stop(&meas_timer);
    LOG_INF("Measurements stopped");
}
