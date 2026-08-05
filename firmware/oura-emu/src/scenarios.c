/*
 * scenarios.c — preset scenario definitions
 *
 * Five presets: normal, desat, sleep, exercise, idle.
 * Each scenario sets measurement parameter ranges and timer intervals.
 */

#include "scenarios.h"

#include <string.h>
#include <ctype.h>

#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>

LOG_MODULE_REGISTER(scenarios, LOG_LEVEL_DBG);

/* ── Scenario definitions ────────────────────────────────────────────── */

static const struct oura_scenario scenario_table[] = {
    [SCENARIO_NORMAL] = {
        .name = "normal",
        .ibi   = { .hr_min_bpm = 60, .hr_max_bpm = 80, .ibi_mean_ms = 0, .ibi_jitter_ms = 5 },
        .spo2  = { .spo2_min_pct = 96, .spo2_max_pct = 99, .quality = -1 },
        .accel = { .active = false, .max_mg = 100 },
        .temp  = { .deviation_hundredths = 0, .drift_hundredths = 1 },
        .sleep = { .forced_stage = 0 },  /* awake */
        .battery = { .percent = 85, .charging = false },
        .ibi_interval_ms    = 900,
        .spo2_interval_ms   = 3000,
        .accel_interval_ms  = 0,      /* off */
        .temp_interval_ms   = 60000,
        .sleep_interval_ms  = 30000,
        .battery_interval_ms = 60000,
    },

    [SCENARIO_DESAT] = {
        .name = "desat",
        .ibi   = { .hr_min_bpm = 65, .hr_max_bpm = 90, .ibi_mean_ms = 0, .ibi_jitter_ms = 8 },
        .spo2  = { .spo2_min_pct = 88, .spo2_max_pct = 94, .quality = 8 },
        .accel = { .active = false, .max_mg = 100 },
        .temp  = { .deviation_hundredths = 0, .drift_hundredths = 1 },
        .sleep = { .forced_stage = -1 },  /* auto */
        .battery = { .percent = 85, .charging = false },
        .ibi_interval_ms    = 900,
        .spo2_interval_ms   = 2000,
        .accel_interval_ms  = 0,
        .temp_interval_ms   = 60000,
        .sleep_interval_ms  = 15000,
        .battery_interval_ms = 60000,
    },

    [SCENARIO_SLEEP] = {
        .name = "sleep",
        .ibi   = { .hr_min_bpm = 48, .hr_max_bpm = 62, .ibi_mean_ms = 0, .ibi_jitter_ms = 10 },
        .spo2  = { .spo2_min_pct = 90, .spo2_max_pct = 98, .quality = -1 },
        .accel = { .active = true, .max_mg = 30 },  /* small body movements */
        .temp  = { .deviation_hundredths = -20, .drift_hundredths = 1 },
        .sleep = { .forced_stage = -1 }, /* auto-cycling through stages */
        .battery = { .percent = 82, .charging = false },
        .ibi_interval_ms    = 1000,
        .spo2_interval_ms   = 2000,
        .accel_interval_ms  = 20,   /* 50 Hz */
        .temp_interval_ms   = 60000,
        .sleep_interval_ms  = 5000,
        .battery_interval_ms = 60000,
    },

    [SCENARIO_EXERCISE] = {
        .name = "exercise",
        .ibi   = { .hr_min_bpm = 110, .hr_max_bpm = 155, .ibi_mean_ms = 0, .ibi_jitter_ms = 15 },
        .spo2  = { .spo2_min_pct = 96, .spo2_max_pct = 99, .quality = 12 },
        .accel = { .active = true, .max_mg = 1500 }, /* running motion */
        .temp  = { .deviation_hundredths = 80, .drift_hundredths = 2 },
        .sleep = { .forced_stage = 0 },  /* awake */
        .battery = { .percent = 80, .charging = false },
        .ibi_interval_ms    = 600,
        .spo2_interval_ms   = 3000,
        .accel_interval_ms  = 20,   /* 50 Hz */
        .temp_interval_ms   = 30000,
        .sleep_interval_ms  = 30000,
        .battery_interval_ms = 60000,
    },

    [SCENARIO_IDLE] = {
        .name = "idle",
        .ibi   = { .hr_min_bpm = 60, .hr_max_bpm = 80, .ibi_mean_ms = 0, .ibi_jitter_ms = 0 },
        .spo2  = { .spo2_min_pct = 96, .spo2_max_pct = 99, .quality = -1 },
        .accel = { .active = false, .max_mg = 0 },
        .temp  = { .deviation_hundredths = 0, .drift_hundredths = 0 },
        .sleep = { .forced_stage = 0 },
        .battery = { .percent = 80, .charging = false },
        .ibi_interval_ms    = 0,     /* ALL OFF — idle = ring-off-finger */
        .spo2_interval_ms   = 0,
        .accel_interval_ms  = 0,
        .temp_interval_ms   = 0,
        .sleep_interval_ms  = 0,
        .battery_interval_ms = 0,
    },
};

static const struct oura_scenario *active_scenario = &scenario_table[SCENARIO_NORMAL];

/* ── API ──────────────────────────────────────────────────────────────── */

const struct oura_scenario *oura_scenario_get(enum oura_scenario_id id)
{
    if (id < 0 || id >= SCENARIO_COUNT) return NULL;
    return &scenario_table[id];
}

/**
 * Case-insensitive prefix match: "sle", "sleep", "SLEEP" all match "sleep".
 */
const struct oura_scenario *oura_scenario_by_name(const char *name)
{
    size_t len = strlen(name);

    for (int i = 0; i < SCENARIO_COUNT; i++) {
        const char *sname = scenario_table[i].name;
        /* Match: name is a prefix of scenario name, case-insensitive */
        if (len > 0 && strlen(sname) >= len) {
            bool match = true;
            for (size_t j = 0; j < len; j++) {
                if (tolower((unsigned char)name[j])
                    != tolower((unsigned char)sname[j])) {
                    match = false;
                    break;
                }
            }
            if (match) {
                return &scenario_table[i];
            }
        }
    }
    return NULL;
}

const struct oura_scenario *oura_scenario_active(void)
{
    return active_scenario;
}

void oura_scenario_activate(const struct oura_scenario *s)
{
    if (!s) return;

    active_scenario = s;

    /* Push parameters into the measurement engine */
    oura_meas_set_ibi_params(&s->ibi);
    oura_meas_set_spo2_params(&s->spo2);
    oura_meas_set_accel_params(&s->accel);
    oura_meas_set_temp_params(&s->temp);
    oura_meas_set_sleep_params(&s->sleep);
    oura_meas_set_battery_params(&s->battery);

    oura_meas_reset_walk();

    LOG_INF("Scenario activated: %s", s->name);
}

void oura_scenario_list_all(void (*print_fn)(const char *line))
{
    for (int i = 0; i < SCENARIO_COUNT; i++) {
        char buf[64];
        snprintf(buf, sizeof(buf), "  %-12s — IBI %d ms  SpO2 %d–%d%%",
            scenario_table[i].name,
            scenario_table[i].ibi_interval_ms,
            scenario_table[i].spo2.spo2_min_pct,
            scenario_table[i].spo2.spo2_max_pct);
        print_fn(buf);
    }
}
