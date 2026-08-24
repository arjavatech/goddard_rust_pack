-- Prevent overlapping synchronous Tap-Time retries for the same school.
-- A stale lock is recovered by the application after five minutes.
ALTER TABLE tap_time_connections
    ADD COLUMN IF NOT EXISTS sync_started_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_tap_time_connections_sync_started_at
    ON tap_time_connections (sync_started_at)
    WHERE sync_started_at IS NOT NULL;
