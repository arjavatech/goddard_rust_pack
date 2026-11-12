-- Add Owner to the allowed role values
ALTER TABLE users DROP CONSTRAINT IF EXISTS check_role;
ALTER TABLE users ADD CONSTRAINT check_role
  CHECK (role IN ('SuperAdmin', 'Admin', 'Teacher', 'Parent', 'primary-parent', 'secondary-parent', 'Employee', 'Owner'));
