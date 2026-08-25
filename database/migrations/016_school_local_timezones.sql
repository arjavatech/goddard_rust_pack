-- School-local audit timestamps.
-- A school stores a controlled short code; PostgreSQL uses the mapped IANA
-- timezone only while calculating the local wall-clock timestamp.

ALTER TABLE schools ADD COLUMN IF NOT EXISTS timezone VARCHAR(16);

UPDATE schools
SET timezone = CASE settings->>'timezone'
    WHEN 'America/New_York' THEN 'EST'
    WHEN 'America/Chicago' THEN 'CST'
    WHEN 'America/Denver' THEN 'MST'
    WHEN 'America/Los_Angeles' THEN 'PST'
    WHEN 'Asia/Kolkata' THEN 'IST'
    ELSE 'EST'
END
WHERE timezone IS NULL;

-- Keep the legacy settings field aligned for older API clients during the
-- rollout. The authoritative value is schools.timezone.
UPDATE schools
SET settings = jsonb_set(COALESCE(settings, '{}'::jsonb), '{timezone}', to_jsonb(timezone), true);

ALTER TABLE schools ALTER COLUMN timezone SET DEFAULT 'EST';
ALTER TABLE schools ALTER COLUMN timezone SET NOT NULL;
ALTER TABLE schools DROP CONSTRAINT IF EXISTS schools_timezone_check;
ALTER TABLE schools ADD CONSTRAINT schools_timezone_check CHECK (timezone IN (
    'EST','CST','MST','PST','AKST','HST','IST','GMT','CET','EET','GST','PKT',
    'BST','ICT','CST_CN','JST','KST','WIB','WITA','WIT','AEST','ACST','AWST',
    'NZST','BRT','ART','CLT','SAST','EAT','WAT'
));

CREATE OR REPLACE FUNCTION school_timezone_region(timezone_code TEXT)
RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $$
    SELECT CASE timezone_code
        WHEN 'EST' THEN 'America/New_York' WHEN 'CST' THEN 'America/Chicago'
        WHEN 'MST' THEN 'America/Denver' WHEN 'PST' THEN 'America/Los_Angeles'
        WHEN 'AKST' THEN 'America/Anchorage' WHEN 'HST' THEN 'Pacific/Honolulu'
        WHEN 'IST' THEN 'Asia/Kolkata' WHEN 'GMT' THEN 'Europe/London'
        WHEN 'CET' THEN 'Europe/Berlin' WHEN 'EET' THEN 'Europe/Helsinki'
        WHEN 'GST' THEN 'Asia/Dubai' WHEN 'PKT' THEN 'Asia/Karachi'
        WHEN 'BST' THEN 'Asia/Dhaka' WHEN 'ICT' THEN 'Asia/Bangkok'
        WHEN 'CST_CN' THEN 'Asia/Shanghai' WHEN 'JST' THEN 'Asia/Tokyo'
        WHEN 'KST' THEN 'Asia/Seoul' WHEN 'WIB' THEN 'Asia/Jakarta'
        WHEN 'WITA' THEN 'Asia/Makassar' WHEN 'WIT' THEN 'Asia/Jayapura'
        WHEN 'AEST' THEN 'Australia/Sydney' WHEN 'ACST' THEN 'Australia/Adelaide'
        WHEN 'AWST' THEN 'Australia/Perth' WHEN 'NZST' THEN 'Pacific/Auckland'
        WHEN 'BRT' THEN 'America/Sao_Paulo' WHEN 'ART' THEN 'America/Argentina/Buenos_Aires'
        WHEN 'CLT' THEN 'America/Santiago' WHEN 'SAST' THEN 'Africa/Johannesburg'
        WHEN 'EAT' THEN 'Africa/Nairobi' WHEN 'WAT' THEN 'Africa/Lagos'
        ELSE 'America/Los_Angeles'
    END;
$$;

CREATE OR REPLACE FUNCTION school_local_now(target_school_id UUID)
RETURNS TIMESTAMP LANGUAGE SQL STABLE AS $$
    SELECT timezone(school_timezone_region(s.timezone), NOW())::timestamp
    FROM schools s WHERE s.id = target_school_id;
