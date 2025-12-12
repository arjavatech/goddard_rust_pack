-- ==========================================
-- MIGRATION: Add due_date Column to form_templates Table
-- Date: 2025-12-12
-- Author: Goddard Backend Team
-- Description: Add optional due_date field with validation constraint
--              to track form completion deadlines
-- ==========================================

-- Step 1: Add the due_date column (nullable, DATE type)
ALTER TABLE form_templates
ADD COLUMN due_date DATE;

-- Step 2: Add CHECK constraint to ensure due_date is not in the past
-- This allows NULL values but enforces >= CURRENT_DATE when provided
ALTER TABLE form_templates
ADD CONSTRAINT check_due_date_valid
CHECK (due_date IS NULL OR due_date >= CURRENT_DATE);

-- Step 3 (Optional): Add index for due_date queries
-- Uncomment the lines below if you plan to query/filter by due_date frequently
-- This creates a partial index (only indexes non-NULL values for efficiency)
-- CREATE INDEX idx_form_templates_due_date
-- ON form_templates(due_date)
-- WHERE due_date IS NOT NULL;

-- ==========================================
-- VERIFICATION QUERIES
-- ==========================================

-- Query 1: Verify the column was added successfully
SELECT
    column_name,
    data_type,
    is_nullable,
    column_default
FROM information_schema.columns
WHERE table_name = 'form_templates'
AND column_name = 'due_date';

-- Query 2: Verify the constraint was added
SELECT
    constraint_name,
    check_clause
FROM information_schema.check_constraints
WHERE constraint_name = 'check_due_date_valid';

-- Query 3: Check existing records (should all have NULL due_date)
SELECT
    COUNT(*) as total_forms,
    COUNT(due_date) as forms_with_due_date,
    COUNT(*) - COUNT(due_date) as forms_without_due_date
FROM form_templates;

-- ==========================================
-- TEST QUERIES (Optional - for validation)
-- ==========================================
-- Uncomment and modify these queries to test the constraint

-- Test 1: Insert with NULL due_date (should succeed)
-- INSERT INTO form_templates (school_id, form_name, fillout_form_id, due_date, status, is_active)
-- VALUES (
--     'your-school-id-here'::UUID,
--     'Test Form Without Due Date',
--     'fillout123',
--     NULL,
--     'school_default',
--     true
-- );

-- Test 2: Insert with today's date (should succeed)
-- INSERT INTO form_templates (school_id, form_name, fillout_form_id, due_date, status, is_active)
-- VALUES (
--     'your-school-id-here'::UUID,
--     'Test Form With Today Due Date',
--     'fillout124',
--     CURRENT_DATE,
--     'school_default',
--     true
-- );

-- Test 3: Insert with future date (should succeed)
-- INSERT INTO form_templates (school_id, form_name, fillout_form_id, due_date, status, is_active)
-- VALUES (
--     'your-school-id-here'::UUID,
--     'Test Form With Future Due Date',
--     'fillout125',
--     '2025-12-31',
--     'school_default',
--     true
-- );

-- Test 4: Insert with past date (should FAIL with constraint violation)
-- INSERT INTO form_templates (school_id, form_name, fillout_form_id, due_date, status, is_active)
-- VALUES (
--     'your-school-id-here'::UUID,
--     'Test Form With Past Due Date',
--     'fillout126',
--     '2024-01-01',
--     'school_default',
--     true
-- );

-- ==========================================
-- ROLLBACK SCRIPT (if needed)
-- ==========================================
-- Use this only if you need to undo the migration
-- WARNING: This will permanently delete the due_date column and all its data

-- DROP INDEX IF EXISTS idx_form_templates_due_date;
-- ALTER TABLE form_templates DROP CONSTRAINT IF EXISTS check_due_date_valid;
-- ALTER TABLE form_templates DROP COLUMN IF EXISTS due_date;

-- ==========================================
-- MIGRATION COMPLETE
-- ==========================================
-- Expected Results:
-- ✅ due_date column added to form_templates table
-- ✅ CHECK constraint enforces due_date >= CURRENT_DATE (or NULL)
-- ✅ All existing records have due_date = NULL
-- ✅ New API requests can now include optional due_date field
-- ==========================================
