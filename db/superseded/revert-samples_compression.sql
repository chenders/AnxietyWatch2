-- Revert vitals:samples_compression from pg
SELECT remove_compression_policy('samples', if_exists => TRUE);
SELECT decompress_chunk(c, if_compressed => TRUE)
  FROM show_chunks('samples') c;
ALTER TABLE samples SET (timescaledb.compress = FALSE);
