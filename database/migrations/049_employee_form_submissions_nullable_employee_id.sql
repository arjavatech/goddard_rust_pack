-- Allow admin form submissions where employee_id is NULL (admins have no employees record)
ALTER TABLE employee_form_submissions ALTER COLUMN employee_id DROP NOT NULL;
