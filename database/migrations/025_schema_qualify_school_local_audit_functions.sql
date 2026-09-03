-- Supabase Auth executes the public.users trigger with a search_path that may
-- not include public. Qualify all helper calls and pin the function search
-- path so audit triggers behave consistently for Auth and application writes.

CREATE OR REPLACE FUNCTION public.set_school_record_audit_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := timezone(public.school_timezone_region(NEW.timezone), NOW())::timestamp;
    IF TG_OP = 'INSERT' THEN NEW.created_at := local_now; END IF;
    NEW.updated_at := local_now;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.set_school_local_audit_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := public.school_local_now(NEW.school_id);
    IF TG_OP = 'INSERT' THEN NEW.created_at := local_now; END IF;
    NEW.updated_at := local_now;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.set_school_local_created_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
BEGIN
    NEW.created_at := public.school_local_now(NEW.school_id);
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.set_school_local_student_assignment_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := public.school_local_now(NEW.school_id);
    IF TG_OP = 'INSERT' THEN
        NEW.assigned_at := local_now;
    ELSIF NEW.approved_on IS DISTINCT FROM OLD.approved_on THEN
        NEW.approved_on := local_now;
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.set_school_local_employee_assignment_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := public.school_local_now(NEW.school_id);
    IF TG_OP = 'INSERT' THEN
        NEW.assigned_at := local_now;
    ELSIF NEW.approved_on IS DISTINCT FROM OLD.approved_on THEN
        NEW.approved_on := local_now;
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.set_school_local_submission_processing_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.processed_at IS DISTINCT FROM OLD.processed_at THEN
        NEW.processed_at := public.school_local_now(NEW.school_id);
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.set_school_local_document_upload_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
BEGIN
    NEW.uploaded_at := public.school_local_now(NEW.school_id);
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.set_school_local_transition_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := public.school_local_now(NEW.school_id);
    NEW.created_at := local_now;
    NEW.transitioned_at := local_now;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.set_school_local_document_event_time()
RETURNS TRIGGER LANGUAGE plpgsql
SET search_path = public, pg_temp AS $$
BEGIN
    NEW.created_at := public.school_local_now(NEW.school_id);
    RETURN NEW;
END;
$$;
