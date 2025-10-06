-- Goddard School Enrollment Management System - Database Setup
-- Database Schema for 11 tables from System Architecture Design v2.2

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. SCHOOLS Table
CREATE TABLE schools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) UNIQUE NOT NULL,  -- Globally unique school name
    subdomain VARCHAR(100) UNIQUE NOT NULL,  -- Globally unique subdomain
    settings JSONB,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

-- 2. USERS Table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL,  -- Unique per school (composite constraint below)
    role VARCHAR(50) NOT NULL,
    is_verified BOOLEAN DEFAULT false,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP,
    metadata JSONB,
    is_active BOOLEAN DEFAULT true
);

-- Add composite unique constraint for email per school
ALTER TABLE users ADD CONSTRAINT unique_email_per_school UNIQUE (school_id, email);


-- 3. CHILDREN Table
CREATE TABLE children (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent_id UUID NOT NULL REFERENCES users(id),
    secondary_parent_id UUID REFERENCES users(id),
    school_id UUID NOT NULL REFERENCES schools(id),
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    birth_date DATE,
    gender VARCHAR(20),
    status VARCHAR(50) DEFAULT 'active',
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

-- 4. CLASSROOMS Table
CREATE TABLE classrooms (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    name VARCHAR(255) NOT NULL,  -- Unique per school (composite constraint below)
    age_group VARCHAR(50),
    capacity INTEGER,
    enrolled_count INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

-- Add composite unique constraint for classroom name per school
ALTER TABLE classrooms ADD CONSTRAINT unique_classroom_name_per_school UNIQUE (school_id, name);

-- 5. FORM_TEMPLATES Table
CREATE TABLE form_templates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    form_name VARCHAR(255) NOT NULL,  -- Unique per school (composite constraint below)
    form_type VARCHAR(100),
    fillout_form_id VARCHAR(255),  -- Stores the Fillout form ID/URL
    status VARCHAR(50),
    is_required BOOLEAN DEFAULT false,
    display_order INTEGER,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

-- Add composite unique constraint for form name per school
ALTER TABLE form_templates ADD CONSTRAINT unique_form_name_per_school UNIQUE (school_id, form_name);

-- 6. ENROLLMENTS Table
CREATE TABLE enrollments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    child_id UUID NOT NULL REFERENCES children(id),
    school_id UUID NOT NULL REFERENCES schools(id),
    classroom_id UUID NOT NULL REFERENCES classrooms(id),
    status VARCHAR(50),
    application_status JSONB,
    progress JSONB,
    submitted_at TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

-- 7. CLASS_FORM_OVERRIDES Table
CREATE TABLE class_form_overrides (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    classroom_id UUID NOT NULL REFERENCES classrooms(id),
    form_template_id UUID NOT NULL REFERENCES form_templates(id),
    action VARCHAR(50),
    is_required BOOLEAN,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP,
    is_active BOOLEAN DEFAULT true
);

-- 8. STUDENT_FORM_ASSIGNMENTS Table
CREATE TABLE student_form_assignments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    enrollment_id UUID NOT NULL REFERENCES enrollments(id),
    child_id UUID NOT NULL REFERENCES children(id),
    form_template_id UUID NOT NULL REFERENCES form_templates(id),
    assignment_source VARCHAR(50),
    status VARCHAR(50) DEFAULT 'Not Started',
    is_required BOOLEAN DEFAULT false,
    assigned_at TIMESTAMP DEFAULT NOW(),
    recent_form_submission_id UUID,
    approved_by UUID,
    notes TEXT,
    approved_on TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

-- 9. FORM_SUBMISSIONS Table
CREATE TABLE form_submissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    enrollment_id UUID NOT NULL REFERENCES enrollments(id),
    student_form_assignment_id UUID NOT NULL REFERENCES student_form_assignments(id),
    form_template_id UUID NOT NULL REFERENCES form_templates(id),
    fillout_submission_id VARCHAR(255) UNIQUE NOT NULL,
    form_data JSONB,
    metadata JSONB,
    submitted_at TIMESTAMP,
    processed_at TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

-- 10. DOCUMENTS Table
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    enrollment_id UUID NOT NULL REFERENCES enrollments(id),
    school_id UUID NOT NULL REFERENCES schools(id),
    document_type VARCHAR(100),
    storage_path TEXT,
    file_name VARCHAR(255),
    uploaded_at TIMESTAMP DEFAULT NOW()
);



-- Create indexes for better performance
CREATE INDEX idx_users_school_id ON users(school_id);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_children_school_id ON children(school_id);
CREATE INDEX idx_children_parent_id ON children(parent_id);
CREATE INDEX idx_enrollments_school_id ON enrollments(school_id);
CREATE INDEX idx_enrollments_child_id ON enrollments(child_id);
CREATE INDEX idx_enrollments_classroom_id ON enrollments(classroom_id);
CREATE INDEX idx_form_templates_school_id ON form_templates(school_id);
CREATE INDEX idx_classrooms_school_id ON classrooms(school_id);
CREATE INDEX idx_class_form_overrides_school_id ON class_form_overrides(school_id);
CREATE INDEX idx_class_form_overrides_classroom_id ON class_form_overrides(classroom_id);
CREATE INDEX idx_student_form_assignments_school_id ON student_form_assignments(school_id);
CREATE INDEX idx_student_form_assignments_enrollment_id ON student_form_assignments(enrollment_id);
CREATE INDEX idx_form_submissions_school_id ON form_submissions(school_id);
CREATE INDEX idx_form_submissions_fillout_id ON form_submissions(fillout_submission_id);
CREATE INDEX idx_documents_school_id ON documents(school_id);
CREATE INDEX idx_documents_enrollment_id ON documents(enrollment_id);

-- Add constraints for data integrity
ALTER TABLE users ADD CONSTRAINT check_role
CHECK (role IN ('SuperAdmin', 'Admin', 'Teacher', 'Parent', 'primary-parent', 'secondary-parent'));

ALTER TABLE children ADD CONSTRAINT check_status
CHECK (status IN ('active', 'inactive', 'graduated', 'withdrawn'));

ALTER TABLE children ADD CONSTRAINT check_gender
CHECK (gender IS NULL OR gender IN ('male', 'female', 'other'));

ALTER TABLE form_templates ADD CONSTRAINT check_status
CHECK (status IN ('active', 'inactive', 'draft', 'archived', 'school_default', 'available'));

ALTER TABLE enrollments ADD CONSTRAINT check_status
CHECK (status IN ('pending', 'active', 'completed', 'cancelled', 'incomplete'));

ALTER TABLE class_form_overrides ADD CONSTRAINT check_action
CHECK (action IS NULL OR action IN ('include', 'exclude', 'modify'));

ALTER TABLE student_form_assignments ADD CONSTRAINT check_assignment_source
CHECK (assignment_source IN ('school_default', 'class_override', 'manual'));

ALTER TABLE student_form_assignments ADD CONSTRAINT check_status
CHECK (status IN ('incomplete', 'in_progress', 'completed', 'archived'));

-- Comments for documentation
COMMENT ON TABLE schools IS 'School entities in the multi-tenant system';
COMMENT ON TABLE users IS 'System users including parents, admins, and staff';
COMMENT ON TABLE children IS 'Student/child records linked to parents';
COMMENT ON TABLE classrooms IS 'Physical or logical classroom groupings';
COMMENT ON TABLE form_templates IS 'Fillout.com form templates managed by schools';
COMMENT ON TABLE enrollments IS 'Child enrollment records linking students to classrooms';
COMMENT ON TABLE class_form_overrides IS 'Classroom-specific form requirements and overrides';
COMMENT ON TABLE student_form_assignments IS 'Individual form assignments to students';
COMMENT ON TABLE form_submissions IS 'Actual form submissions from Fillout.com webhooks';
COMMENT ON TABLE documents IS 'File attachments and documents related to enrollments';

-- ==========================================
-- TRIGGER FUNCTION FOR SUPABASE AUTH INTEGRATION
-- ==========================================
-- This function automatically creates a user record in the public.users table
-- when a new user signs up via Supabase Auth
-- It extracts user metadata from auth.users and creates corresponding record

CREATE OR REPLACE FUNCTION public.handle_new_auth_user()
RETURNS TRIGGER AS $$
BEGIN
    -- Insert new user into public.users table using auth user data
    INSERT INTO public.users (
        id,
        school_id,
        first_name,
        last_name,
        email,
        role,
        is_verified,
        created_at,
        metadata,
        is_active
    ) VALUES (
        NEW.id,  -- Use the same UUID from auth.users
        COALESCE(
            (NEW.raw_user_meta_data->>'school_id')::UUID,
            NULL
        ),
        COALESCE(
            NEW.raw_user_meta_data->>'first_name',
            ''
        ),
        COALESCE(
            NEW.raw_user_meta_data->>'last_name',
            ''
        ),
        NEW.email,
        COALESCE(
            NEW.raw_user_meta_data->>'role',
            'Parent'  -- Default role if not specified
        ),
        COALESCE(NEW.email_confirmed_at IS NOT NULL, false),  -- Set verified based on email confirmation
        NOW(),
        NEW.raw_user_meta_data,  -- Store complete metadata
        true
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Create trigger on auth.users table
-- This trigger fires after a new user is inserted in auth.users
CREATE TRIGGER on_auth_user_created
    AFTER INSERT ON auth.users
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_new_auth_user();

-- ==========================================
-- OPTIONAL: Function to sync existing auth users
-- ==========================================
-- Run this once if you have existing auth users that need to be synced

CREATE OR REPLACE FUNCTION public.sync_existing_auth_users()
RETURNS void AS $$
DECLARE
    auth_user RECORD;
BEGIN
    FOR auth_user IN
        SELECT * FROM auth.users
        WHERE id NOT IN (SELECT id FROM public.users)
    LOOP
        INSERT INTO public.users (
            id,
            school_id,
            first_name,
            last_name,
            email,
            role,
            is_verified,
            created_at,
            metadata,
            is_active
        ) VALUES (
            auth_user.id,
            COALESCE(
                (auth_user.raw_user_meta_data->>'school_id')::UUID,
                NULL
            ),
            COALESCE(
                auth_user.raw_user_meta_data->>'first_name',
                ''
            ),
            COALESCE(
                auth_user.raw_user_meta_data->>'last_name',
                ''
            ),
            auth_user.email,
            COALESCE(
                auth_user.raw_user_meta_data->>'role',
                'Parent'
            ),
            COALESCE(auth_user.email_confirmed_at IS NOT NULL, false),
            auth_user.created_at,
            auth_user.raw_user_meta_data,
            true
        ) ON CONFLICT (id) DO NOTHING;
    END LOOP;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- To sync existing users (run once if needed):
-- SELECT public.sync_existing_auth_users();

-- ==========================================
-- MIGRATION: Table Structure Updates
-- Date: 2025-09-29
-- Description: Adding missing tables and fields for complete enrollment system
-- ==========================================

-- ==========================================
-- PART 1: ALTER EXISTING TABLES
-- ==========================================

-- 1.0 Remove redundant fillout_form_url column from FORM_TEMPLATES table
ALTER TABLE form_templates DROP COLUMN IF EXISTS fillout_form_url;

-- 1.1 Add new columns to FORM_SUBMISSIONS table
ALTER TABLE form_submissions
ADD COLUMN IF NOT EXISTS edit_link TEXT,
ADD COLUMN IF NOT EXISTS pdf_link TEXT;

COMMENT ON COLUMN form_submissions.edit_link IS 'Link to edit the form submission in Fillout';
COMMENT ON COLUMN form_submissions.pdf_link IS 'Link to PDF version of the form submission';

-- 1.2 Add new columns to STUDENT_FORM_ASSIGNMENTS table
ALTER TABLE student_form_assignments
ADD COLUMN IF NOT EXISTS recent_edit_link TEXT,
ADD COLUMN IF NOT EXISTS recent_pdf_link TEXT;

COMMENT ON COLUMN student_form_assignments.recent_edit_link IS 'Most recent edit link from latest submission';
COMMENT ON COLUMN student_form_assignments.recent_pdf_link IS 'Most recent PDF link from latest submission';

-- 1.3 Add approval workflow columns to ENROLLMENTS table
ALTER TABLE enrollments
ADD COLUMN IF NOT EXISTS admin_approval_status VARCHAR(20) DEFAULT 'pending',
ADD COLUMN IF NOT EXISTS approved_at TIMESTAMP,
ADD COLUMN IF NOT EXISTS approved_by UUID REFERENCES users(id),
ADD COLUMN IF NOT EXISTS approval_notes TEXT,
ADD COLUMN IF NOT EXISTS forms_locked_at TIMESTAMP;

-- Add check constraint for approval status
ALTER TABLE enrollments DROP CONSTRAINT IF EXISTS check_admin_approval_status;
ALTER TABLE enrollments ADD CONSTRAINT check_admin_approval_status
CHECK (admin_approval_status IN ('pending', 'approved', 'rejected', 'needs_revision'));

COMMENT ON COLUMN enrollments.admin_approval_status IS 'Admin approval status for the enrollment';
COMMENT ON COLUMN enrollments.approved_at IS 'Timestamp when enrollment was approved';
COMMENT ON COLUMN enrollments.approved_by IS 'User ID of admin who approved';
COMMENT ON COLUMN enrollments.approval_notes IS 'Admin notes regarding approval decision';
COMMENT ON COLUMN enrollments.forms_locked_at IS 'Timestamp when forms were locked after approval';

-- ==========================================
-- PART 2: UPDATE is_verified LOGIC FOR USERS
-- ==========================================

-- Drop the existing default constraint on is_verified
ALTER TABLE users ALTER COLUMN is_verified DROP DEFAULT;

-- Create a function to set is_verified based on role
CREATE OR REPLACE FUNCTION set_is_verified_based_on_role()
RETURNS TRIGGER AS $$
BEGIN
    -- Set is_verified based on role
    IF NEW.is_verified IS NULL THEN
        IF NEW.role = 'Parent' OR NEW.role = 'primary-parent' OR NEW.role = 'secondary-parent' THEN
            NEW.is_verified := true;
        ELSE
            NEW.is_verified := false;
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to automatically set is_verified for new users
DROP TRIGGER IF EXISTS set_is_verified_on_insert ON users;
CREATE TRIGGER set_is_verified_on_insert
    BEFORE INSERT ON users
    FOR EACH ROW
    EXECUTE FUNCTION set_is_verified_based_on_role();

-- Update existing Parent users to have is_verified = true if currently false or null
UPDATE users
SET is_verified = true
WHERE role IN ('Parent', 'primary-parent', 'secondary-parent')
AND (is_verified = false OR is_verified IS NULL);

-- ==========================================
-- PART 3: UPDATE TRIGGER FOR AUTH.USERS SYNC
-- ==========================================

-- Update the auth user sync function to respect role-based is_verified
CREATE OR REPLACE FUNCTION public.handle_new_auth_user()
RETURNS TRIGGER AS $$
BEGIN
    -- Insert new user into public.users table using auth user data
    INSERT INTO public.users (
        id,
        school_id,
        first_name,
        last_name,
        email,
        role,
        is_verified,
        created_at,
        metadata,
        is_active
    ) VALUES (
        NEW.id,
        COALESCE(
            (NEW.raw_user_meta_data->>'school_id')::UUID,
            NULL
        ),
        COALESCE(
            NEW.raw_user_meta_data->>'first_name',
            ''
        ),
        COALESCE(
            NEW.raw_user_meta_data->>'last_name',
            ''
        ),
        NEW.email,
        COALESCE(
            NEW.raw_user_meta_data->>'role',
            'Parent'
        ),
        -- Set is_verified based on role
        CASE
            WHEN COALESCE(NEW.raw_user_meta_data->>'role', 'Parent') IN ('Parent', 'primary-parent', 'secondary-parent')
            THEN true
            ELSE false
        END,
        NOW(),
        NEW.raw_user_meta_data,
        true
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ==========================================
-- PART 4: ADDITIONAL INDEXES FOR PERFORMANCE
-- ==========================================

-- Add indexes for new approval columns
CREATE INDEX IF NOT EXISTS idx_enrollments_admin_approval_status ON enrollments(admin_approval_status);
CREATE INDEX IF NOT EXISTS idx_enrollments_approved_by ON enrollments(approved_by);
CREATE INDEX IF NOT EXISTS idx_enrollments_approved_at ON enrollments(approved_at);

-- Add indexes for link columns
CREATE INDEX IF NOT EXISTS idx_student_form_assignments_recent_edit_link ON student_form_assignments(recent_edit_link) WHERE recent_edit_link IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_student_form_assignments_recent_pdf_link ON student_form_assignments(recent_pdf_link) WHERE recent_pdf_link IS NOT NULL;

-- ==========================================
-- MIGRATION COMPLETE
-- ==========================================




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


-- ==========================================
-- MIGRATION: Remove unused approval fields from enrollments
-- Date: 2025-10-06
-- Description: Removing approval workflow fields that are no longer needed
-- ==========================================

-- Remove approval workflow columns from enrollments table
ALTER TABLE enrollments DROP COLUMN IF EXISTS progress;
ALTER TABLE enrollments DROP COLUMN IF EXISTS submitted_at;
ALTER TABLE enrollments DROP COLUMN IF EXISTS approved_at;
ALTER TABLE enrollments DROP COLUMN IF EXISTS approved_by;
ALTER TABLE enrollments DROP COLUMN IF EXISTS approval_notes;
ALTER TABLE enrollments DROP COLUMN IF EXISTS forms_locked_at;

-- Remove admin_approval_status column and its constraint
ALTER TABLE enrollments DROP CONSTRAINT IF EXISTS check_admin_approval_status;
ALTER TABLE enrollments DROP COLUMN IF EXISTS admin_approval_status;

-- Remove indexes for dropped columns
DROP INDEX IF EXISTS idx_enrollments_admin_approval_status;
DROP INDEX IF EXISTS idx_enrollments_approved_by;
DROP INDEX IF EXISTS idx_enrollments_approved_at;

-- Migration complete
COMMENT ON TABLE enrollments IS 'Child enrollment records linking students to classrooms (simplified without approval workflow)';


-- ==========================================
-- MIGRATION: Remove children status constraint for flexible status values
-- Date: 2025-10-06
-- Description: Allowing admins to set any status value for children
-- ==========================================

-- Drop the existing check constraint on children.status
ALTER TABLE children DROP CONSTRAINT IF EXISTS check_status;

-- Migration complete - children.status now accepts any string value
COMMENT ON COLUMN children.status IS 'Child status - can be any value set by admin (e.g., active, inactive, graduated, withdrawn, archive, etc.)';