$$;

CREATE OR REPLACE FUNCTION to_school_local_time(target_school_id UUID, instant TIMESTAMPTZ)
RETURNS TIMESTAMP LANGUAGE SQL STABLE AS $$
    SELECT timezone(school_timezone_region(s.timezone), instant)::timestamp
    FROM schools s WHERE s.id = target_school_id;
$$;

-- Legacy audit values were written as UTC clock values without timezone
-- metadata. Convert them once to the owning school's local clock value before
-- enabling the enforcement triggers below.
UPDATE schools s SET
    created_at = timezone(school_timezone_region(s.timezone), s.created_at AT TIME ZONE 'UTC')::timestamp,
    updated_at = CASE WHEN s.updated_at IS NULL THEN NULL ELSE timezone(school_timezone_region(s.timezone), s.updated_at AT TIME ZONE 'UTC')::timestamp END;
UPDATE users t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'), updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE children t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'), updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE classrooms t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'), updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE form_templates t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'), updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE enrollments t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'), updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE class_form_overrides t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'), updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE student_form_assignments t SET
    assigned_at = to_school_local_time(t.school_id, t.assigned_at AT TIME ZONE 'UTC'),
    approved_on = CASE WHEN t.approved_on IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.approved_on AT TIME ZONE 'UTC') END,
    created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'),
    updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE form_submissions t SET
    submitted_at = CASE WHEN t.submitted_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.submitted_at AT TIME ZONE 'UTC') END,
    processed_at = CASE WHEN t.processed_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.processed_at AT TIME ZONE 'UTC') END,
    created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'),
    updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE employees t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'), updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE employee_form_templates t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'), updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE employee_form_assignments t SET
    assigned_at = to_school_local_time(t.school_id, t.assigned_at AT TIME ZONE 'UTC'),
    approved_on = CASE WHEN t.approved_on IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.approved_on AT TIME ZONE 'UTC') END,
    created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'),
    updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE employee_form_submissions t SET
    submitted_at = CASE WHEN t.submitted_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.submitted_at AT TIME ZONE 'UTC') END,
    processed_at = CASE WHEN t.processed_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.processed_at AT TIME ZONE 'UTC') END,
    created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC'),
    updated_at = CASE WHEN t.updated_at IS NULL THEN NULL ELSE to_school_local_time(t.school_id, t.updated_at AT TIME ZONE 'UTC') END;
UPDATE requests t SET created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC');
UPDATE documents t SET uploaded_at = to_school_local_time(t.school_id, t.uploaded_at AT TIME ZONE 'UTC');
UPDATE class_transitions t SET
    transitioned_at = to_school_local_time(t.school_id, t.transitioned_at AT TIME ZONE 'UTC'),
    created_at = to_school_local_time(t.school_id, t.created_at AT TIME ZONE 'UTC');

CREATE OR REPLACE FUNCTION set_school_record_audit_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := timezone(school_timezone_region(NEW.timezone), NOW())::timestamp;
    IF TG_OP = 'INSERT' THEN NEW.created_at := local_now; END IF;
    NEW.updated_at := local_now;
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS set_school_record_audit_time_trigger ON schools;
CREATE TRIGGER set_school_record_audit_time_trigger BEFORE INSERT OR UPDATE ON schools
FOR EACH ROW EXECUTE FUNCTION set_school_record_audit_time();

-- Enforce backend-owned, school-local audit time for tables carrying school_id.
CREATE OR REPLACE FUNCTION set_school_local_audit_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := school_local_now(NEW.school_id);
    IF TG_OP = 'INSERT' THEN
        NEW.created_at := local_now;
    END IF;
    NEW.updated_at := local_now;
    RETURN NEW;
END;
$$;

