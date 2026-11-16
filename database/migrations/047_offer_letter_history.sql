-- Allow multiple offer letters per employee (history tracking)
-- Drop the unique constraint that prevented multiple records per employee
ALTER TABLE offer_letters DROP CONSTRAINT IF EXISTS unique_offer_per_employee;

-- Add is_current flag; existing rows are all current
ALTER TABLE offer_letters ADD COLUMN IF NOT EXISTS is_current BOOLEAN NOT NULL DEFAULT TRUE;

-- Index for efficient current-offer lookups
CREATE INDEX IF NOT EXISTS idx_offer_letters_employee_current
    ON offer_letters (employee_id, school_id, is_current);
