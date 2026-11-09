-- GENERATED FROM registry.toml — DO NOT EDIT
-- Reference data for the registry lookup tables.
-- Structure lives in db/deploy/registry_tables.sql — sqitch owns it.
-- Ids are permanent: ON CONFLICT UPDATE so a renamed unit or widened
-- range propagates, but an id is never reused or renumbered.

-- Type ids are permanent, and so are their units and ranges.
DO $$
DECLARE
    conflict RECORD;
    row_count BIGINT;
BEGIN
    IF to_regclass('samples') IS NULL OR to_regclass('vitals_type') IS NULL THEN
        RETURN;  -- fresh database: nothing to protect yet
    END IF;
    FOR conflict IN
        SELECT stored.id,
               stored.name AS stored_name, declared.name AS declared_name,
               stored.unit AS stored_unit, declared.unit AS declared_unit
        FROM vitals_type AS stored
        JOIN (VALUES
        (1, 'hr', 'bpm', 20.0, 250.0),
        (2, 'spo2', 'percent', 50.0, 100.0),
        (3, 'accel', 'g', -16.0, 16.0),
        (4, 'hrv_rmssd', 'ms', 1.0, 500.0),
        (5, 'rr_interval', 'seconds', 0.2, 3.0),
        (6, 'bp_systolic', 'mmHg', 50.0, 260.0),
        (7, 'bp_diastolic', 'mmHg', 30.0, 160.0),
        (8, 'bp_map', 'mmHg', 40.0, 200.0),
        (9, 'sleep_stage', 'stage_code', 0.0, 4.0),
        (10, 'resting_hr', 'bpm', 20.0, 250.0),
        (11, 'respiratory_rate', 'breaths/min', 1.0, 100.0),
        (12, 'wrist_temperature', 'degC', -20.0, 60.0),
        (13, 'step_count', 'count', 0.0, 1000000.0),
        (14, 'active_energy', 'kcal', 0.0, 100000.0),
        (15, 'exercise_time', 'min', 0.0, 1440.0),
        (16, 'environmental_audio', 'dBASPL', 0.0, 200.0),
        (17, 'blood_glucose', 'mg/dL', 1.0, 1000.0),
        (18, 'vo2_max', 'mL/kg*min', 1.0, 150.0),
        (19, 'walking_hr', 'bpm', 20.0, 250.0),
        (20, 'headphone_audio', 'dBASPL', 0.0, 200.0),
        (21, 'walking_steadiness', 'percent', 0.0, 100.0),
        (22, 'afib_burden', 'percent', 0.0, 100.0),
        (23, 'walking_speed', 'm/s', 0.0, 20.0),
        (24, 'walking_step_length', 'm', 0.0, 5.0),
        (25, 'walking_double_support', 'percent', 0.0, 100.0),
        (26, 'walking_asymmetry', 'percent', 0.0, 100.0),
        (27, 'time_in_daylight', 'min', 0.0, 1440.0),
        (28, 'physical_effort', 'kcal/kg*hr', 0.0, 100.0),
        (29, 'body_mass', 'kg', 0.1, 500.0),
        (30, 'patient_flow', 'L/s', -20.0, 20.0),
        (31, 'mask_pressure', 'cmH2O', 0.0, 50.0),
        (32, 'mask_pressure_2s', 'cmH2O', 0.0, 50.0),
        (33, 'inspiratory_pressure', 'cmH2O', 0.0, 50.0),
        (34, 'expiratory_pressure', 'cmH2O', 0.0, 50.0),
        (35, 'leak', 'L/min', 0.0, 300.0),
        (36, 'tidal_volume', 'mL', 0.0, 5000.0),
        (37, 'minute_ventilation', 'L/min', 0.0, 60.0),
        (38, 'target_minute_ventilation', 'L/min', 0.0, 60.0),
        (39, 'ie_ratio', 'ratio', 0.0, 10.0),
        (40, 'snore_index', 'index', 0.0, 100.0),
        (41, 'flow_limitation', 'ratio', 0.0, 1.0),
        (42, 'inspiratory_duration', 'seconds', 0.0, 10.0),
        (43, 'trigger_cycle_event', 'code', 0.0, 255.0)
        ) AS declared(id, name, unit, range_lo, range_hi)
          ON declared.id = stored.id
        WHERE stored.name <> declared.name
           OR stored.unit <> declared.unit
           OR stored.range_lo <> declared.range_lo
           OR stored.range_hi <> declared.range_hi
    LOOP
        SELECT count(*) INTO row_count
        FROM samples WHERE type = conflict.id;
        IF row_count > 0 THEN
            RAISE EXCEPTION
                'type id % holds % sample rows as ''%'' in % but registry.toml now declares ''%'' in %: type ids, units and ranges are permanent. Overwriting them reinterprets every stored value.',
                conflict.id, row_count, conflict.stored_name, conflict.stored_unit,
                conflict.declared_name, conflict.declared_unit;
        END IF;
    END LOOP;
END $$;

-- Ids are permanent. This is the guard, not the comment above it.
DO $$
DECLARE
    conflict RECORD;
    row_count BIGINT;
BEGIN
    IF to_regclass('samples') IS NULL THEN
        RETURN;  -- fresh database: nothing to protect yet
    END IF;
    FOR conflict IN
        SELECT stored.id, stored.name AS stored_name, declared.name AS declared_name
        FROM vitals_source AS stored
        JOIN (VALUES
        (0, 'emay'),
        (1, 'polar_h10'),
        (2, 'healthkit'),
        (3, 'derived'),
        (4, 'wellue_ks60fwb'),
        (5, 'resmed_air11'),
        (6, 'ad_ua651ble'),
        (7, 'polar_h10_emulator'),
        (8, 'resmed_air11_sd'),
        (9, 'acconeer_xe125')
        ) AS declared(id, name) ON declared.id = stored.id
        WHERE stored.name <> declared.name
    LOOP
        SELECT count(*) INTO row_count
        FROM samples WHERE source = conflict.id;
        IF row_count > 0 THEN
            RAISE EXCEPTION
                'source id % holds % sample rows as ''%'' but registry.toml now declares ''%'': ids are permanent. Retire the old name (retired = true, same id) and give the new device the next free id.',
                conflict.id, row_count, conflict.stored_name, conflict.declared_name;
        END IF;
    END LOOP;
END $$;

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
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (30, 'patient_flow', 'L/s', -20.0, 20.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (31, 'mask_pressure', 'cmH2O', 0.0, 50.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (32, 'mask_pressure_2s', 'cmH2O', 0.0, 50.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (33, 'inspiratory_pressure', 'cmH2O', 0.0, 50.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (34, 'expiratory_pressure', 'cmH2O', 0.0, 50.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (35, 'leak', 'L/min', 0.0, 300.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (36, 'tidal_volume', 'mL', 0.0, 5000.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (37, 'minute_ventilation', 'L/min', 0.0, 60.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (38, 'target_minute_ventilation', 'L/min', 0.0, 60.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (39, 'ie_ratio', 'ratio', 0.0, 10.0, 0.01)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (40, 'snore_index', 'index', 0.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (41, 'flow_limitation', 'ratio', 0.0, 1.0, 0.01)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (42, 'inspiratory_duration', 'seconds', 0.0, 10.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (43, 'trigger_cycle_event', 'code', 0.0, 255.0, 1.0)
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
VALUES (4, 'wellue_ks60fwb', 'ble')
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
INSERT INTO vitals_source (id, name, transport)
VALUES (8, 'resmed_air11_sd', 'sd_edf')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (9, 'acconeer_xe125', 'usb_serial')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
