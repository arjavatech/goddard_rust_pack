-- School-scoped request configuration and request completion tracking.
-- Configuration arrays are nullable so existing schools continue to work unchanged.

ALTER TABLE schools
    ADD COLUMN IF NOT EXISTS request_categories JSONB,
    ADD COLUMN IF NOT EXISTS location JSONB;

ALTER TABLE requests
    ADD COLUMN IF NOT EXISTS location VARCHAR(255),
    ADD COLUMN IF NOT EXISTS expected_completion_date DATE;

CREATE INDEX IF NOT EXISTS idx_requests_in_progress_expected_completion
    ON requests (school_id, expected_completion_date)
    WHERE status = 'In Progress' AND expected_completion_date IS NOT NULL;
