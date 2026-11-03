-- Allow admin users (who have no employees record) to receive form assignments.
-- employee_id becomes nullable; user_id already exists and identifies any user.

ALTER TABLE employee_form_assignments
  ALTER COLUMN employee_id DROP NOT NULL;

-- The old unique constraint only covered employee assignments. Replace it with
-- two partial unique indexes that cover both cases without conflict.
ALTER TABLE employee_form_assignments
  DROP CONSTRAINT IF EXISTS employee_form_assignments_employee_id_employee_form_template_id_key;

CREATE UNIQUE INDEX IF NOT EXISTS efa_employee_template_unique
  ON employee_form_assignments(employee_id, employee_form_template_id)
  WHERE employee_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS efa_admin_template_unique
  ON employee_form_assignments(user_id, employee_form_template_id)
  WHERE employee_id IS NULL;
