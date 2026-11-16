-- Prevent duplicate Fillout webhook deliveries from double-processing the same submission
CREATE UNIQUE INDEX IF NOT EXISTS idx_offer_letters_fillout_submission_unique
  ON offer_letters (fillout_submission_id)
  WHERE fillout_submission_id IS NOT NULL;

-- Store the school-local timestamp from Fillout's submitted_at field
ALTER TABLE offer_letters
  ADD COLUMN IF NOT EXISTS employee_submitted_at TIMESTAMP;
