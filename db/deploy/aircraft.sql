-- Deploy vitals:aircraft to pg
--
-- The durable record of what flies over the station, and who owns it.
--
-- Two tables, deliberately separate:
--
--   aircraft       one row per AIRFRAME. Slow-changing identity: registration,
--                  ICAO type, operator. Keyed on the Mode S address because that
--                  is the only identifier an aircraft always broadcasts.
--   aircraft_pass  one row per PASS. Time-series, so a hypertable.
--
-- Why the Mode S address is the key
-- ---------------------------------
-- Roughly half the aircraft this receiver hears broadcast no emitter category at
-- all, and the air ambulances that generate the complaints here are
-- over-represented in that group. Flightradar24 additionally withholds `reg`,
-- `hex` and `callsign` from its summary endpoints for the Portland Life Flight
-- tails. The 24-bit ICAO address is what survives all of that: it is burned into
-- the transponder and broadcast whether or not anything else is.
--
-- Callsigns are NOT identity. They are pilot-entered, frequently blank, and
-- rotatable, which is why `aircraft` is not keyed on one.
--
-- Why a sold aircraft must be representable
-- -----------------------------------------
-- N739MR was surplused by Portland Police in 2023 and sold to a private owner;
-- it still flies locally under hex A9ED9D and was the single most frequent
-- registration in a 14-day sample. A roster that cannot express "this address is
-- no longer that operator" will attribute every one of those flights to the
-- police. Hence `retired` and `retired_reason`, which are checked BEFORE any
-- operator attribution.
--
-- Why deletion is allowed here, unlike samples
-- -------------------------------------------
-- `samples` is append-only and removal is a tombstone row. This table is
-- different: Flightradar24's terms require that data obtained from their API be
-- deleted within a bounded window, so rows carrying FR24 provenance MUST be
-- destroyable. `retain_until` records the deadline per row rather than leaving
-- it to be re-derived by whatever runs the cleanup. Rows from the local receiver
-- carry NULL and are kept forever: that data came off an antenna on this roof
-- and no vendor licence governs it.
--
-- Why noise_index is nullable and must never be defaulted
-- ------------------------------------------------------
-- NULL means the model could not score the aircraft — no emitter category, or no
-- vertical rate. An unscored aircraft that passed overhead is the single most
-- important row this table can hold. Writing 0 there would sort it with the
-- quiet ones and lose exactly the case the whole system exists to catch. The
-- CHECK below enforces that a NULL index is always paired with band 'UNKNOWN',
-- so the two can never disagree.

BEGIN;

CREATE TABLE aircraft (
    icao_hex        text PRIMARY KEY
                    CHECK (icao_hex ~ '^[0-9A-F]{6}$'),
    registration    text,
    icao_type       text,
    model           text,
    operator_key    text,
    operator_name   text,
    operator_kind   text CHECK (operator_kind IN ('AIR_AMBULANCE', 'POLICE')),
    -- A sold or reassigned airframe. Checked before attribution, never after.
    retired         boolean NOT NULL DEFAULT false,
    retired_reason  text,
    -- Where this identity came from, so a hand-curated row is distinguishable
    -- from one a lookup service supplied and may later contradict.
    source          text NOT NULL
                    CHECK (source IN ('roster', 'adsbdb', 'fr24', 'manual')),
    first_seen      timestamptz,
    last_seen       timestamptz,
    updated_at      timestamptz NOT NULL DEFAULT now(),
    -- A retired airframe must say why. A bare flag invites someone to clear it.
    CHECK (NOT retired OR retired_reason IS NOT NULL)
);

COMMENT ON TABLE aircraft IS
    'One row per airframe, keyed on the Mode S address — the only identifier an '
    'aircraft always broadcasts. Callsigns are pilot-entered and are not identity.';

CREATE INDEX aircraft_registration_idx ON aircraft (registration)
    WHERE registration IS NOT NULL;
CREATE INDEX aircraft_operator_idx ON aircraft (operator_key)
    WHERE operator_key IS NOT NULL;

CREATE TABLE aircraft_pass (
    at              timestamptz NOT NULL,
    provenance      text NOT NULL CHECK (provenance IN ('adsb', 'fr24')),
    icao_hex        text CHECK (icao_hex IS NULL OR icao_hex ~ '^[0-9A-F]{6}$'),
    callsign        text,
    registration    text,
    icao_type       text,
    fr24_id         text,
    distance_km     double precision CHECK (distance_km >= 0),
    altitude_ft     integer,
    phase           text CHECK (phase IN ('CLIMB', 'LEVEL', 'DESCENT')),
    -- NULL means UNSCORED. Never 0. See the header.
    noise_index     double precision,
    loudness_band   text NOT NULL
                    CHECK (loudness_band IN
                           ('VERY LOUD', 'LOUD', 'AUDIBLE', 'FAINT', 'UNKNOWN')),
    -- Whether this is one the owner probably cared about. A FLAG, not a filter:
    -- every pass inside the watch radius is recorded, so a later question about
    -- a quiet hour is still answerable.
    notable         boolean,
    operator_state  text CHECK (operator_state IN ('CONFIRMED', 'UNCONFIRMED')),
    reason          text,
    -- NULL = keep forever (receiver data). Non-NULL = destroy by then (FR24).
    retain_until    timestamptz,
    -- An unscored aircraft must read as unscored, not as quiet.
    CHECK ((noise_index IS NULL) = (loudness_band = 'UNKNOWN')),
    -- FR24 rows must always carry a deadline; receiver rows must never.
    CHECK ((provenance = 'fr24') = (retain_until IS NOT NULL))
);

COMMENT ON TABLE aircraft_pass IS
    'One row per pass, not per observation. A 1 Hz publisher would otherwise '
    'write hundreds of rows for a single overflight.';

SELECT create_hypertable('aircraft_pass', 'at', chunk_time_interval => interval '7 days');

-- Idempotent ingest. The drain replays from a byte offset and may re-present a
-- row after a crash, so the same pass must not land twice. COALESCE because
-- neither hex nor callsign is reliably present — NULLs would defeat a plain
-- unique constraint, which is how duplicates get in. The partition column is
-- included because TimescaleDB requires it in any unique index.
CREATE UNIQUE INDEX aircraft_pass_identity_idx ON aircraft_pass
    (at, provenance, COALESCE(icao_hex, ''), COALESCE(callsign, ''));

CREATE INDEX aircraft_pass_hex_idx ON aircraft_pass (icao_hex, at DESC)
    WHERE icao_hex IS NOT NULL;
CREATE INDEX aircraft_pass_notable_idx ON aircraft_pass (at DESC)
    WHERE notable;
-- Drives the retention sweep; partial so it stays small.
CREATE INDEX aircraft_pass_retention_idx ON aircraft_pass (retain_until)
    WHERE retain_until IS NOT NULL;

COMMIT;
