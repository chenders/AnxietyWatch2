#include "scenarios.h"
#include <string.h>
#include <strings.h>

static struct polar_scenario scenarios[SCENARIO_COUNT] = {
    [SCENARIO_NORMAL] = {
        .name = "normal",
        .id = SCENARIO_NORMAL,
        .hr_min = 60,
        .hr_max = 80,
        .hrv_jitter_ms = 40,
    },
    [SCENARIO_SLEEP] = {
        .name = "sleep",
        .id = SCENARIO_SLEEP,
        .hr_min = 45,
        .hr_max = 55,
        .hrv_jitter_ms = 80, // Higher HRV at rest
    },
    [SCENARIO_EXERCISE] = {
        .name = "exercise",
        .id = SCENARIO_EXERCISE,
        .hr_min = 120,
        .hr_max = 160,
        .hrv_jitter_ms = 10, // Very low HRV under strain
    },
    [SCENARIO_AFIB] = {
        .name = "afib",
        .id = SCENARIO_AFIB,
        .hr_min = 80,
        .hr_max = 140,
        .hrv_jitter_ms = 250, // Massive erratic RR intervals
    },
};

static struct polar_scenario active_scenario;

const struct polar_scenario *polar_scenario_get(enum polar_scenario_id id) {
    if (id >= SCENARIO_COUNT) return NULL;
    return &scenarios[id];
}

const struct polar_scenario *polar_scenario_by_name(const char *name) {
    for (int i = 0; i < SCENARIO_COUNT; i++) {
        if (strcmp(scenarios[i].name, name) == 0) {
            return &scenarios[i];
        }
    }
    return NULL;
}

const struct polar_scenario *polar_scenario_active(void) {
    return &active_scenario;
}

void polar_scenario_activate(const struct polar_scenario *s) {
    if (!s) return;
    memcpy(&active_scenario, s, sizeof(struct polar_scenario));
}

void polar_scenario_list_all(void (*print_fn)(const char *line)) {
    for (int i = 0; i < SCENARIO_COUNT; i++) {
        print_fn(scenarios[i].name);
    }
}

void polar_scenario_set_hr(uint16_t min_bpm, uint16_t max_bpm) {
    active_scenario.hr_min = min_bpm;
    active_scenario.hr_max = max_bpm;
}

void polar_scenario_set_jitter(int32_t jitter_ms) {
    active_scenario.hrv_jitter_ms = jitter_ms;
}
