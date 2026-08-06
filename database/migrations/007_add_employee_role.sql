-- Migration 007: Add Employee role to users check constraint

ALTER TABLE users DROP CONSTRAINT IF EXISTS check_role;
ALTER TABLE users ADD CONSTRAINT check_role
  CHECK (role IN ('SuperAdmin', 'Admin', 'Teacher', 'Parent', 'primary-parent', 'secondary-parent', 'Employee'));
