-- Deploy vitals:aircraft_source_adsb to pg
--
-- Allow 'adsb' as an identity source.
--
-- The original CHECK listed roster/adsbdb/fr24/manual, written when only the
-- FR24 poller learned identities. When the drain was corrected to record the
-- row's REAL provenance instead of a hardcoded 'fr24', every receiver-learned
-- identity began violating this constraint, and the drain died on the first one.
--
-- The spool absorbed it exactly as designed -- 709 rows queued, nothing lost,
-- and the failure was visible in the journal rather than silent. That is the
-- argument for the spool in one sentence.
--
-- 'adsb' means the identity was learned from the local receiver: it heard a
-- Mode S address and, if the roster or a lookup knew the airframe, a
-- registration and type with it. It is distinct from 'adsbdb', which is the
-- public airframe database queried over the network.

BEGIN;

ALTER TABLE aircraft DROP CONSTRAINT aircraft_source_check;
ALTER TABLE aircraft ADD CONSTRAINT aircraft_source_check
    CHECK (source IN ('roster', 'adsbdb', 'fr24', 'adsb', 'manual'));

COMMIT;
