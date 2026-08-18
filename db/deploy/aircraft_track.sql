-- Deploy vitals:aircraft_track to pg
--
-- Continuous track points for every aircraft inside the watch radius, from
-- both feeds, so a flight path can be REPLAYED from our own record: where the
-- aircraft was, second by second, when a noise event happened -- without
-- depending on FR24 still having the flight in its playback window.
--
-- Why now: the pass log keeps one row per pass (closest approach only) and
-- the event evidence keeps snapshots at press time. Neither can draw the
-- path. Position data is the canonical example of "difficult to retroactively
-- get": once the aircraft lands, no poll will sell the track back precisely.
--
-- Volume, measured against the owner's explicit direction to prefer capture
-- over thrift: ~10 visible aircraft at one point per 5 s (1 s inside 4 km) is
-- on the order of 2x10^5 rows/day, ~100 bytes each -- roughly 1 GB per two
-- months before TimescaleDB columnar compression, which this table gets after
-- 7 days. Not a disk-full hazard on this Pi within two months; the drain
-- also names remaining disk space every run so growth is watched, not
-- discovered.
--
-- Retention matches the rest of the archive: FR24-derived points carry the
-- owner's 182-day retain_until (owner's documented decision; vendor terms say
-- 30 -- see panel_noise_log.py); receiver points are ours and keep forever.

BEGIN;

CREATE TABLE aircraft_track_point (
    at                timestamptz NOT NULL,
    provenance        text NOT NULL CHECK (provenance IN ('adsb', 'fr24')),
    icao_hex          text CHECK (icao_hex IS NULL OR icao_hex ~ '^[0-9A-F]{6}$'),
    fr24_id           text,
    callsign          text,
    registration      text,
    lat               double precision NOT NULL CHECK (lat BETWEEN -90 AND 90),
    lon               double precision NOT NULL CHECK (lon BETWEEN -180 AND 180),
    alt_ft            integer,
    speed_kt          integer,
    track_deg         double precision,
    vertical_rate_fpm integer,
    rssi_dbm          double precision,
    distance_km       double precision CHECK (distance_km IS NULL OR distance_km >= 0),
    bearing_deg       double precision,
    retain_until      timestamptz,
    CHECK ((provenance = 'fr24') = (retain_until IS NOT NULL))
);

COMMENT ON TABLE aircraft_track_point IS
    'Raw positions from both feeds inside the watch radius, dense enough to '
    'replay a flight path over the house without vendor playback.';

SELECT create_hypertable('aircraft_track_point', 'at', chunk_time_interval => interval '1 day');

-- Idempotent ingest under drain replay: one point per instant per aircraft
-- per feed. COALESCE because FR24 sometimes withholds hex and the receiver
-- never has an fr24_id.
CREATE UNIQUE INDEX aircraft_track_point_identity_idx ON aircraft_track_point
    (at, provenance, COALESCE(icao_hex, ''), COALESCE(fr24_id, ''));

CREATE INDEX aircraft_track_point_hex_idx ON aircraft_track_point (icao_hex, at DESC)
    WHERE icao_hex IS NOT NULL;
CREATE INDEX aircraft_track_point_retention_idx ON aircraft_track_point (retain_until)
    WHERE retain_until IS NOT NULL;

ALTER TABLE aircraft_track_point SET (
    timescaledb.compress,
    timescaledb.compress_orderby = 'at',
    timescaledb.compress_segmentby = 'icao_hex'
);
SELECT add_compression_policy('aircraft_track_point', compress_after => interval '7 days');

COMMIT;
