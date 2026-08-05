/*
 * measurements.c — synthetic physiological data generators
 *
 * Each generator maintains an internal random-walk state so values drift
 * smoothly.  Scenario parameters (injected via the scenario engine) set
 * range limits and drift step sizes.
 *
 * Random number source: a simple xorshift32 PRNG seeded at init.
 * Not cryptographically secure — fine for synthetic data.
 */

#include "measurements.h"
#include "packet.h"

#include <string.h>
#include <stdlib.h>

#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>

LOG_MODULE_REGISTER(meas, LOG_LEVEL_DBG);

/* ── PRNG (xorshift32) ─────────────────────────────────────────────── */

static uint32_t prng_state;

static uint32_t prng_u32(void) {
    uint32_t x = prng_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    prng_state = x;
    return x;
}

/** Returns a pseudo-random float in [-jitter, +jitter] (integer step). */
static int prng_jitter(int jitter) {
    if (jitter <= 0) return 0;
    uint32_t r = prng_u32() % (uint32_t)(2 * jitter + 1);
    return (int)r - jitter;
}

/* ── Parameter storage ─────────────────────────────────────────────── */

static struct oura_meas_ibi_params   ibi_params   = {60, 80, 0, 5};
static struct oura_meas_spo2_params  spo2_params  = {96, 99, -1};
static struct oura_meas_accel_params accel_params = {false, 2000};
static struct oura_meas_temp_params  temp_params  = {0, 1};
static struct oura_meas_sleep_params sleep_params = {-1};
static struct oura_meas_battery_params battery_params = {100, false};

/* Random-walk state (initialized by oura_meas_reset_walk) */
static int walk_ibi_ms;        /* current IBI value in ms */
static int walk_spo2_pct;      /* current SpO2 % */
static int walk_temp;          /* current temp deviation × 100 */
static int sleep_stage_cycle;  /* counter for auto-progression */

/* ── Initialization ────────────────────────────────────────────────── */

void oura_measurements_init(uint32_t seed)
{
    if (seed == 0) {
        seed = (uint32_t)k_uptime_get_32();
    }
    prng_state = seed;
    oura_meas_reset_walk();
    LOG_INF("Measurements seeded: 0x%08X", seed);
}

/* ── Parameter setters ─────────────────────────────────────────────── */

void oura_meas_set_ibi_params(const struct oura_meas_ibi_params *p) {
    memcpy(&ibi_params, p, sizeof(ibi_params));
}
void oura_meas_set_spo2_params(const struct oura_meas_spo2_params *p) {
    memcpy(&spo2_params, p, sizeof(spo2_params));
}
void oura_meas_set_accel_params(const struct oura_meas_accel_params *p) {
    memcpy(&accel_params, p, sizeof(accel_params));
}
void oura_meas_set_temp_params(const struct oura_meas_temp_params *p) {
    memcpy(&temp_params, p, sizeof(temp_params));
}
void oura_meas_set_sleep_params(const struct oura_meas_sleep_params *p) {
    memcpy(&sleep_params, p, sizeof(sleep_params));
}
void oura_meas_set_battery_params(const struct oura_meas_battery_params *p) {
    memcpy(&battery_params, p, sizeof(battery_params));
}

/* ── Reset walk ────────────────────────────────────────────────────── */

void oura_meas_reset_walk(void)
{
    /* IBI: start at mid-range HR */
    int mid_hr = (ibi_params.hr_min_bpm + ibi_params.hr_max_bpm) / 2;
    if (mid_hr > 0) {
        walk_ibi_ms = 60000 / mid_hr;
    } else if (ibi_params.ibi_mean_ms > 0) {
        walk_ibi_ms = ibi_params.ibi_mean_ms;
    } else {
        walk_ibi_ms = 800;
    }

    /* SpO2 */
    walk_spo2_pct = (spo2_params.spo2_min_pct + spo2_params.spo2_max_pct) / 2;

    /* Temp */
    walk_temp = temp_params.deviation_hundredths;

    /* Sleep cycle */
    sleep_stage_cycle = 0;
}

