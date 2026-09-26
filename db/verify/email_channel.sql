-- Verify vitals:email_channel
SELECT enabled, from_address, destination_address, password_nonce,
       password_ciphertext, password_last_four, last_test_succeeded,
       last_test_at, last_test_detail
  FROM email_channel_config WHERE FALSE;
