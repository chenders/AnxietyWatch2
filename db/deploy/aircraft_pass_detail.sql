-- Deploy vitals:aircraft_pass_detail to pg
--
-- Record what the panel actually showed, so a pass can be reconstructed.
--
-- Until now `aircraft_pass` stored identity, one distance, one altitude, a
-- phase and a score. The live view carried far more -- bearing, speed, track,
-- vertical rate, signal strength, the emitter category, the airframe class the
-- model chose, and the whole closest-approach geometry the forecast was built
-- on -- and all of it was discarded at write time. So the log could say a
-- 124-dBA-scoring aircraft passed, and could not say from which direction, how
-- fast, how close it was predicted to come, or why it was classed as it was.
--
-- That gap has already cost real analysis. When the owner reported hearing an
-- aircraft the model had rated FAINT, reconstructing the case meant replaying
-- a live feed rather than reading the record.
--
-- WHY ambient_dba MATTERS MOST
-- ---------------------------
-- On 2026-08-13 the loudness model was recalibrated from an invented relative
-- scale to estimated outdoor dBA, and the thresholds moved from fixed
-- constants to offsets from an assumed ambient. Every row written before that
-- carries a `noise_index` on a scale that no longer exists, and nothing in the
-- table says so. Two numbers in one column meaning two different things is the
-- same defect as the emitter category once written into `icao_type`.
--
-- Recording the ambient the score was computed against makes each row
-- self-describing: NULL means the old invented scale, a number means dBA
-- relative to that ambient. Old rows are not back-filled, because inventing a
-- value for them would assert something we do not know.

BEGIN;

ALTER TABLE aircraft_pass
    -- Live geometry at the moment of the pass. Bearing is degrees true from
    -- the station; the station's own coordinates never appear here or in the
    -- served JSON, only relative geometry.
    ADD COLUMN bearing_deg        double precision,
    ADD COLUMN speed_kt           integer,
    ADD COLUMN track_deg          double precision,
    ADD COLUMN vertical_rate_fpm  integer,

    -- What the receiver heard, as opposed to what we concluded. Signal
    -- strength is a measure of path quality, never of range -- it is recorded
    -- so a weak-signal pass can be told apart from a distant one, not so it
    -- can be turned into a distance.
    ADD COLUMN rssi_dbm           double precision,

    -- The broadcast ADS-B emitter category, e.g. 'A1'. A WEIGHT bucket, not a
    -- designator, and deliberately a separate column from icao_type: writing
    -- one into the other conflated two vocabularies once already and had to be
    -- repaired across 11 rows.
    ADD COLUMN emitter_category   text,

    -- The class the loudness model actually used, and where the operator name
    -- came from. Without these, a wrong score cannot be attributed to a wrong
    -- classification versus a wrong source level.
    ADD COLUMN airframe_class     text,
    ADD COLUMN operator_source    text,

    -- The forecast the row was judged on: closest airborne approach, and how
    -- far ahead it was seen. distance_km above is where the aircraft WAS;
    -- these are where it was predicted to come.
    ADD COLUMN cpa_ground_m       double precision,
    ADD COLUMN cpa_slant_m        double precision,
    ADD COLUMN cpa_altitude_ft    integer,
    ADD COLUMN eta_seconds        integer,

    -- The ambient the score was computed against. NULL means a row from before
    -- the 2026-08-13 recalibration, whose noise_index is on the old invented
    -- relative scale and is NOT comparable with a later one.
    ADD COLUMN ambient_dba        double precision;

COMMENT ON COLUMN aircraft_pass.ambient_dba IS
    'Assumed outdoor ambient in dBA that noise_index was scored against. NULL '
    'means a pre-2026-08-13 row on the superseded relative scale; such rows '
    'must not be compared numerically with later ones.';

COMMENT ON COLUMN aircraft_pass.rssi_dbm IS
    'Received signal strength. Path quality, not range -- never derive a '
    'distance from this.';

COMMENT ON COLUMN aircraft_pass.emitter_category IS
    'Broadcast ADS-B emitter category (A0-A7). A weight bucket, not an ICAO '
    'type designator; icao_type holds the designator.';

-- Constrain the classes to what the model can actually produce, so a typo in
-- the publisher shows up as a rejected row rather than as a silent new class
-- that never matches a query.
ALTER TABLE aircraft_pass
    ADD CONSTRAINT aircraft_pass_airframe_class_known
        CHECK (airframe_class IS NULL OR airframe_class IN
               ('heavy', 'jet', 'turboprop', 'light', 'rotorcraft')),
    ADD CONSTRAINT aircraft_pass_operator_source_known
        CHECK (operator_source IS NULL OR operator_source IN
               ('ROSTER', 'LOOKUP'));

COMMIT;
