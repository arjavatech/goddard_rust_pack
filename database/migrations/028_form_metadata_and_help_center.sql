-- Add academic year and approved form name fields for form/document approvals

-- Student form assignments: track the academic year and the admin-set file name at approval time
ALTER TABLE student_form_assignments
    ADD COLUMN IF NOT EXISTS form_academic_year VARCHAR(9),
    ADD COLUMN IF NOT EXISTS approved_form_name TEXT;

-- Employee form assignments: only form name (no academic year for employees)
ALTER TABLE employee_form_assignments
    ADD COLUMN IF NOT EXISTS approved_form_name TEXT;

-- Document request assignments: student docs need academic year, employee docs need name only
ALTER TABLE document_request_assignments
    ADD COLUMN IF NOT EXISTS form_academic_year VARCHAR(9),
    ADD COLUMN IF NOT EXISTS approved_form_name TEXT;

-- Backfill form_academic_year and approved_form_name for already-approved assignments
-- Only fills rows where both fields are NULL/empty (safe to re-run)

-- Student form assignments: set academic year to 2026-2027 and derive form name
UPDATE student_form_assignments
SET
  form_academic_year = '2026-2027',
  approved_form_name = COALESCE(
    -- Prefer the original uploaded filename (strip file extension)
    NULLIF(TRIM(REGEXP_REPLACE(student_form_assignments.manual_pdf_file_name, '\.[^.]+$', '')), ''),
    -- Fall back to <form-name>-<first>-<last>-2026-2027 (slugified)
    LOWER(REGEXP_REPLACE(
      ft.form_name || '-' || c.first_name || '-' || c.last_name || '-2026-2027',
      '[^a-zA-Z0-9]+', '-', 'g'
    ))
  ),
  updated_at = NOW()
FROM form_templates ft, children c
WHERE student_form_assignments.form_template_id = ft.id
  AND student_form_assignments.child_id = c.id
  AND student_form_assignments.status = 'approved'
  AND (student_form_assignments.form_academic_year IS NULL OR student_form_assignments.form_academic_year = '')
  AND (student_form_assignments.approved_form_name IS NULL OR student_form_assignments.approved_form_name = '');

-- Employee form assignments: no academic year field, only backfill approved_form_name
UPDATE employee_form_assignments
SET
  approved_form_name = COALESCE(
    NULLIF(TRIM(REGEXP_REPLACE(employee_form_assignments.manual_pdf_file_name, '\.[^.]+$', '')), ''),
    LOWER(REGEXP_REPLACE(
      eft.form_name || '-' || u.first_name || '-' || u.last_name,
      '[^a-zA-Z0-9]+', '-', 'g'
    ))
  ),
  updated_at = NOW()
FROM employee_form_templates eft, users u
WHERE employee_form_assignments.employee_form_template_id = eft.id
  AND employee_form_assignments.user_id = u.id
  AND employee_form_assignments.status = 'approved'
  AND (employee_form_assignments.approved_form_name IS NULL OR employee_form_assignments.approved_form_name = '');

-- Help center messages table
CREATE TABLE help_center_messages (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id       UUID        NOT NULL REFERENCES schools(id),
    sender_user_id  UUID        REFERENCES users(id),
    sender_role     TEXT,
    sender_name     TEXT        NOT NULL,
    sender_email    TEXT        NOT NULL,
    subject         TEXT        NOT NULL,
    message         TEXT        NOT NULL,
    to_email        TEXT        NOT NULL,
    from_email      TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ON help_center_messages(school_id);
CREATE INDEX ON help_center_messages(sender_user_id);
