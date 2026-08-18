-- Verify vitals:noise_analysis_helpers on pg

BEGIN;

SELECT event_id, icao_hex, observed_at, points_in_window, closest_km,
       seconds_within_4km, seconds_within_8km
  FROM noise_event_audibility WHERE false;

SELECT icao_hex, gap_start, gap_end, gap_seconds
  FROM aircraft_track_gaps WHERE false;

ROLLBACK;
