/* GENERATED FROM registry.toml — DO NOT EDIT */
#ifndef VITALS_TYPES_H
#define VITALS_TYPES_H

#ifdef __cplusplus
extern "C" {
#endif

// Vitals types
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


// Vitals sources
typedef enum {
    VITALS_SOURCE_EMAY = 0,
    VITALS_SOURCE_POLAR_H10 = 1,
    VITALS_SOURCE_HEALTHKIT = 2,
    VITALS_SOURCE_DERIVED = 3,
    VITALS_SOURCE_AD_UA651BLE = 6
} vitals_source_t;

// Type information functions
typedef struct {
    double lo;
    double hi;
} vitals_range_t;

vitals_range_t vitals_type_range(int type_id);
const char* vitals_type_unit(int type_id);

#ifdef __cplusplus
}
#endif

#endif // VITALS_TYPES_H
