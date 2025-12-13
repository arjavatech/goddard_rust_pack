-- Migration: Update student_form_assignments status constraint to allow 'approved' and 'rejected'
-- Date: 2025-12-13
-- Purpose: Enable form review status tracking in the status column

-- Drop existing constraint
ALTER TABLE student_form_assignments DROP CONSTRAINT IF EXISTS check_status;

-- Add updated constraint with approved and rejected values
ALTER TABLE student_form_assignments ADD CONSTRAINT check_status
CHECK (status IN ('incomplete', 'in_progress', 'completed', 'archived', 'approved', 'rejected'));

-- Add comment for clarity
COMMENT ON CONSTRAINT check_status ON student_form_assignments IS
'Allowed status values: incomplete, in_progress, completed, archived, approved, rejected';
