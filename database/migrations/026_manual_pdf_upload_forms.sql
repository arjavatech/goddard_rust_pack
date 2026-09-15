-- Add support for manual PDF uploads to form assignments
-- Enables admins to upload completed physical forms as PDFs directly

-- Add columns to student_form_assignments for manual PDF uploads
ALTER TABLE student_form_assignments
    ADD COLUMN IF NOT EXISTS submission_source VARCHAR(50) NOT NULL DEFAULT 'digital'
        CHECK (submission_source IN ('digital', 'manual_upload')),
    ADD COLUMN IF NOT EXISTS manual_pdf_storage_key TEXT,
    ADD COLUMN IF NOT EXISTS manual_pdf_file_name TEXT,
    ADD COLUMN IF NOT EXISTS manual_pdf_content_type TEXT,
    ADD COLUMN IF NOT EXISTS manual_pdf_file_size_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS manual_pdf_uploaded_at TIMESTAMP,
    ADD COLUMN IF NOT EXISTS manual_pdf_uploaded_by VARCHAR(255);

-- Add 'manually_uploaded' to student_form_assignments status check constraint
ALTER TABLE student_form_assignments DROP CONSTRAINT IF EXISTS check_status;
ALTER TABLE student_form_assignments ADD CONSTRAINT check_status
    CHECK (status IN ('incomplete', 'in_progress', 'completed', 'manually_uploaded', 'archived', 'approved', 'rejected'));

-- Add columns to employee_form_assignments for manual PDF uploads
ALTER TABLE employee_form_assignments
    ADD COLUMN IF NOT EXISTS submission_source VARCHAR(50) NOT NULL DEFAULT 'digital'
        CHECK (submission_source IN ('digital', 'manual_upload')),
    ADD COLUMN IF NOT EXISTS manual_pdf_storage_key TEXT,
    ADD COLUMN IF NOT EXISTS manual_pdf_file_name TEXT,
    ADD COLUMN IF NOT EXISTS manual_pdf_content_type TEXT,
    ADD COLUMN IF NOT EXISTS manual_pdf_file_size_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS manual_pdf_uploaded_at TIMESTAMP,
    ADD COLUMN IF NOT EXISTS manual_pdf_uploaded_by VARCHAR(255);

-- Add check constraint for employee_form_assignments status if it doesn't exist
ALTER TABLE employee_form_assignments ADD CONSTRAINT check_employee_form_status
    CHECK (status IN ('incomplete', 'in_progress', 'completed', 'manually_uploaded', 'approved', 'rejected'));

-- Create indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_sfa_submission_source ON student_form_assignments(submission_source);
CREATE INDEX IF NOT EXISTS idx_sfa_manual_pdf_storage_key ON student_form_assignments(manual_pdf_storage_key);

CREATE INDEX IF NOT EXISTS idx_efa_submission_source ON employee_form_assignments(submission_source);
CREATE INDEX IF NOT EXISTS idx_efa_manual_pdf_storage_key ON employee_form_assignments(manual_pdf_storage_key);
