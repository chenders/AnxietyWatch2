-- Verify vitals:aircraft_warning on pg
BEGIN;
SELECT warning_id, decided_at, fired, suppressed_by, icao_hex, loudness_band,
       prediction, matched_pass, raw
FROM aircraft_warning WHERE false;
SELECT warning_id, reviewed_at, verdict, note, provenance
FROM aircraft_warning_review WHERE false;
SELECT warning_id, review_verdict FROM aircraft_warning_with_latest_review WHERE false;
ROLLBACK;
