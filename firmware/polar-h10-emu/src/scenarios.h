#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

enum polar_scenario_id {
    SCENARIO_NORMAL = 0,
    SCENARIO_SLEEP,
    SCENARIO_EXERCISE,
    SCENARIO_AFIB,
    SCENARIO_COUNT
};

struct polar_scenario {
    const char *name;
    enum polar_scenario_id id;

    uint16_t hr_min;
    uint16_t hr_max;

    int32_t hrv_jitter_ms; // Amplitude of random jitter added to RR interval
};

const struct polar_scenario *polar_scenario_get(enum polar_scenario_id id);
const struct polar_scenario *polar_scenario_by_name(const char *name);
const struct polar_scenario *polar_scenario_active(void);
void polar_scenario_activate(const struct polar_scenario *s);
void polar_scenario_list_all(void (*print_fn)(const char *line));

// Modify currently active scenario properties (used by serial console)
void polar_scenario_set_hr(uint16_t min_bpm, uint16_t max_bpm);
void polar_scenario_set_jitter(int32_t jitter_ms);

#ifdef __cplusplus
}
#endif
