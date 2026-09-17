-- Verify vitals:mobile_status_beacons
SELECT client_id, beacon_id, body_sha256, asserted_at, changed_at, type_facts
  FROM mobile_status_beacons WHERE FALSE;
SELECT 1
 WHERE EXISTS (
   SELECT 1 FROM pg_constraint
   WHERE conrelid = 'mobile_status_beacons'::regclass
     AND conname = 'mobile_status_beacons_changed_at_check'
 );
