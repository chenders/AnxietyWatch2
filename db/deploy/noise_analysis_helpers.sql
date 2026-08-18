-- Deploy vitals:noise_analysis_helpers to pg
--
-- Read-only analysis views over the noise archive. Both adapt SQL patterns
-- proven in the sibling phx-helicopter-tracker project (see
-- docs/reviews/phx-helicopter-tracker-full-audit-deltas.md):
--
--   noise_event_audibility  "how long was the confirmed aircraft near the
--                           house around each event" -- the LAG-window
--                           time-in-radius query. An interval counts only
--                           when BOTH endpoints are inside the radius, so a
--                           track gap across the circle never fabricates
--                           dwell time. Gaps longer than 60 s contribute
--                           nothing even when both endpoints are inside,
--                           because asserting presence across an unobserved
--                           minute would be invention, not measurement.
--   aircraft_track_gaps     archive QA: per-airframe tracking gaps within a
--                           pass window, the input for deciding whether an
--                           FR24 definitive-track fetch is worth its credits.

BEGIN;

CREATE VIEW noise_event_audibility AS
WITH confirmed AS (
    -- Latest verdict per event only; superseded verdicts must not resurrect.
    SELECT DISTINCT ON (na.event_id)
           na.event_id, na.confirmed_hex, na.verdict, ev.observed_at
    FROM noise_attribution na
    JOIN noise_event ev USING (event_id)
    ORDER BY na.event_id, na.generated_at DESC
),
points AS (
    SELECT c.event_id, c.confirmed_hex, c.observed_at,
           tp.at,
           tp.distance_km,
           LAG(tp.at) OVER w            AS prev_at,
           LAG(tp.distance_km) OVER w   AS prev_distance_km
    FROM confirmed c
    JOIN aircraft_track_point tp
      ON tp.icao_hex = c.confirmed_hex
     AND tp.at BETWEEN c.observed_at - interval '10 minutes'
                   AND c.observed_at + interval '10 minutes'
    WHERE c.verdict = 'CONFIRMED' AND c.confirmed_hex IS NOT NULL
    WINDOW w AS (PARTITION BY c.event_id ORDER BY tp.at)
)
SELECT event_id,
       confirmed_hex AS icao_hex,
       observed_at,
       count(*)                                   AS points_in_window,
       min(distance_km)                           AS closest_km,
       -- Seconds inside 4 km (the model's NEAR band): both endpoints inside,
       -- interval <= 60 s. NULL distance never counts as near.
       COALESCE(sum(
           CASE WHEN distance_km <= 4.0 AND prev_distance_km <= 4.0
                 AND at - prev_at <= interval '60 seconds'
                THEN EXTRACT(EPOCH FROM (at - prev_at)) ELSE 0 END), 0)
                                                  AS seconds_within_4km,
       COALESCE(sum(
           CASE WHEN distance_km <= 8.0 AND prev_distance_km <= 8.0
                 AND at - prev_at <= interval '60 seconds'
                THEN EXTRACT(EPOCH FROM (at - prev_at)) ELSE 0 END), 0)
                                                  AS seconds_within_8km
FROM points
GROUP BY event_id, confirmed_hex, observed_at;

COMMENT ON VIEW noise_event_audibility IS
    'Dwell time of the confirmed aircraft near the station around each event, '
    'from the local track archive. Both-endpoints-inside LAG intervals only; '
    'track gaps contribute nothing rather than fabricated presence.';

CREATE VIEW aircraft_track_gaps AS
WITH deltas AS (
    SELECT icao_hex, at,
           LAG(at) OVER (PARTITION BY icao_hex ORDER BY at) AS prev_at
    FROM aircraft_track_point
    WHERE icao_hex IS NOT NULL
)
SELECT icao_hex,
       prev_at   AS gap_start,
       at        AS gap_end,
       EXTRACT(EPOCH FROM (at - prev_at)) AS gap_seconds
FROM deltas
WHERE prev_at IS NOT NULL
  -- Below 5 minutes is normal churn (aircraft leaving and re-entering the
  -- logged radius); above a day is two different visits, not a gap.
  AND at - prev_at BETWEEN interval '5 minutes' AND interval '1 day';

COMMENT ON VIEW aircraft_track_gaps IS
    'Tracking gaps per airframe in the local archive. Large gaps around a '
    'confirmed event are the argument for spending 40 FR24 credits on the '
    'definitive track.';

COMMIT;
