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
VALUES (9, 'respiratory_rate', 'bpm', 1.0, 120.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (10, 'patient_flow', 'L/s', -20.0, 20.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (11, 'mask_pressure', 'cmH2O', 0.0, 50.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (12, 'mask_pressure_2s', 'cmH2O', 0.0, 50.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (13, 'inspiratory_pressure', 'cmH2O', 0.0, 50.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (14, 'expiratory_pressure', 'cmH2O', 0.0, 50.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (15, 'leak', 'L/min', 0.0, 300.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (16, 'tidal_volume', 'mL', 0.0, 5000.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (17, 'minute_ventilation', 'L/min', 0.0, 60.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (18, 'target_minute_ventilation', 'L/min', 0.0, 60.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (19, 'ie_ratio', 'ratio', 0.0, 10.0, 0.01)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (20, 'snore_index', 'index', 0.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (21, 'flow_limitation', 'ratio', 0.0, 1.0, 0.01)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (22, 'inspiratory_duration', 'seconds', 0.0, 10.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (23, 'trigger_cycle_event', 'code', 0.0, 255.0, 1.0)
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
VALUES (5, 'resmed_air11', 'ble')
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
