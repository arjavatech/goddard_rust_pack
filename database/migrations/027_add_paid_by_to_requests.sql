-- Migration 027: Track who recorded the payment
-- Adds columns to identify which user (Admin or SuperAdmin) recorded the payment

ALTER TABLE requests
    ADD COLUMN IF NOT EXISTS paid_by_user_id UUID,
    ADD COLUMN IF NOT EXISTS paid_by_name VARCHAR(255);

CREATE INDEX IF NOT EXISTS idx_requests_paid_by
    ON requests(paid_by_user_id) WHERE paid_by_user_id IS NOT NULL;
