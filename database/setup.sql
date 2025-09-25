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
    fillout_form_id VARCHAR(255),
    fillout_form_url TEXT,
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