DO $$
DECLARE table_name TEXT;
BEGIN
    FOREACH table_name IN ARRAY ARRAY[
        'users','children','classrooms','form_templates','enrollments','class_form_overrides',
        'student_form_assignments','form_submissions','employees','employee_form_templates',
        'employee_form_assignments','employee_form_submissions'
    ] LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', 'set_school_local_audit_time_trigger', table_name);
        EXECUTE format('CREATE TRIGGER %I BEFORE INSERT OR UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION set_school_local_audit_time()', 'set_school_local_audit_time_trigger', table_name);
    END LOOP;
END $$;

-- Requests have created_at only.
CREATE OR REPLACE FUNCTION set_school_local_created_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.created_at := school_local_now(NEW.school_id);
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS set_school_local_created_time_trigger ON requests;
CREATE TRIGGER set_school_local_created_time_trigger BEFORE INSERT ON requests
FOR EACH ROW EXECUTE FUNCTION set_school_local_created_time();

-- Assignment and review timestamps are server-owned audit values too.  These
-- specific triggers supplement the generic created_at/updated_at trigger.
CREATE OR REPLACE FUNCTION set_school_local_student_assignment_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := school_local_now(NEW.school_id);
    IF TG_OP = 'INSERT' THEN
        NEW.assigned_at := local_now;
    ELSIF NEW.approved_on IS DISTINCT FROM OLD.approved_on THEN
        NEW.approved_on := local_now;
    END IF;
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS set_school_local_student_assignment_time_trigger ON student_form_assignments;
CREATE TRIGGER set_school_local_student_assignment_time_trigger
BEFORE INSERT OR UPDATE ON student_form_assignments
FOR EACH ROW EXECUTE FUNCTION set_school_local_student_assignment_time();

CREATE OR REPLACE FUNCTION set_school_local_employee_assignment_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := school_local_now(NEW.school_id);
    IF TG_OP = 'INSERT' THEN
        NEW.assigned_at := local_now;
    ELSIF NEW.approved_on IS DISTINCT FROM OLD.approved_on THEN
        NEW.approved_on := local_now;
    END IF;
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS set_school_local_employee_assignment_time_trigger ON employee_form_assignments;
CREATE TRIGGER set_school_local_employee_assignment_time_trigger
BEFORE INSERT OR UPDATE ON employee_form_assignments
FOR EACH ROW EXECUTE FUNCTION set_school_local_employee_assignment_time();

CREATE OR REPLACE FUNCTION set_school_local_submission_processing_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.processed_at IS DISTINCT FROM OLD.processed_at THEN
        NEW.processed_at := school_local_now(NEW.school_id);
    END IF;
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS set_school_local_form_submission_processing_time_trigger ON form_submissions;
CREATE TRIGGER set_school_local_form_submission_processing_time_trigger
BEFORE UPDATE ON form_submissions
FOR EACH ROW EXECUTE FUNCTION set_school_local_submission_processing_time();
DROP TRIGGER IF EXISTS set_school_local_employee_submission_processing_time_trigger ON employee_form_submissions;
CREATE TRIGGER set_school_local_employee_submission_processing_time_trigger
BEFORE UPDATE ON employee_form_submissions
FOR EACH ROW EXECUTE FUNCTION set_school_local_submission_processing_time();

-- Documents and classroom transitions also carry school-owned audit values.
CREATE OR REPLACE FUNCTION set_school_local_document_upload_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.uploaded_at := school_local_now(NEW.school_id);
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS set_school_local_document_upload_time_trigger ON documents;
CREATE TRIGGER set_school_local_document_upload_time_trigger BEFORE INSERT ON documents
FOR EACH ROW EXECUTE FUNCTION set_school_local_document_upload_time();

CREATE OR REPLACE FUNCTION set_school_local_transition_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := school_local_now(NEW.school_id);
    NEW.created_at := local_now;
    NEW.transitioned_at := local_now;
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS set_school_local_transition_time_trigger ON class_transitions;
CREATE TRIGGER set_school_local_transition_time_trigger BEFORE INSERT ON class_transitions
FOR EACH ROW EXECUTE FUNCTION set_school_local_transition_time();

-- Form submissions receive the Fillout submission time explicitly from the backend.
-- This trigger only protects created/updated audit values.
