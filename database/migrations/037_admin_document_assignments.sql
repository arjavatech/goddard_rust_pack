-- Extend document requests and assignments to support Admin audience.

-- 1. Widen the audience check on document_requests to allow 'admin'.
ALTER TABLE document_requests
  DROP CONSTRAINT IF EXISTS document_requests_audience_check;

ALTER TABLE document_requests
  ADD CONSTRAINT document_requests_audience_check
    CHECK (audience IN ('student', 'employee', 'admin'));

-- 2. Add admin_user_id FK column to document_request_assignments.
ALTER TABLE document_request_assignments
  ADD COLUMN IF NOT EXISTS admin_user_id UUID REFERENCES users(id);

-- 3. Replace the mutual-exclusivity CHECK to allow admin assignments.
ALTER TABLE document_request_assignments
  DROP CONSTRAINT IF EXISTS document_request_assignments_check;

ALTER TABLE document_request_assignments
  ADD CONSTRAINT document_request_assignments_check CHECK (
    (child_id IS NOT NULL AND enrollment_id IS NOT NULL AND employee_id IS NULL AND admin_user_id IS NULL) OR
    (employee_id IS NOT NULL AND child_id IS NULL AND enrollment_id IS NULL AND admin_user_id IS NULL) OR
    (admin_user_id IS NOT NULL AND employee_id IS NULL AND child_id IS NULL AND enrollment_id IS NULL)
  );

-- 4. Unique index for admin assignments.
CREATE UNIQUE INDEX IF NOT EXISTS dra_admin_request_unique
  ON document_request_assignments(document_request_id, admin_user_id)
  WHERE admin_user_id IS NOT NULL;
