/*
 * scenarios.h — preset scenario definitions for the measurement engine
 *
 * Scenarios:
 *   normal    — healthy adult, HR 60–80, SpO2 96–99%, minimal motion
 *   desat     — moderate desaturation, SpO2 88–94%, HR normal
 *   sleep     — low HR, periodic SpO2 dips, sleep stage cycling
 *   exercise  — elevated HR 110–150, stable SpO2, active accel
 *   idle      — no measurements (simulates ring-off-finger)
 */

#ifndef SCENARIOS_H
#define SCENARIOS_H

#include <stdint.h>
#include <stdbool.h>

#include "measurements.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ── Scenario identifiers ──────────────────────────────────────── */

enum oura_scenario_id {
    SCENARIO_NORMAL = 0,
    SCENARIO_DESAT,
    SCENARIO_SLEEP,
    SCENARIO_EXERCISE,
    SCENARIO_IDLE,
    SCENARIO_COUNT
};

struct oura_scenario {
    const char *name;                      /* display name */
    enum oura_scenario_id id;

    struct oura_meas_ibi_params   ibi;
    struct oura_meas_spo2_params  spo2;
    struct oura_meas_accel_params accel;
    struct oura_meas_temp_params  temp;
    struct oura_meas_sleep_params sleep;
    struct oura_meas_battery_params battery;

    /* Measurement timing (ms) — zero means "don't stream this type" */
    uint32_t ibi_interval_ms;    /* typical ~900 ms */
    uint32_t spo2_interval_ms;   /* typical ~2000 ms */
    uint32_t accel_interval_ms;  /* typical 20 ms (50 Hz) */
    uint32_t temp_interval_ms;   /* typical 60000 ms */
    uint32_t sleep_interval_ms;  /* zero disables sleep-stage notifications */
    uint32_t battery_interval_ms;/* typical 60000 ms */
};

/* ── API ───────────────────────────────────────────────────────── */

/**
 * Get a pointer to the preset scenario definition.
 * Returns NULL if id is out of range.
 */
const struct oura_scenario *oura_scenario_get(enum oura_scenario_id id);

/**
 * Look up a scenario by name.  Case-insensitive prefix match is fine
 * (e.g. "n", "no", "norm", "normal" all match SCENARIO_NORMAL).
 * Returns NULL if no match.
 */
const struct oura_scenario *oura_scenario_by_name(const char *name);

/**
 * Return the currently active scenario pointer.
 */
const struct oura_scenario *oura_scenario_active(void);

/**
 * Activate a scenario: copies parameters into the measurement engine
 * and restarts timers as needed.
 */
void oura_scenario_activate(const struct oura_scenario *s);

/**
 * Print scenario names to the USB console (one per line).
 */
void oura_scenario_list_all(void (*print_fn)(const char *line));

#ifdef __cplusplus
}
#endif

#endif /* SCENARIOS_H */
