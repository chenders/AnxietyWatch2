-- Verify vitals:apns_delivery
SELECT message_id, recipient_scope, target_snapshot, is_test
FROM apns_messages
LIMIT 1;
SELECT message_id, state, provider_status, provider_reason
FROM apns_delivery_attempts
LIMIT 1;
SELECT message_id, outcome, observed_at
FROM apns_client_outcomes
LIMIT 1;
