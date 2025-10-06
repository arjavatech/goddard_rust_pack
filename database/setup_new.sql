-- ==========================================
-- MIGRATION: Remove unused approval fields from enrollments
-- Date: 2025-10-06
-- Description: Removing approval workflow fields that are no longer needed
-- ==========================================

-- Remove approval workflow columns from enrollments table
ALTER TABLE enrollments DROP COLUMN IF EXISTS progress;
ALTER TABLE enrollments DROP COLUMN IF EXISTS submitted_at;
ALTER TABLE enrollments DROP COLUMN IF EXISTS approved_at;
ALTER TABLE enrollments DROP COLUMN IF EXISTS approved_by;
ALTER TABLE enrollments DROP COLUMN IF EXISTS approval_notes;
ALTER TABLE enrollments DROP COLUMN IF EXISTS forms_locked_at;

-- Remove admin_approval_status column and its constraint
ALTER TABLE enrollments DROP CONSTRAINT IF EXISTS check_admin_approval_status;
ALTER TABLE enrollments DROP COLUMN IF EXISTS admin_approval_status;

-- Remove indexes for dropped columns
DROP INDEX IF EXISTS idx_enrollments_admin_approval_status;
DROP INDEX IF EXISTS idx_enrollments_approved_by;
DROP INDEX IF EXISTS idx_enrollments_approved_at;

-- Migration complete
COMMENT ON TABLE enrollments IS 'Child enrollment records linking students to classrooms (simplified without approval workflow)';
