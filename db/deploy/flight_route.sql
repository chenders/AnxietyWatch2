-- Deploy vitals:flight_route to pg
--
-- Where a flight came from and where it is going, keyed on callsign.
--
-- Why a separate table, keyed on something `aircraft` deliberately refuses
-- ----------------------------------------------------------------------
-- `aircraft` is keyed on the Mode S address precisely because a callsign is not
-- identity: it is pilot-entered, frequently blank, and rotatable. All of that
-- remains true. But a callsign IS a route key -- ASA783 flies Atlanta to
-- Portland whichever airframe operates it on a given day -- and that is exactly
-- the fact this table records. The two keys answer different questions, so they
-- get different tables rather than one table with a confused primary key.
--
-- This is reference data, not observation. A route does not belong to a pass;
-- it belongs to a flight number, and it is as true tomorrow as it was today.
-- Hence no timestamp on the row beyond when it was fetched, and no retention
-- deadline: unlike Flightradar24, adsbdb attaches no expiry to what it serves.
--
-- Why most callsigns will never appear here
-- -----------------------------------------
-- Only airline-style callsigns have routes. A private aircraft transmits its
-- registration as its callsign -- N425EJ, the Portland Police Cessna -- and
-- adsbdb answers 404 for those, correctly. The enricher filters to callsigns
-- shaped like an ICAO airline callsign before spending a request, and records
-- the negative for the rest so a free service is not asked twice.

BEGIN;

CREATE TABLE flight_route (
    callsign                 text PRIMARY KEY,

    -- Nullable throughout. A 404 records the attempt with every field null,
    -- which is how "we asked and there is no route" is distinguished from
    -- "we have not asked yet" (no row at all). Unknown is not OK; it is a
    -- state, and it needs somewhere to live.
    airline_name             text,
    airline_icao             text,
    airline_iata             text,

    origin_iata              text,
    origin_icao              text,
    origin_name              text,
    origin_municipality      text,

    destination_iata         text,
    destination_icao         text,
    destination_name         text,
    destination_municipality text,

    -- When we last asked. Never null: a row exists only because a request was
    -- made, and a row that cannot say when it was fetched cannot be retried
    -- sensibly.
    looked_up_at             timestamptz NOT NULL DEFAULT now(),
    updated_at               timestamptz NOT NULL DEFAULT now(),

    -- A callsign is uppercase by convention and by ICAO. Storing it any other
    -- way would let ASA783 and asa783 become two routes.
    CONSTRAINT flight_route_callsign_upper CHECK (callsign = upper(callsign)),
    CONSTRAINT flight_route_callsign_shape CHECK (callsign <> '')
);

COMMENT ON TABLE flight_route IS
    'Origin and destination per airline callsign, from adsbdb. Reference data, '
    'not observation: keyed on callsign because a route belongs to a flight '
    'number, not to an airframe.';

COMMENT ON COLUMN flight_route.looked_up_at IS
    'When adsbdb was last asked. A row with all-null route fields means we '
    'asked and there is no route, which is not the same as never having asked.';

-- The enricher''s queue: callsigns not looked up recently. Partial on the
-- retry horizon would need a stable NOW(), so this stays a plain index.
CREATE INDEX flight_route_stale_idx ON flight_route (looked_up_at);

COMMIT;
