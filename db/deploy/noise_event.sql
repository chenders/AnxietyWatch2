-- Deploy vitals:noise_event to pg
--
-- The durable, queryable record of noise EVENTS: who reported one (the
-- resident, the cat via the resident, later a microphone), every aircraft that
-- was physically a candidate at that moment, and how each attribution verdict
-- was reached. The JSONL spools under /var/lib/panel-noise remain the
-- write-ahead record; these tables are the archive the questions get asked of:
-- "what has flown over, which airframe caused which event, when, and how was
-- that determined."
--
-- Three tables and a view, deliberately separate:
--
--   noise_event            one row per REAL event. Synthetic plumbing tests
--                          never land here -- the drain refuses them by
--                          provenance, not by convention.
--   noise_event_candidate  every aircraft considered a possible cause of that
--                          event, with everything both feeds knew about it at
--                          the time. This is the row that makes a six-month
--                          "what else was up there" question answerable
--                          without replaying a live feed.
--   noise_attribution      one row per DISTINCT verdict per event. The method
--                          column says how the determination was made
--                          (per-event physical heuristic today; a microphone
--                          correlation later), so re-runs that reach the same
--                          verdict do not spam the history, and a verdict that
--                          CHANGES -- e.g. after a model fix -- is preserved
--                          beside its predecessor rather than overwriting it.
--   aircraft_noise_history one row per airframe that has ever been CONFIRMED
--                          as the cause of an event. A view, so the flag can
--                          never drift from the evidence it summarises.
--
-- Retention mirrors aircraft_pass: rows built from FR24 data carry the owner's
-- 182-day retain_until deadline (the owner's documented decision; the vendor
-- terms say 30 days -- see panel_noise_log.py before changing either way);
-- local-receiver rows and the events themselves carry NULL and are kept
-- forever. Event evidence is ours -- a button press and an operator's own
-- words are not licensed data.

BEGIN;

CREATE TABLE noise_event (
    event_id     text PRIMARY KEY,
    observed_at  timestamptz NOT NULL,
    recorded_at  timestamptz,
    -- Outcomes are explicit booleans, not a single label: a double press has
    -- always meant BOTH the resident heard it and the cat was frightened.
    human_heard  boolean NOT NULL,
    cat_scared   boolean NOT NULL,
    press_count  integer,
    -- How the evidence arrived. Synthetic is structurally unrepresentable.
    provenance   text NOT NULL
                 CHECK (provenance IN ('hue_button', 'operator_report')),
    note         text,
    -- The full captured context: complete ADS-B and FR24 snapshots, feed
    -- freshness, recent completed passes. Everything the label row held.
    evidence     jsonb,
    -- Acoustic weather at event time (METAR projection: temperature, wind,
    -- gusts, cloud, pressure, raw report). NULL = not measured, never zero.
    weather      jsonb
);

COMMENT ON TABLE noise_event IS
    'One row per real noise event. Synthetic rows are refused by the drain; '
    'a double press records BOTH outcomes, with the cat top priority.';

CREATE INDEX noise_event_observed_idx ON noise_event (observed_at DESC);

