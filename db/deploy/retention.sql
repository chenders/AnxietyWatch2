-- Deploy vitals:retention to pg
-- Raw samples age out at 2 years; samples_1min is kept indefinitely, so history
-- survives at minute resolution with min/max intact.
--
-- Depends on samples_1min: dropping raw data before the rollup exists would
-- lose it permanently. That dependency is why sqitch orders by graph rather
-- than by a linear counter.
SELECT add_retention_policy('samples', INTERVAL '2 years', if_not_exists => TRUE);