/* ── Random-walk helper ───────────────────────────────────────────── */

/**
 * Clamp val to [lo, hi] and apply random walk within jitter.
 */
static int walk_step(int *val, int lo, int hi, int jitter)
{
    if (jitter <= 0) jitter = 1;
    *val += prng_jitter(jitter);
    if (*val < lo) *val = lo;
    if (*val > hi) *val = hi;
    return *val;
}

/* ── Generators ───────────────────────────────────────────────────── */

void oura_meas_generate_ibi(uint8_t *buf)
{
    /* Compute IBI target range from HR params or explicit IBI params */
    int ibi_lo, ibi_hi;
    if (ibi_params.ibi_mean_ms > 0) {
        ibi_lo = ibi_params.ibi_mean_ms - ibi_params.ibi_jitter_ms * 3;
        ibi_hi = ibi_params.ibi_mean_ms + ibi_params.ibi_jitter_ms * 3;
        if (ibi_lo < 300) ibi_lo = 300;  /* min IBI = 200 BPM cap */
        if (ibi_hi > 2000) ibi_hi = 2000; /* max IBI = 30 BPM cap */
    } else {
        /* Convert HR range to IBI range */
        int hr_lo = ibi_params.hr_min_bpm;
        int hr_hi = ibi_params.hr_max_bpm;
        if (hr_lo <= 0) hr_lo = 40;
        if (hr_hi <= 0) hr_hi = 150;
        ibi_lo = 60000 / hr_hi;
        ibi_hi = 60000 / hr_lo;
    }

    int jitter = ibi_params.ibi_jitter_ms;
    if (jitter <= 0) jitter = 5;

    int ibi = walk_step(&walk_ibi_ms, ibi_lo, ibi_hi, jitter);

    /* Build packet: [0x2F][subtag=40][IBI_lo][IBI_hi][reserved...][XOR] */
    oura_packet_clear_payload(buf);
    buf[OURA_PACKET_CMD_OFFSET] = OURA_TAG_LIVE_HR;
    buf[OURA_PACKET_PAYLOAD_OFF]     = OURA_SUBTAG_IBI;
    buf[OURA_PACKET_PAYLOAD_OFF + 1] = (uint8_t)(ibi & 0xFF);
    buf[OURA_PACKET_PAYLOAD_OFF + 2] = (uint8_t)((ibi >> 8) & 0xFF);
    oura_packet_finalize(buf);
}

void oura_meas_generate_spo2(uint8_t *buf)
{
    int lo = spo2_params.spo2_min_pct;
    int hi = spo2_params.spo2_max_pct;
    if (lo < 50) lo = 50;
    if (hi > 100) hi = 100;

    int spo2 = walk_step(&walk_spo2_pct, lo, hi, 1);

    int quality = spo2_params.quality;
    if (quality < 0) {
        /* Auto quality: higher SpO2 → better signal */
        quality = (spo2 >= 95) ? 14 : ((spo2 >= 90) ? 10 : 6);
        quality += prng_jitter(1);
        if (quality < 0) quality = 0;
        if (quality > 15) quality = 15;
    }

    /* Build packet: [0x07][SpO2][quality<<4|contact][reserved][XOR] */
    oura_packet_clear_payload(buf);
    buf[OURA_PACKET_CMD_OFFSET] = OURA_TAG_SPO2;
    buf[OURA_PACKET_PAYLOAD_OFF]     = (uint8_t)spo2;
    buf[OURA_PACKET_PAYLOAD_OFF + 1] = (uint8_t)((quality << 4) | 1); /* skin contact=1 */
    oura_packet_finalize(buf);
}

