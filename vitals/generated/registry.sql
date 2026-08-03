/* GENERATED FROM registry.toml — DO NOT EDIT */

-- Vitals types lookup table
CREATE TABLE IF NOT EXISTS vitals_type (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    unit TEXT NOT NULL,
    range_lo REAL NOT NULL,
    range_hi REAL NOT NULL,
    precision REAL NOT NULL
);

INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (1, 'hr', 'bpm', 20.0, 250.0, 1.0);
INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (2, 'spo2', 'percent', 50.0, 100.0, 0.1);
INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (3, 'accel', 'g', -16.0, 16.0, 0.001);
INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (4, 'hrv_rmssd', 'ms', 1.0, 500.0, 0.1);
INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (5, 'rr_interval', 'seconds', 0.2, 3.0, 0.001);
INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (6, 'bp_systolic', 'mmHg', 50.0, 260.0, 1.0);
INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (7, 'bp_diastolic', 'mmHg', 30.0, 160.0, 1.0);
INSERT OR REPLACE INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (8, 'bp_map', 'mmHg', 40.0, 200.0, 1.0);

-- Vitals sources lookup table
CREATE TABLE IF NOT EXISTS vitals_source (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    transport TEXT NOT NULL
);

INSERT OR REPLACE INTO vitals_source (id, name, transport)
VALUES (0, 'emay', 'ble');
INSERT OR REPLACE INTO vitals_source (id, name, transport)
VALUES (1, 'polar_h10', 'ble');
INSERT OR REPLACE INTO vitals_source (id, name, transport)
VALUES (2, 'healthkit', 'push');
INSERT OR REPLACE INTO vitals_source (id, name, transport)
VALUES (3, 'derived', 'internal');
INSERT OR REPLACE INTO vitals_source (id, name, transport)
VALUES (6, 'ad_ua651ble', 'ble');

-- Add foreign key constraints to samples table
CREATE TABLE IF NOT EXISTS samples (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    type INTEGER NOT NULL,
    source INTEGER NOT NULL,
    value REAL NOT NULL,
    FOREIGN KEY(type) REFERENCES vitals_type(id),
    FOREIGN KEY(source) REFERENCES vitals_source(id)
);
