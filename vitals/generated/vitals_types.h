/* GENERATED FROM registry.toml — DO NOT EDIT */
#ifndef VITALS_TYPES_H
#define VITALS_TYPES_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    VITALS_TYPE_HR = 1,
    VITALS_TYPE_SPO2 = 2,
    VITALS_TYPE_ACCEL = 3,
    VITALS_TYPE_HRV_RMSSD = 4,
    VITALS_TYPE_RR_INTERVAL = 5,
    VITALS_TYPE_BP_SYSTOLIC = 6,
    VITALS_TYPE_BP_DIASTOLIC = 7,
    VITALS_TYPE_BP_MAP = 8
} vitals_type_t;

typedef enum {
    VITALS_SOURCE_EMAY = 0,
    VITALS_SOURCE_POLAR_H10 = 1,
    VITALS_SOURCE_HEALTHKIT = 2,
    VITALS_SOURCE_DERIVED = 3,
    VITALS_SOURCE_AD_UA651BLE = 6
} vitals_source_t;

#define VITALS_TYPE_COUNT   8
#define VITALS_SOURCE_COUNT 5

typedef struct {
    double lo;
    double hi;
} vitals_range_t;

/* Valid range for a type, from registry.toml. Returns false for an unknown id;
 * *out is untouched. Values outside the range are quarantined, never clamped. */
bool vitals_type_range(int type_id, vitals_range_t *out);

/* Canonical unit for a type. NULL for an unknown id. Conversion happens once,
 * at the decoder — the store holds exactly one unit per type. */
const char *vitals_type_unit(int type_id);

#ifdef __cplusplus
}
#endif

#endif /* VITALS_TYPES_H */
