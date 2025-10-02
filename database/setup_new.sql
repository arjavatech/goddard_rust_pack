-- ==========================================
-- ENROLLMENT FORM STATUS SYNCHRONIZATION
-- Date: 2025-10-02
-- Description: Automatic synchronization of form statuses from student_form_assignments
--              to enrollments table with auto-completion logic
-- ==========================================

-- ==========================================
-- FUNCTION 1: Sync Enrollment Application Status
-- ==========================================
-- This function updates the enrollments.application_status JSONB field
-- with a mapping of form_name -> status for all assigned forms
-- Example result: {"Authorization_form": "approved", "Admission_form": "incomplete"}

CREATE OR REPLACE FUNCTION sync_enrollment_form_status()
RETURNS TRIGGER AS $$
DECLARE
    target_enrollment_id UUID;
    new_application_status JSONB;
BEGIN
    -- Determine which enrollment_id to update
    -- Handle INSERT/UPDATE (use NEW) and DELETE (use OLD)
    IF TG_OP = 'DELETE' THEN
        target_enrollment_id := OLD.enrollment_id;
    ELSE
        target_enrollment_id := NEW.enrollment_id;
    END IF;

    -- Build the application_status JSONB by aggregating all form assignments
    -- Join with form_templates to get the form_name
    SELECT COALESCE(
        jsonb_object_agg(
            ft.form_name,  -- Key: form name from form_templates
            sfa.status     -- Value: status from student_form_assignments
        ),
        '{}'::jsonb  -- Empty JSONB if no forms found
    )
    INTO new_application_status
    FROM student_form_assignments sfa
    JOIN form_templates ft ON ft.id = sfa.form_template_id
    WHERE sfa.enrollment_id = target_enrollment_id
      AND sfa.is_active = true;  -- Only include active assignments

    -- Update the enrollments table with the new application_status
    UPDATE enrollments
    SET
        application_status = new_application_status,
        updated_at = NOW()
    WHERE id = target_enrollment_id;

    -- After updating application_status, check if enrollment should be completed
    PERFORM check_enrollment_completion(target_enrollment_id);

    -- Return appropriate value based on operation
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    ELSE
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- ==========================================
-- FUNCTION 2: Check and Update Enrollment Completion Status
-- ==========================================
-- This function checks if ALL forms for an enrollment are approved
-- If yes, it automatically sets enrollments.status to 'completed'

CREATE OR REPLACE FUNCTION check_enrollment_completion(p_enrollment_id UUID)
RETURNS void AS $$
DECLARE
    total_forms INTEGER;
    approved_forms INTEGER;
    all_approved BOOLEAN;
BEGIN
    -- Count total active form assignments for this enrollment
    SELECT COUNT(*)
    INTO total_forms
    FROM student_form_assignments
    WHERE enrollment_id = p_enrollment_id
      AND is_active = true;

    -- Count how many forms are approved
    -- You can adjust the status check based on your business logic
    -- Options: 'approved', 'completed', or both
    SELECT COUNT(*)
    INTO approved_forms
    FROM student_form_assignments
    WHERE enrollment_id = p_enrollment_id
      AND is_active = true
      AND status IN ('approved', 'completed');  -- Adjust as needed

    -- Determine if all forms are approved
    all_approved := (total_forms > 0 AND total_forms = approved_forms);

    -- Update enrollment status based on completion
    IF all_approved THEN
        UPDATE enrollments
        SET
            status = 'completed',
            updated_at = NOW()
        WHERE id = p_enrollment_id
          AND status != 'completed';  -- Only update if not already completed
    ELSE
        -- Optional: Set back to 'incomplete' or 'pending' if not all approved
        -- Uncomment the following if you want to auto-revert status
        /*
        UPDATE enrollments
        SET
            status = 'incomplete',
            updated_at = NOW()
        WHERE id = p_enrollment_id
          AND status = 'completed';  -- Only downgrade if it was completed
        */
    END IF;
END;
$$ LANGUAGE plpgsql;

-- ==========================================
-- TRIGGER: Auto-sync on student_form_assignments changes
-- ==========================================
-- This trigger fires whenever a form assignment is inserted, updated, or deleted
-- It ensures the enrollments table always has up-to-date form status information

DROP TRIGGER IF EXISTS trigger_sync_enrollment_form_status ON student_form_assignments;

CREATE TRIGGER trigger_sync_enrollment_form_status
    AFTER INSERT OR UPDATE OR DELETE ON student_form_assignments
    FOR EACH ROW
    EXECUTE FUNCTION sync_enrollment_form_status();

-- ==========================================
-- OPTIONAL: Initial Sync Function
-- ==========================================
-- Run this function once to sync existing data if you're adding these triggers
-- to an existing database with data

CREATE OR REPLACE FUNCTION sync_all_enrollment_statuses()
RETURNS void AS $$
DECLARE
    enrollment_record RECORD;
BEGIN
    -- Loop through all enrollments and sync their statuses
    FOR enrollment_record IN
        SELECT DISTINCT id FROM enrollments WHERE is_active = true
    LOOP
        -- Build and update application_status for each enrollment
        UPDATE enrollments e
        SET application_status = (
            SELECT COALESCE(
                jsonb_object_agg(ft.form_name, sfa.status),
                '{}'::jsonb
            )
            FROM student_form_assignments sfa
            JOIN form_templates ft ON ft.id = sfa.form_template_id
            WHERE sfa.enrollment_id = enrollment_record.id
              AND sfa.is_active = true
        ),
        updated_at = NOW()
        WHERE e.id = enrollment_record.id;

        -- Check completion status
        PERFORM check_enrollment_completion(enrollment_record.id);
    END LOOP;

    RAISE NOTICE 'Synced all enrollment statuses successfully';
END;
$$ LANGUAGE plpgsql;

-- ==========================================
-- USAGE INSTRUCTIONS
-- ==========================================
-- 1. Run this script to create the functions and trigger
-- 2. (Optional) Run the initial sync for existing data:
--    SELECT sync_all_enrollment_statuses();
-- 3. The trigger will now automatically maintain:
--    - enrollments.application_status JSONB field
--    - enrollments.status (auto-complete when all forms approved)
--
-- TESTING:
-- INSERT INTO student_form_assignments (...) VALUES (...);
-- UPDATE student_form_assignments SET status = 'approved' WHERE id = '...';
-- DELETE FROM student_form_assignments WHERE id = '...';
-- SELECT id, application_status, status FROM enrollments;
-- ==========================================

COMMENT ON FUNCTION sync_enrollment_form_status() IS 'Automatically syncs form statuses from student_form_assignments to enrollments.application_status JSONB field';
COMMENT ON FUNCTION check_enrollment_completion(UUID) IS 'Checks if all forms are approved and updates enrollment status to completed';
COMMENT ON FUNCTION sync_all_enrollment_statuses() IS 'One-time sync function to update all existing enrollments with current form statuses';