bool oura_meas_generate_accel(uint8_t *buf)
{
    if (!accel_params.active) {
        return false;
    }

    int max_mg = accel_params.max_mg;
    if (max_mg <= 0) max_mg = 100;

    int16_t x = (int16_t)prng_jitter(max_mg);
    int16_t y = (int16_t)prng_jitter(max_mg);
    int16_t z = (int16_t)(prng_jitter(max_mg / 4) - 1000); /* slight -Z bias (gravity) */

    /* Build packet: [0x06][X_lo][X_hi][Y_lo][Y_hi][Z_lo][Z_hi][reserved][XOR] */
    oura_packet_clear_payload(buf);
    buf[OURA_PACKET_CMD_OFFSET] = OURA_TAG_ACCELEROMETER;
    buf[OURA_PACKET_PAYLOAD_OFF]     = (uint8_t)(x & 0xFF);
    buf[OURA_PACKET_PAYLOAD_OFF + 1] = (uint8_t)((x >> 8) & 0xFF);
    buf[OURA_PACKET_PAYLOAD_OFF + 2] = (uint8_t)(y & 0xFF);
    buf[OURA_PACKET_PAYLOAD_OFF + 3] = (uint8_t)((y >> 8) & 0xFF);
    buf[OURA_PACKET_PAYLOAD_OFF + 4] = (uint8_t)(z & 0xFF);
    buf[OURA_PACKET_PAYLOAD_OFF + 5] = (uint8_t)((z >> 8) & 0xFF);
    oura_packet_finalize(buf);
    return true;
}

void oura_meas_generate_temp(uint8_t *buf)
{
    int drift = temp_params.drift_hundredths;
    if (drift <= 0) drift = 1;
    (void)walk_step(&walk_temp, -500, 500, drift);

    int16_t dev = (int16_t)walk_temp;

    /* Build packet: [0x08][dev_lo][dev_hi][reserved][XOR] */
    oura_packet_clear_payload(buf);
    buf[OURA_PACKET_CMD_OFFSET] = OURA_TAG_TEMPERATURE;
    buf[OURA_PACKET_PAYLOAD_OFF]     = (uint8_t)(dev & 0xFF);
    buf[OURA_PACKET_PAYLOAD_OFF + 1] = (uint8_t)((dev >> 8) & 0xFF);
    oura_packet_finalize(buf);
}

void oura_meas_generate_sleep(uint8_t *buf)
{
    int stage;

    if (sleep_params.forced_stage >= 0 && sleep_params.forced_stage <= 3) {
        stage = sleep_params.forced_stage;
    } else {
        /* Auto-progression: cycle through stages very slowly.
         * Change every ~12 calls (simulating ~12 min per stage during sleep). */
        sleep_stage_cycle++;
        int cycle_pos = (sleep_stage_cycle / 12) % 4;

        /* Map: 0=awake, 1=light, 2=deep, 3=REM, 2=deep, 1=light, ... */
        /* Simplified: just cycle through 0→1→2→3→2→1→0… */
        static const int sleep_map[] = {0, 1, 2, 3, 2, 1};
        stage = sleep_map[cycle_pos % 6];
    }

    /* Build packet: [0x09][stage][reserved][XOR] */
    oura_packet_clear_payload(buf);
    buf[OURA_PACKET_CMD_OFFSET] = OURA_TAG_SLEEP_STAGE;
    buf[OURA_PACKET_PAYLOAD_OFF] = (uint8_t)stage;
    oura_packet_finalize(buf);
}

void oura_meas_generate_battery(uint8_t *buf)
{
    /* Battery drains slowly if not charging */
    if (!battery_params.charging && battery_params.percent > 0) {
        /* ~0.001% per call = ~3.6%/hr at 60s interval */
        battery_params.percent -= (prng_u32() % 100 == 0) ? 1 : 0;
        if (battery_params.percent < 0) battery_params.percent = 0;
    }

    /* Build packet: [0x0A][%][flags][reserved][XOR] */
    oura_packet_clear_payload(buf);
    buf[OURA_PACKET_CMD_OFFSET] = OURA_TAG_BATTERY;
    buf[OURA_PACKET_PAYLOAD_OFF]     = (uint8_t)battery_params.percent;
    buf[OURA_PACKET_PAYLOAD_OFF + 1] = battery_params.charging ? 1 : 0;
    oura_packet_finalize(buf);
}
