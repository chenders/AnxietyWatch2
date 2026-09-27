-- GENERATED FROM registry.toml — DO NOT EDIT
-- Reference data for the registry lookup tables.
-- Structure lives in db/deploy/registry_tables.sql — sqitch owns it.
-- Ids are permanent: ON CONFLICT UPDATE so a renamed unit or widened
-- range propagates, but an id is never reused or renumbered.

INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (1, 'hr', 'bpm', 20.0, 250.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (2, 'spo2', 'percent', 50.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (3, 'accel', 'g', -16.0, 16.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (4, 'hrv_rmssd', 'ms', 1.0, 500.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (5, 'rr_interval', 'seconds', 0.2, 3.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (6, 'bp_systolic', 'mmHg', 50.0, 260.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (7, 'bp_diastolic', 'mmHg', 30.0, 160.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (8, 'bp_map', 'mmHg', 40.0, 200.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (9, 'sleep_stage', 'stage_code', 0.0, 4.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (10, 'resting_hr', 'bpm', 20.0, 250.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (11, 'respiratory_rate', 'breaths/min', 1.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (12, 'wrist_temperature', 'degC', -20.0, 60.0, 0.01)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (13, 'step_count', 'count', 0.0, 1000000.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (14, 'active_energy', 'kcal', 0.0, 100000.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (15, 'exercise_time', 'min', 0.0, 1440.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (16, 'environmental_audio', 'dBASPL', 0.0, 200.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (17, 'blood_glucose', 'mg/dL', 1.0, 1000.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (18, 'vo2_max', 'mL/kg*min', 1.0, 150.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (19, 'walking_hr', 'bpm', 20.0, 250.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (20, 'headphone_audio', 'dBASPL', 0.0, 200.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (21, 'walking_steadiness', 'percent', 0.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (22, 'afib_burden', 'percent', 0.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (23, 'walking_speed', 'm/s', 0.0, 20.0, 0.01)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (24, 'walking_step_length', 'm', 0.0, 5.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (25, 'walking_double_support', 'percent', 0.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (26, 'walking_asymmetry', 'percent', 0.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (27, 'time_in_daylight', 'min', 0.0, 1440.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (28, 'physical_effort', 'kcal/kg*hr', 0.0, 100.0, 0.01)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (29, 'body_mass', 'kg', 0.1, 500.0, 0.01)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;

INSERT INTO vitals_source (id, name, transport)
VALUES (0, 'emay', 'ble')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (1, 'polar_h10', 'ble')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (2, 'healthkit', 'push')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (3, 'derived', 'internal')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (4, 'acconeer_xe125', 'usb_serial')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (6, 'ad_ua651ble', 'ble')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (7, 'polar_h10_emulator', 'internal')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
