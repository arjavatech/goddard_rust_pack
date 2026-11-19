-- Migration 050: Mark all existing active employee form templates as school defaults
--
-- Employee form templates now support status = 'school_default' to trigger
-- auto-assignment whenever a new employee or admin is added to the school.
-- All currently active templates are promoted to 'school_default' so that
-- every existing school benefits from the auto-assignment behaviour
-- immediately after this migration runs.
--
-- Templates with status = 'inactive' are left unchanged.

UPDATE employee_form_templates
SET    status     = 'school_default',
       updated_at = NOW()
WHERE  (is_active = true OR is_active IS NULL)
  AND  status != 'inactive';
