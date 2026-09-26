-- Verify vitals:telemetry
SELECT enabled, region, key_nonce, key_ciphertext, key_last_four,
       last_test_succeeded, last_test_at, dropped_count
  FROM telemetry_config WHERE FALSE;
