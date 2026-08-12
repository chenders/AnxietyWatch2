-- Deploy vitals:aircraft_enrichment to pg
--
-- Records that an address was looked up against the public airframe database,
-- so a NEGATIVE answer is remembered.
--
-- Without this, an address the lookup service has never heard of is re-queried
-- on every run, forever, against a free service that owes us nothing. Coverage
-- is genuinely incomplete -- a38c66 returned no data during evaluation -- so
-- "we asked and there is nothing" has to be a storable answer distinct from
-- "we have not asked".
--
-- It is deliberately NOT a boolean. A timestamp additionally allows a retry
-- after long enough that the database may have gained the airframe, which a
-- flag could not express.

BEGIN;

ALTER TABLE aircraft ADD COLUMN looked_up_at timestamptz;

COMMENT ON COLUMN aircraft.looked_up_at IS
    'When this address was last queried against the public airframe database. '
    'NULL means never asked. Set even when nothing was found, so a miss is not '
    're-queried forever.';

-- Drives the "needs a lookup" scan; partial so it holds only the pending ones.
CREATE INDEX aircraft_pending_lookup_idx ON aircraft (looked_up_at)
    WHERE icao_type IS NULL AND source <> 'roster';

COMMIT;
