-- Backfill approved_form_name (and form_academic_year) for all approved assignments.
-- Format: form/doc-name-first-last[-2026-2027] (original casing preserved)
-- Only touches approved records; overwrites any prior value to enforce the standard format.

-- 1. Student document assignments
UPDATE document_request_assignments dra
SET
  form_academic_year = '2026-2027',
  approved_form_name  = REGEXP_REPLACE(
    dr.document_name || '-' || c.first_name || '-' || c.last_name || '-2026-2027',
    '[^a-zA-Z0-9]+', '-', 'g'
  ),
  updated_at = NOW()
FROM document_requests dr, children c
WHERE dra.document_request_id = dr.id
  AND dra.child_id = c.id
  AND dra.status = 'approved';

-- 2. Employee document assignments
UPDATE document_request_assignments dra
SET
  approved_form_name = REGEXP_REPLACE(
    dr.document_name || '-' || u.first_name || '-' || u.last_name,
    '[^a-zA-Z0-9]+', '-', 'g'
  ),
  updated_at = NOW()
FROM document_requests dr, employees e, users u
WHERE dra.document_request_id = dr.id
  AND dra.employee_id = e.id
  AND e.user_id = u.id
  AND dra.status = 'approved';

-- 3. Student form assignments
UPDATE student_form_assignments sfa
SET
  form_academic_year = '2026-2027',
  approved_form_name  = REGEXP_REPLACE(
    ft.form_name || '-' || c.first_name || '-' || c.last_name || '-2026-2027',
    '[^a-zA-Z0-9]+', '-', 'g'
  ),
  updated_at = NOW()
FROM form_templates ft, children c
WHERE sfa.form_template_id = ft.id
  AND sfa.child_id = c.id
  AND sfa.status = 'approved';

-- 4. Employee form assignments
UPDATE employee_form_assignments efa
SET
  approved_form_name = REGEXP_REPLACE(
    eft.form_name || '-' || u.first_name || '-' || u.last_name,
    '[^a-zA-Z0-9]+', '-', 'g'
  ),
  updated_at = NOW()
FROM employee_form_templates eft, users u
WHERE efa.employee_form_template_id = eft.id
  AND efa.user_id = u.id
  AND efa.status = 'approved';
