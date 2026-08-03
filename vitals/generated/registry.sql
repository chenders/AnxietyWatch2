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


-- Vitals sources lookup table
CREATE TABLE IF NOT EXISTS vitals_source (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    transport TEXT NOT NULL
);


-- Add foreign key constraints to samples table
CREATE TABLE IF NOT EXISTS samples (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    type INTEGER NOT NULL,
    source INTEGER NOT NULL,
    value REAL NOT NULL,
    FOREIGN KEY(type) REFERENCES vitals_type(id),
    FOREIGN KEY(source) REFERENCES vitals_source(id),
    CHECK(type IN (SELECT id FROM vitals_type)),
    CHECK(source IN (SELECT id FROM vitals_source))
);
