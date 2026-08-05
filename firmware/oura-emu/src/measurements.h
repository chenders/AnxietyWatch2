/*
 * measurements.h — synthetic physiological data generators
 *
 * Each generator produces a 16-byte packet for its measurement type.
 * The generators use a shared random-walk state so values drift naturally
 * rather than jumping randomly.  Scenario parameters (HR range, SpO2 range,
 * etc.) are injected from the active scenario via the scenario engine.
 *
 * All generators are called from timer / work-queue context (non-ISR).
 */

#ifndef MEASUREMENTS_H
#define MEASUREMENTS_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Generator parameters (set per scenario) ───────────────────── */

/** Parameters controlling IBI / HR generation. */
struct oura_meas_ibi_params {
    int hr_min_bpm;       /* floor heart rate */
    int hr_max_bpm;       /* ceiling heart rate */
    int ibi_mean_ms;      /* target mean IBI (overrides hr_min/hr_max if > 0) */
    int ibi_jitter_ms;    /* per-beat random walk step (± ms) */
};

/** Parameters controlling SpO2 generation. */
struct oura_meas_spo2_params {
    int spo2_min_pct;
    int spo2_max_pct;
    int quality;          /* fixed signal quality 0–15, or -1 for random */
};

/** Parameters controlling accelerometer generation. */
struct oura_meas_accel_params {
    bool active;          /* true = streaming at 50 Hz */
    int max_mg;           /* max magnitude per axis (default 2000 = ±2g) */
};

/** Parameters controlling temperature generation. */
struct oura_meas_temp_params {
    int deviation_hundredths;  /* baseline deviation in °C × 100 */
    int drift_hundredths;      /* per-sample random walk step */
};

/** Sleep stage override (0=awake, 1=light, 2=deep, 3=REM, -1=auto). */
struct oura_meas_sleep_params {
    int forced_stage;     /* -1 to use scenario auto-progression */
};

/** Battery params. */
struct oura_meas_battery_params {
    int percent;
    bool charging;
};

/* ── Initialization ────────────────────────────────────────────── */

/**
 * Seed the internal random-walk state.  Call once at startup.
 * If seed == 0 the current uptime tick is used.
 */
void oura_measurements_init(uint32_t seed);

/* ── Parameter setters ──────────────────────────────────────────── */

void oura_meas_set_ibi_params(const struct oura_meas_ibi_params *p);
void oura_meas_set_spo2_params(const struct oura_meas_spo2_params *p);
void oura_meas_set_accel_params(const struct oura_meas_accel_params *p);
void oura_meas_set_temp_params(const struct oura_meas_temp_params *p);
void oura_meas_set_sleep_params(const struct oura_meas_sleep_params *p);
void oura_meas_set_battery_params(const struct oura_meas_battery_params *p);

/* ── Generators — each fills a 16-byte packet ────────────────────── */

/**
 * Generate an IBI (tag 0x2F, sub-tag 40) packet.
 * buf must be 16 bytes.
 */
void oura_meas_generate_ibi(uint8_t *buf);

/**
 * Generate a SpO2 (tag 0x07) packet.
 */
void oura_meas_generate_spo2(uint8_t *buf);

/**
 * Generate an accelerometer (tag 0x06) packet.
 * Returns false if accel is disabled.
 */
bool oura_meas_generate_accel(uint8_t *buf);

/**
 * Generate a temperature (tag 0x08) packet.
 */
void oura_meas_generate_temp(uint8_t *buf);

/**
 * Generate a sleep stage (tag 0x09) packet.
 * If forced_stage >= 0 it always returns that stage.
 * Otherwise uses internal auto-progression (mostly awake during
 * 'normal'/'exercise', cycling through stages during 'sleep').
 */
void oura_meas_generate_sleep(uint8_t *buf);

/**
 * Generate a battery (tag 0x0A) packet.
 */
void oura_meas_generate_battery(uint8_t *buf);

/**
 * Call at the start of each scenario to reset random-walk values
 * to the centre of the new range.
 */
void oura_meas_reset_walk(void);

#ifdef __cplusplus
}
#endif

#endif /* MEASUREMENTS_H */
