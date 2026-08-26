-- Document request and review workflow.  The legacy `documents` table remains
-- untouched because it is generic enrollment attachment metadata.

CREATE TABLE IF NOT EXISTS document_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id),
    audience VARCHAR(16) NOT NULL CHECK (audience IN ('student', 'employee')),
    target_scope VARCHAR(16) NOT NULL CHECK (target_scope IN ('all', 'selected')),
    document_name VARCHAR(255) NOT NULL,
    instructions TEXT,
    due_date DATE,
    status VARCHAR(16) NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'active', 'closed', 'archived')),
    created_by UUID NOT NULL REFERENCES users(id),
    published_at TIMESTAMP,
    closed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP
);

CREATE TABLE IF NOT EXISTS document_request_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_request_id UUID NOT NULL REFERENCES document_requests(id) ON DELETE CASCADE,
    school_id UUID NOT NULL REFERENCES schools(id),
    enrollment_id UUID REFERENCES enrollments(id),
    child_id UUID REFERENCES children(id),
    employee_id UUID REFERENCES employees(id),
    status VARCHAR(16) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'submitted', 'approved', 'rejected')),
    latest_submission_id UUID,
    assigned_at TIMESTAMP NOT NULL DEFAULT NOW(),
    submitted_at TIMESTAMP,
    reviewed_at TIMESTAMP,
    reviewed_by UUID REFERENCES users(id),
    rejection_reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP,
    CONSTRAINT document_assignment_target_check CHECK (
        (child_id IS NOT NULL AND enrollment_id IS NOT NULL AND employee_id IS NULL)
        OR (employee_id IS NOT NULL AND child_id IS NULL AND enrollment_id IS NULL)
    ),
    CONSTRAINT unique_document_request_child UNIQUE (document_request_id, child_id),
    CONSTRAINT unique_document_request_employee UNIQUE (document_request_id, employee_id)
);

CREATE TABLE IF NOT EXISTS document_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    assignment_id UUID NOT NULL REFERENCES document_request_assignments(id) ON DELETE CASCADE,
    school_id UUID NOT NULL REFERENCES schools(id),
    version_number INTEGER NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    original_file_name VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    file_size_bytes BIGINT NOT NULL,
    checksum_sha256 VARCHAR(128),
    uploaded_by UUID NOT NULL REFERENCES users(id),
    submitted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_document_submission_version UNIQUE (assignment_id, version_number)
);

ALTER TABLE document_request_assignments
    ADD CONSTRAINT document_assignment_latest_submission_fk
    FOREIGN KEY (latest_submission_id) REFERENCES document_submissions(id);

CREATE TABLE IF NOT EXISTS document_audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id),
    document_request_id UUID NOT NULL REFERENCES document_requests(id) ON DELETE CASCADE,
    assignment_id UUID REFERENCES document_request_assignments(id) ON DELETE CASCADE,
    submission_id UUID REFERENCES document_submissions(id) ON DELETE SET NULL,
    event_type VARCHAR(32) NOT NULL,
    actor_id UUID REFERENCES users(id),
    reason TEXT,
    metadata JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_document_requests_school_audience_status ON document_requests(school_id, audience, status);
CREATE INDEX IF NOT EXISTS idx_document_assignments_request_status ON document_request_assignments(document_request_id, status);
CREATE INDEX IF NOT EXISTS idx_document_assignments_school_status ON document_request_assignments(school_id, status);
CREATE INDEX IF NOT EXISTS idx_document_assignments_child ON document_request_assignments(child_id);
CREATE INDEX IF NOT EXISTS idx_document_assignments_employee ON document_request_assignments(employee_id);
CREATE INDEX IF NOT EXISTS idx_document_submissions_assignment_version ON document_submissions(assignment_id, version_number DESC);
CREATE INDEX IF NOT EXISTS idx_document_audit_assignment_created ON document_audit_events(assignment_id, created_at DESC);

-- 016 installs the common local audit trigger. Add its equivalent for these
-- tables so user devices never provide audit timestamps.
DROP TRIGGER IF EXISTS set_school_local_audit_time_trigger ON document_requests;
CREATE TRIGGER set_school_local_audit_time_trigger BEFORE INSERT OR UPDATE ON document_requests
FOR EACH ROW EXECUTE FUNCTION set_school_local_audit_time();
DROP TRIGGER IF EXISTS set_school_local_audit_time_trigger ON document_request_assignments;
CREATE TRIGGER set_school_local_audit_time_trigger BEFORE INSERT OR UPDATE ON document_request_assignments
FOR EACH ROW EXECUTE FUNCTION set_school_local_audit_time();

CREATE OR REPLACE FUNCTION set_school_local_document_event_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.created_at := school_local_now(NEW.school_id);
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS set_school_local_document_submission_time_trigger ON document_submissions;
CREATE TRIGGER set_school_local_document_submission_time_trigger BEFORE INSERT ON document_submissions
FOR EACH ROW EXECUTE FUNCTION set_school_local_document_event_time();
DROP TRIGGER IF EXISTS set_school_local_document_audit_time_trigger ON document_audit_events;
CREATE TRIGGER set_school_local_document_audit_time_trigger BEFORE INSERT ON document_audit_events
FOR EACH ROW EXECUTE FUNCTION set_school_local_document_event_time();