-- An operator restating an existing bridge event ("I told you within a couple
-- of seconds") corroborates it; it must never become a second event.
CREATE TABLE noise_event_corroboration (
    event_id     text NOT NULL REFERENCES noise_event (event_id),
    reported_at  timestamptz NOT NULL,
    source       text,
    note         text,
    PRIMARY KEY (event_id, reported_at)
);

CREATE TABLE noise_event_candidate (
    event_id       text NOT NULL REFERENCES noise_event (event_id),
    identity       text NOT NULL,   -- Mode S hex when known, else best stable id
    icao_hex       text CHECK (icao_hex IS NULL OR icao_hex ~ '^[0-9A-F]{6}$'),
    callsign       text,
    registration   text,
    icao_type      text,
    airframe_class text CHECK (airframe_class IS NULL OR airframe_class IN
                               ('heavy', 'jet', 'turboprop', 'light', 'rotorcraft')),
    fr24_id        text,
    feeds          text[] NOT NULL,
    score          double precision,
    reasons        jsonb,
    distance_km    double precision CHECK (distance_km IS NULL OR distance_km >= 0),
    altitude_ft    integer,
    noise_index    double precision,
    phase          text CHECK (phase IS NULL OR phase IN ('CLIMB', 'LEVEL', 'DESCENT')),
    -- Everything else either feed reported, verbatim. The columns above are
    -- the queryable projection; this is the record.
    raw            jsonb,
    -- NULL = local data, keep forever. Non-NULL = FR24-derived, destroy by then.
    retain_until   timestamptz,
    PRIMARY KEY (event_id, identity)
);

COMMENT ON TABLE noise_event_candidate IS
    'Every aircraft considered a possible cause of an event, with all data '
    'both feeds carried at that moment. FR24-derived rows expire per the '
    'owner''s 182-day retention decision; local rows never do.';

CREATE INDEX noise_event_candidate_hex_idx ON noise_event_candidate (icao_hex)
    WHERE icao_hex IS NOT NULL;
CREATE INDEX noise_event_candidate_retention_idx ON noise_event_candidate (retain_until)
    WHERE retain_until IS NOT NULL;

CREATE TABLE noise_attribution (
    event_id        text NOT NULL REFERENCES noise_event (event_id),
    generated_at    timestamptz NOT NULL,
    verdict         text NOT NULL
                    CHECK (verdict IN ('CONFIRMED', 'AMBIGUOUS', 'UNATTRIBUTED')),
    verdict_reason  text NOT NULL,
    margin          double precision,
    candidate_count integer,
    -- The winning airframe, only when CONFIRMED. Ambiguity stays unassigned.
    confirmed_hex          text CHECK (confirmed_hex IS NULL OR confirmed_hex ~ '^[0-9A-F]{6}$'),
    confirmed_identity     text,
    confirmed_registration text,
    confirmed_type         text,
    flight_key             text,
    -- HOW this determination was made: 'per_event_physical_v2' today, a
    -- microphone-correlation method later. Never a bare 'confirmed'.
    method          text NOT NULL,
    PRIMARY KEY (event_id, generated_at),
    CHECK ((verdict = 'CONFIRMED') = (confirmed_identity IS NOT NULL))
);

COMMENT ON TABLE noise_attribution IS
    'Verdict history per event. The drain appends a row only when the verdict '
    'content changes, so the history reads as decisions, not as cron noise.';

CREATE INDEX noise_attribution_confirmed_idx ON noise_attribution (confirmed_hex, generated_at DESC)
    WHERE confirmed_hex IS NOT NULL;

-- The flag the maps and consoles ask for: has this airframe ever caused a
-- marked noise event, split by outcome, with timestamps. Latest verdict per
-- event only, so a superseded CONFIRMED does not keep flagging an airframe.
CREATE VIEW aircraft_noise_history AS
WITH latest AS (
    SELECT DISTINCT ON (event_id) event_id, verdict, confirmed_hex,
           confirmed_identity, confirmed_registration, confirmed_type
    FROM noise_attribution
    ORDER BY event_id, generated_at DESC
)
SELECT latest.confirmed_hex               AS icao_hex,
       latest.confirmed_identity          AS identity,
       max(latest.confirmed_registration) AS registration,
       max(latest.confirmed_type)         AS icao_type,
       count(*) FILTER (WHERE ev.human_heard) AS human_heard_count,
       count(*) FILTER (WHERE ev.cat_scared)  AS cat_scared_count,
       min(ev.observed_at)                AS first_heard_at,
       max(ev.observed_at)                AS last_heard_at,
       array_agg(ev.event_id ORDER BY ev.observed_at) AS event_ids
FROM latest
JOIN noise_event ev USING (event_id)
WHERE latest.verdict = 'CONFIRMED'
GROUP BY latest.confirmed_hex, latest.confirmed_identity;

COMMENT ON VIEW aircraft_noise_history IS
    'Airframes confirmed as the cause of at least one marked noise event. A '
    'view over the evidence, so the flag cannot drift from what it summarises.';

COMMIT;
