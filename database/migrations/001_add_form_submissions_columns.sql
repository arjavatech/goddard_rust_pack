-- Migration: Add missing columns to form_submissions table
-- Date: 2026-02-14
-- Description: Add status, revision tracking, and link columns to support UPSERT logic

-- Add status column with default 'pending'
ALTER TABLE form_submissions
ADD COLUMN IF NOT EXISTS status VARCHAR(50) DEFAULT 'pending';

-- Add revision tracking columns
ALTER TABLE form_submissions
ADD COLUMN IF NOT EXISTS revision_number INTEGER DEFAULT 1;

ALTER TABLE form_submissions
ADD COLUMN IF NOT EXISTS revision_reason VARCHAR(500);

-- Add Fillout link columns
ALTER TABLE form_submissions
ADD COLUMN IF NOT EXISTS edit_link TEXT;

ALTER TABLE form_submissions
ADD COLUMN IF NOT EXISTS pdf_link TEXT;

-- Add check constraint for status
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'check_form_submission_status'
    ) THEN
        ALTER TABLE form_submissions
        ADD CONSTRAINT check_form_submission_status
        CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'requires_review', 'approved', 'rejected'));
    END IF;
END $$;

-- Add comment for documentation
COMMENT ON COLUMN form_submissions.status IS 'Form submission processing status';
COMMENT ON COLUMN form_submissions.revision_number IS 'Revision number for tracking webhook updates';
COMMENT ON COLUMN form_submissions.revision_reason IS 'Reason for the revision (e.g., Webhook update)';
COMMENT ON COLUMN form_submissions.edit_link IS 'Fillout.com edit link for the submission';
COMMENT ON COLUMN form_submissions.pdf_link IS 'Fillout.com PDF link for the submission';
