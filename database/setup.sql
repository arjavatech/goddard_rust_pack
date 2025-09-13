-- =============================================
-- The Goddard School Enrollment Management System
-- Complete Database Setup Script
-- Version: 2.0.0 - Production Ready with Full Audit System
-- =============================================

-- Drop existing schema if exists (USE WITH CAUTION!)
-- Uncomment the following lines if you want to reset the database
-- DROP SCHEMA IF EXISTS public CASCADE;
-- CREATE SCHEMA public;

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- =============================================
-- CORE TABLES
-- =============================================

-- Schools table (root of multi-tenancy)
CREATE TABLE IF NOT EXISTS schools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    subdomain VARCHAR(100) UNIQUE NOT NULL,
    settings JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID,
    updated_by UUID
);

-- Users table (parents, teachers, admins)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE,
    email VARCHAR(255) UNIQUE NOT NULL,
    role VARCHAR(50) NOT NULL CHECK (role IN ('parent', 'teacher', 'admin', 'super_admin')),
    metadata JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Add foreign key constraints for users table audit fields after users table is created
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_schools_created_by') THEN
        ALTER TABLE schools ADD CONSTRAINT fk_schools_created_by FOREIGN KEY (created_by) REFERENCES users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_schools_updated_by') THEN
        ALTER TABLE schools ADD CONSTRAINT fk_schools_updated_by FOREIGN KEY (updated_by) REFERENCES users(id);
    END IF;
END $$;

-- Parent additional emails table
CREATE TABLE IF NOT EXISTS parent_additional_emails (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    parent_id UUID REFERENCES users(id) ON DELETE CASCADE NOT NULL,
    email_address VARCHAR(255) NOT NULL,
    email_type VARCHAR(50) NOT NULL CHECK (email_type IN ('work', 'backup', 'emergency', 'additional')),
    is_verified BOOLEAN DEFAULT FALSE,
    notes TEXT,
    last_verified_at TIMESTAMP WITH TIME ZONE,
    bounce_count INTEGER DEFAULT 0,
    preference_types JSONB DEFAULT '{"enrollment": true, "notifications": true, "newsletters": true}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- =============================================
-- ENROLLMENT TABLES
-- =============================================

-- Children table
CREATE TABLE IF NOT EXISTS children (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id UUID REFERENCES users(id) ON DELETE CASCADE NOT NULL,
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    birth_date DATE NOT NULL,
    medical_info JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Classrooms table
CREATE TABLE IF NOT EXISTS classrooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    name VARCHAR(100) NOT NULL,
    age_group VARCHAR(50),
    capacity INTEGER NOT NULL,
    enrolled_count INTEGER DEFAULT 0,
    min_age_months INTEGER,
    max_age_months INTEGER,
    teacher_ratio DECIMAL(3,1),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Enrollments table (central process hub)
CREATE TABLE IF NOT EXISTS enrollments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    child_id UUID REFERENCES children(id) ON DELETE CASCADE NOT NULL,
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    classroom_id UUID REFERENCES classrooms(id),
    enrollment_number VARCHAR(50),
    academic_year VARCHAR(20),
    status VARCHAR(50) NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'under_review', 'approved', 'rejected', 'withdrawn', 'needs_revision')),
    admin_approval_status VARCHAR(20) DEFAULT 'pending'
        CHECK (admin_approval_status IN ('pending', 'approved', 'rejected', 'needs_revision')),
    progress JSONB DEFAULT '{}',
    start_date DATE,
    withdrawal_date DATE,
    withdrawal_reason TEXT,
    submitted_at TIMESTAMP WITH TIME ZONE,
    approved_at TIMESTAMP WITH TIME ZONE,
    approved_by UUID REFERENCES users(id),
    approval_notes TEXT,
    forms_locked_at TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- =============================================
-- FORM MANAGEMENT TABLES
-- =============================================

-- Form templates table (form registry)
CREATE TABLE IF NOT EXISTS form_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    form_name VARCHAR(255) NOT NULL,
    form_type VARCHAR(100),
    fillout_form_id VARCHAR(255),
    fillout_form_url TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'active', 'school_default', 'archive')),
    is_required BOOLEAN DEFAULT FALSE,
    display_order INTEGER DEFAULT 0,
    version INTEGER DEFAULT 1,
    expires_at TIMESTAMP WITH TIME ZONE,
    prerequisites UUID[],
    conditional_logic JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Class form overrides table
CREATE TABLE IF NOT EXISTS class_form_overrides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    classroom_id UUID REFERENCES classrooms(id) ON DELETE CASCADE NOT NULL,
    form_template_id UUID REFERENCES form_templates(id) ON DELETE CASCADE NOT NULL,
    action VARCHAR(20) NOT NULL CHECK (action IN ('include', 'exclude')),
    is_required BOOLEAN,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Student form assignments table (materialized assignments)
CREATE TABLE IF NOT EXISTS student_form_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    enrollment_id UUID REFERENCES enrollments(id) ON DELETE CASCADE NOT NULL,
    child_id UUID REFERENCES children(id) ON DELETE CASCADE NOT NULL,
    form_template_id UUID REFERENCES form_templates(id) ON DELETE CASCADE NOT NULL,
    assignment_source VARCHAR(50) NOT NULL
        CHECK (assignment_source IN ('school_default', 'class_override', 'individual')),
    is_required BOOLEAN DEFAULT FALSE,
    assigned_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Form submissions table
CREATE TABLE IF NOT EXISTS form_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    enrollment_id UUID REFERENCES enrollments(id) ON DELETE CASCADE NOT NULL,
    student_form_assignment_id UUID REFERENCES student_form_assignments(id),
    form_template_id UUID REFERENCES form_templates(id) NOT NULL,
    fillout_submission_id VARCHAR(255) UNIQUE,
    form_data JSONB NOT NULL DEFAULT '{}',
    metadata JSONB DEFAULT '{}',
    submitted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- =============================================
-- DOCUMENT MANAGEMENT
-- =============================================

-- Documents table
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    enrollment_id UUID REFERENCES enrollments(id) ON DELETE CASCADE NOT NULL,
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    document_type VARCHAR(100) NOT NULL,
    storage_path TEXT NOT NULL,
    file_name VARCHAR(255) NOT NULL,
    file_size BIGINT,
    mime_type VARCHAR(100),
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    uploaded_by UUID REFERENCES users(id),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- =============================================
-- AUDIT & TRACKING TABLES
-- =============================================

-- Enrollment approval audit trail
CREATE TABLE IF NOT EXISTS enrollment_approval_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    enrollment_id UUID REFERENCES enrollments(id) ON DELETE CASCADE NOT NULL,
    admin_id UUID REFERENCES users(id) NOT NULL,
    action VARCHAR(20) NOT NULL
        CHECK (action IN ('approve', 'reject', 'request_revision', 'lock_forms', 'unlock_forms')),
    previous_status VARCHAR(20),
    new_status VARCHAR(20) NOT NULL,
    notes TEXT,
    affected_forms JSONB DEFAULT '[]',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Enrollment communications table
CREATE TABLE IF NOT EXISTS enrollment_communications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    enrollment_id UUID REFERENCES enrollments(id) ON DELETE CASCADE NOT NULL,
    communication_type VARCHAR(50) NOT NULL
        CHECK (communication_type IN ('email', 'sms', 'in_app', 'notification')),
    subject TEXT,
    content TEXT,
    sent_to JSONB DEFAULT '[]',
    sent_at TIMESTAMP WITH TIME ZONE,
    opened_at TIMESTAMP WITH TIME ZONE,
    clicked_at TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- =============================================
-- WAITLIST MANAGEMENT
-- =============================================

-- Waitlist table
CREATE TABLE IF NOT EXISTS waitlist (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE NOT NULL,
    child_id UUID REFERENCES children(id) ON DELETE CASCADE NOT NULL,
    classroom_id UUID REFERENCES classrooms(id),
    position INTEGER NOT NULL,
    priority_level VARCHAR(20) DEFAULT 'standard'
        CHECK (priority_level IN ('sibling', 'employee', 'standard', 'vip')),
    requested_date DATE NOT NULL,
    notes TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- =============================================
-- INDEXES FOR PERFORMANCE
-- =============================================

-- Create indexes only if they don't exist
DO $$
BEGIN
    -- School indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_schools_subdomain') THEN
        CREATE INDEX idx_schools_subdomain ON schools(subdomain) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_schools_active') THEN
        CREATE INDEX idx_schools_active ON schools(is_active);
    END IF;

    -- User indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_school_id') THEN
        CREATE INDEX idx_users_school_id ON users(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_email_lower') THEN
        CREATE INDEX idx_users_email_lower ON users(LOWER(email)) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_role') THEN
        CREATE INDEX idx_users_role ON users(role) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_active') THEN
        CREATE INDEX idx_users_active ON users(is_active);
    END IF;

    -- Parent email indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_parent_emails_parent') THEN
        CREATE INDEX idx_parent_emails_parent ON parent_additional_emails(parent_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_parent_emails_school') THEN
        CREATE INDEX idx_parent_emails_school ON parent_additional_emails(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_parent_emails_active') THEN
        CREATE INDEX idx_parent_emails_active ON parent_additional_emails(parent_id, is_verified) WHERE is_active = TRUE;
    END IF;

    -- Children indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_children_parent') THEN
        CREATE INDEX idx_children_parent ON children(parent_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_children_school') THEN
        CREATE INDEX idx_children_school ON children(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_children_active') THEN
        CREATE INDEX idx_children_active ON children(is_active);
    END IF;

    -- Enrollment indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_enrollments_child') THEN
        CREATE INDEX idx_enrollments_child ON enrollments(child_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_enrollments_school') THEN
        CREATE INDEX idx_enrollments_school ON enrollments(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_enrollments_status_school') THEN
        CREATE INDEX idx_enrollments_status_school ON enrollments(school_id, status) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_enrollments_classroom') THEN
        CREATE INDEX idx_enrollments_classroom ON enrollments(classroom_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_enrollments_approval_status') THEN
        CREATE INDEX idx_enrollments_approval_status ON enrollments(admin_approval_status) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_enrollments_active') THEN
        CREATE INDEX idx_enrollments_active ON enrollments(is_active);
    END IF;

    -- Form template indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_templates_school') THEN
        CREATE INDEX idx_form_templates_school ON form_templates(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_templates_status') THEN
        CREATE INDEX idx_form_templates_status ON form_templates(status) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_templates_school_status') THEN
        CREATE INDEX idx_form_templates_school_status ON form_templates(school_id, status) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_templates_active') THEN
        CREATE INDEX idx_form_templates_active ON form_templates(is_active);
    END IF;

    -- Form assignment indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_assignments_enrollment') THEN
        CREATE INDEX idx_form_assignments_enrollment ON student_form_assignments(enrollment_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_assignments_child') THEN
        CREATE INDEX idx_form_assignments_child ON student_form_assignments(child_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_assignments_template') THEN
        CREATE INDEX idx_form_assignments_template ON student_form_assignments(form_template_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_assignments_active') THEN
        CREATE INDEX idx_form_assignments_active ON student_form_assignments(is_active);
    END IF;

    -- Form submission indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_submissions_enrollment') THEN
        CREATE INDEX idx_form_submissions_enrollment ON form_submissions(enrollment_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_submissions_template') THEN
        CREATE INDEX idx_form_submissions_template ON form_submissions(form_template_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_submissions_fillout_id') THEN
        CREATE INDEX idx_form_submissions_fillout_id ON form_submissions(fillout_submission_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_form_submissions_active') THEN
        CREATE INDEX idx_form_submissions_active ON form_submissions(is_active);
    END IF;

    -- Document indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_documents_enrollment') THEN
        CREATE INDEX idx_documents_enrollment ON documents(enrollment_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_documents_school') THEN
        CREATE INDEX idx_documents_school ON documents(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_documents_active') THEN
        CREATE INDEX idx_documents_active ON documents(is_active);
    END IF;

    -- Classroom indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_classrooms_school') THEN
        CREATE INDEX idx_classrooms_school ON classrooms(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_classrooms_school_active') THEN
        CREATE INDEX idx_classrooms_school_active ON classrooms(school_id, is_active);
    END IF;

    -- Waitlist indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_waitlist_school') THEN
        CREATE INDEX idx_waitlist_school ON waitlist(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_waitlist_child') THEN
        CREATE INDEX idx_waitlist_child ON waitlist(child_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_waitlist_position') THEN
        CREATE INDEX idx_waitlist_position ON waitlist(school_id, classroom_id, position) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_waitlist_active') THEN
        CREATE INDEX idx_waitlist_active ON waitlist(is_active);
    END IF;

    -- Audit indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_approval_audit_enrollment') THEN
        CREATE INDEX idx_approval_audit_enrollment ON enrollment_approval_audit(enrollment_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_approval_audit_school') THEN
        CREATE INDEX idx_approval_audit_school ON enrollment_approval_audit(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_approval_audit_admin') THEN
        CREATE INDEX idx_approval_audit_admin ON enrollment_approval_audit(admin_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_approval_audit_active') THEN
        CREATE INDEX idx_approval_audit_active ON enrollment_approval_audit(is_active);
    END IF;

    -- Communication indexes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_communications_enrollment') THEN
        CREATE INDEX idx_communications_enrollment ON enrollment_communications(enrollment_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_communications_school') THEN
        CREATE INDEX idx_communications_school ON enrollment_communications(school_id) WHERE is_active = TRUE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_communications_active') THEN
        CREATE INDEX idx_communications_active ON enrollment_communications(is_active);
    END IF;

    -- Audit field indexes for tracking changes
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_schools_created_by') THEN
        CREATE INDEX idx_schools_created_by ON schools(created_by);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_schools_updated_by') THEN
        CREATE INDEX idx_schools_updated_by ON schools(updated_by);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_created_by') THEN
        CREATE INDEX idx_users_created_by ON users(created_by);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_users_updated_by') THEN
        CREATE INDEX idx_users_updated_by ON users(updated_by);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_children_created_by') THEN
        CREATE INDEX idx_children_created_by ON children(created_by);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_children_updated_by') THEN
        CREATE INDEX idx_children_updated_by ON children(updated_by);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_enrollments_created_by') THEN
        CREATE INDEX idx_enrollments_created_by ON enrollments(created_by);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_enrollments_updated_by') THEN
        CREATE INDEX idx_enrollments_updated_by ON enrollments(updated_by);
    END IF;
END $$;

-- =============================================
-- ROW LEVEL SECURITY (RLS) POLICIES
-- =============================================

-- Enable RLS on all tables
ALTER TABLE schools ENABLE ROW LEVEL SECURITY;
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE parent_additional_emails ENABLE ROW LEVEL SECURITY;
ALTER TABLE children ENABLE ROW LEVEL SECURITY;
ALTER TABLE classrooms ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollments ENABLE ROW LEVEL SECURITY;
ALTER TABLE form_templates ENABLE ROW LEVEL SECURITY;
ALTER TABLE class_form_overrides ENABLE ROW LEVEL SECURITY;
ALTER TABLE student_form_assignments ENABLE ROW LEVEL SECURITY;
ALTER TABLE form_submissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollment_approval_audit ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollment_communications ENABLE ROW LEVEL SECURITY;
ALTER TABLE waitlist ENABLE ROW LEVEL SECURITY;

-- =============================================
-- TRIGGERS FOR AUDIT FIELDS
-- =============================================

-- Function to update updated_at and updated_by timestamp
CREATE OR REPLACE FUNCTION update_audit_fields()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    -- Set updated_by from current user context if available
    IF current_setting('app.current_user_id', true) IS NOT NULL THEN
        NEW.updated_by = current_setting('app.current_user_id')::UUID;
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Function to set created_by on insert
CREATE OR REPLACE FUNCTION set_created_by()
RETURNS TRIGGER AS $$
BEGIN
    -- Set created_by from current user context if available
    IF current_setting('app.current_user_id', true) IS NOT NULL THEN
        NEW.created_by = current_setting('app.current_user_id')::UUID;
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create audit triggers for all tables (drop if exists first)
DO $$
BEGIN
    -- Schools triggers
    DROP TRIGGER IF EXISTS update_schools_audit ON schools;
    DROP TRIGGER IF EXISTS set_schools_created_by ON schools;
    CREATE TRIGGER update_schools_audit BEFORE UPDATE ON schools FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_schools_created_by BEFORE INSERT ON schools FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Users triggers
    DROP TRIGGER IF EXISTS update_users_audit ON users;
    DROP TRIGGER IF EXISTS set_users_created_by ON users;
    CREATE TRIGGER update_users_audit BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_users_created_by BEFORE INSERT ON users FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Parent additional emails triggers
    DROP TRIGGER IF EXISTS update_parent_additional_emails_audit ON parent_additional_emails;
    DROP TRIGGER IF EXISTS set_parent_additional_emails_created_by ON parent_additional_emails;
    CREATE TRIGGER update_parent_additional_emails_audit BEFORE UPDATE ON parent_additional_emails FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_parent_additional_emails_created_by BEFORE INSERT ON parent_additional_emails FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Children triggers
    DROP TRIGGER IF EXISTS update_children_audit ON children;
    DROP TRIGGER IF EXISTS set_children_created_by ON children;
    CREATE TRIGGER update_children_audit BEFORE UPDATE ON children FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_children_created_by BEFORE INSERT ON children FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Classrooms triggers
    DROP TRIGGER IF EXISTS update_classrooms_audit ON classrooms;
    DROP TRIGGER IF EXISTS set_classrooms_created_by ON classrooms;
    CREATE TRIGGER update_classrooms_audit BEFORE UPDATE ON classrooms FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_classrooms_created_by BEFORE INSERT ON classrooms FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Enrollments triggers
    DROP TRIGGER IF EXISTS update_enrollments_audit ON enrollments;
    DROP TRIGGER IF EXISTS set_enrollments_created_by ON enrollments;
    CREATE TRIGGER update_enrollments_audit BEFORE UPDATE ON enrollments FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_enrollments_created_by BEFORE INSERT ON enrollments FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Form templates triggers
    DROP TRIGGER IF EXISTS update_form_templates_audit ON form_templates;
    DROP TRIGGER IF EXISTS set_form_templates_created_by ON form_templates;
    CREATE TRIGGER update_form_templates_audit BEFORE UPDATE ON form_templates FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_form_templates_created_by BEFORE INSERT ON form_templates FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Class form overrides triggers
    DROP TRIGGER IF EXISTS update_class_form_overrides_audit ON class_form_overrides;
    DROP TRIGGER IF EXISTS set_class_form_overrides_created_by ON class_form_overrides;
    CREATE TRIGGER update_class_form_overrides_audit BEFORE UPDATE ON class_form_overrides FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_class_form_overrides_created_by BEFORE INSERT ON class_form_overrides FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Student form assignments triggers
    DROP TRIGGER IF EXISTS update_student_form_assignments_audit ON student_form_assignments;
    DROP TRIGGER IF EXISTS set_student_form_assignments_created_by ON student_form_assignments;
    CREATE TRIGGER update_student_form_assignments_audit BEFORE UPDATE ON student_form_assignments FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_student_form_assignments_created_by BEFORE INSERT ON student_form_assignments FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Form submissions triggers
    DROP TRIGGER IF EXISTS update_form_submissions_audit ON form_submissions;
    DROP TRIGGER IF EXISTS set_form_submissions_created_by ON form_submissions;
    CREATE TRIGGER update_form_submissions_audit BEFORE UPDATE ON form_submissions FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_form_submissions_created_by BEFORE INSERT ON form_submissions FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Documents triggers
    DROP TRIGGER IF EXISTS update_documents_audit ON documents;
    DROP TRIGGER IF EXISTS set_documents_created_by ON documents;
    CREATE TRIGGER update_documents_audit BEFORE UPDATE ON documents FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_documents_created_by BEFORE INSERT ON documents FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Enrollment approval audit triggers
    DROP TRIGGER IF EXISTS update_enrollment_approval_audit_audit ON enrollment_approval_audit;
    DROP TRIGGER IF EXISTS set_enrollment_approval_audit_created_by ON enrollment_approval_audit;
    CREATE TRIGGER update_enrollment_approval_audit_audit BEFORE UPDATE ON enrollment_approval_audit FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_enrollment_approval_audit_created_by BEFORE INSERT ON enrollment_approval_audit FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Enrollment communications triggers
    DROP TRIGGER IF EXISTS update_enrollment_communications_audit ON enrollment_communications;
    DROP TRIGGER IF EXISTS set_enrollment_communications_created_by ON enrollment_communications;
    CREATE TRIGGER update_enrollment_communications_audit BEFORE UPDATE ON enrollment_communications FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_enrollment_communications_created_by BEFORE INSERT ON enrollment_communications FOR EACH ROW EXECUTE FUNCTION set_created_by();

    -- Waitlist triggers
    DROP TRIGGER IF EXISTS update_waitlist_audit ON waitlist;
    DROP TRIGGER IF EXISTS set_waitlist_created_by ON waitlist;
    CREATE TRIGGER update_waitlist_audit BEFORE UPDATE ON waitlist FOR EACH ROW EXECUTE FUNCTION update_audit_fields();
    CREATE TRIGGER set_waitlist_created_by BEFORE INSERT ON waitlist FOR EACH ROW EXECUTE FUNCTION set_created_by();
END $$;

-- =============================================
-- FUNCTIONS FOR BUSINESS LOGIC
-- =============================================

-- Function to update classroom enrollment count (updated for is_active)
CREATE OR REPLACE FUNCTION update_classroom_enrollment_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' OR TG_OP = 'UPDATE' THEN
        UPDATE classrooms
        SET enrolled_count = (
            SELECT COUNT(*)
            FROM enrollments
            WHERE classroom_id = NEW.classroom_id
            AND status = 'approved'
            AND is_active = TRUE
        )
        WHERE id = NEW.classroom_id;
    END IF;

    IF TG_OP = 'DELETE' OR TG_OP = 'UPDATE' THEN
        IF OLD.classroom_id IS NOT NULL THEN
            UPDATE classrooms
            SET enrolled_count = (
                SELECT COUNT(*)
                FROM enrollments
                WHERE classroom_id = OLD.classroom_id
                AND status = 'approved'
                AND is_active = TRUE
            )
            WHERE id = OLD.classroom_id;
        END IF;
    END IF;

    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Create or replace trigger
DROP TRIGGER IF EXISTS update_classroom_count ON enrollments;
CREATE TRIGGER update_classroom_count
AFTER INSERT OR UPDATE OR DELETE ON enrollments
FOR EACH ROW EXECUTE FUNCTION update_classroom_enrollment_count();

-- Function to generate enrollment number
CREATE OR REPLACE FUNCTION generate_enrollment_number()
RETURNS TRIGGER AS $$
DECLARE
    v_year VARCHAR(4);
    v_sequence INTEGER;
    v_enrollment_number VARCHAR(50);
BEGIN
    IF NEW.enrollment_number IS NULL THEN
        v_year := EXTRACT(YEAR FROM NOW())::VARCHAR;

        SELECT COALESCE(MAX(
            CAST(
                SUBSTRING(enrollment_number FROM '[0-9]+$') AS INTEGER
            )
        ), 0) + 1
        INTO v_sequence
        FROM enrollments
        WHERE school_id = NEW.school_id
        AND enrollment_number LIKE 'ENR-' || v_year || '-%'
        AND is_active = TRUE;

        v_enrollment_number := 'ENR-' || v_year || '-' || LPAD(v_sequence::VARCHAR, 5, '0');
        NEW.enrollment_number := v_enrollment_number;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create or replace trigger
DROP TRIGGER IF EXISTS generate_enrollment_number_trigger ON enrollments;
CREATE TRIGGER generate_enrollment_number_trigger
BEFORE INSERT ON enrollments
FOR EACH ROW EXECUTE FUNCTION generate_enrollment_number();

-- Function to soft delete (set is_active = FALSE)
CREATE OR REPLACE FUNCTION soft_delete(table_name TEXT, record_id UUID, user_id UUID DEFAULT NULL)
RETURNS BOOLEAN AS $$
DECLARE
    sql_query TEXT;
BEGIN
    sql_query := format('UPDATE %I SET is_active = FALSE, updated_at = NOW()', table_name);

    IF user_id IS NOT NULL THEN
        sql_query := sql_query || format(', updated_by = %L', user_id);
    END IF;

    sql_query := sql_query || format(' WHERE id = %L AND is_active = TRUE', record_id);

    EXECUTE sql_query;

    RETURN FOUND;
END;
$$ LANGUAGE plpgsql;

-- Function to get audit trail for any record
CREATE OR REPLACE FUNCTION get_audit_trail(p_table_name TEXT, p_record_id UUID)
RETURNS TABLE (
    table_name TEXT,
    record_id UUID,
    created_at TIMESTAMP WITH TIME ZONE,
    created_by UUID,
    updated_at TIMESTAMP WITH TIME ZONE,
    updated_by UUID,
    is_active BOOLEAN
) AS $$
DECLARE
    sql_query TEXT;
BEGIN
    sql_query := format(
        'SELECT %L as table_name, id as record_id, created_at, created_by, updated_at, updated_by, is_active
         FROM %I WHERE id = %L',
        p_table_name, p_table_name, p_record_id
    );

    RETURN QUERY EXECUTE sql_query;
END;
$$ LANGUAGE plpgsql;

-- Function to restore soft deleted record
CREATE OR REPLACE FUNCTION restore_record(p_table_name TEXT, p_record_id UUID, p_user_id UUID DEFAULT NULL)
RETURNS BOOLEAN AS $$
DECLARE
    sql_query TEXT;
BEGIN
    sql_query := format('UPDATE %I SET is_active = TRUE, updated_at = NOW()', p_table_name);

    IF p_user_id IS NOT NULL THEN
        sql_query := sql_query || format(', updated_by = %L', p_user_id);
    END IF;

    sql_query := sql_query || format(' WHERE id = %L AND is_active = FALSE', p_record_id);

    EXECUTE sql_query;

    RETURN FOUND;
END;
$$ LANGUAGE plpgsql;

-- =============================================
-- INITIAL DATA & CONSTRAINTS
-- =============================================

-- Add check constraint for email format (if not exists)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'check_email_format') THEN
        ALTER TABLE users ADD CONSTRAINT check_email_format
            CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'check_additional_email_format') THEN
        ALTER TABLE parent_additional_emails ADD CONSTRAINT check_additional_email_format
            CHECK (email_address ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$');
    END IF;
END $$;

-- Add constraint to ensure classroom capacity is not exceeded
CREATE OR REPLACE FUNCTION check_classroom_capacity()
RETURNS TRIGGER AS $$
DECLARE
    v_capacity INTEGER;
    v_enrolled INTEGER;
BEGIN
    IF NEW.status = 'approved' AND NEW.classroom_id IS NOT NULL AND NEW.is_active = TRUE THEN
        SELECT capacity, enrolled_count
        INTO v_capacity, v_enrolled
        FROM classrooms
        WHERE id = NEW.classroom_id AND is_active = TRUE;

        IF v_enrolled >= v_capacity THEN
            RAISE EXCEPTION 'Classroom capacity exceeded';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create or replace trigger
DROP TRIGGER IF EXISTS check_capacity_before_approval ON enrollments;
CREATE TRIGGER check_capacity_before_approval
BEFORE INSERT OR UPDATE ON enrollments
FOR EACH ROW EXECUTE FUNCTION check_classroom_capacity();

-- Add constraint for date validation in enrollments (if not exists)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'check_dates') THEN
        ALTER TABLE enrollments ADD CONSTRAINT check_dates
            CHECK (start_date IS NULL OR withdrawal_date IS NULL OR start_date < withdrawal_date);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'check_age_range') THEN
        ALTER TABLE classrooms ADD CONSTRAINT check_age_range
            CHECK (min_age_months IS NULL OR max_age_months IS NULL OR min_age_months < max_age_months);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'unique_form_name_version_per_school') THEN
        ALTER TABLE form_templates ADD CONSTRAINT unique_form_name_version_per_school
            UNIQUE (school_id, form_name, version);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'unique_waitlist_position') THEN
        ALTER TABLE waitlist ADD CONSTRAINT unique_waitlist_position
            UNIQUE (school_id, classroom_id, position);
    END IF;
END $$;

-- =============================================
-- COMMENTS FOR DOCUMENTATION
-- =============================================

-- Add table comments
COMMENT ON TABLE schools IS 'Root table for multi-tenant architecture with full audit trail';
COMMENT ON TABLE users IS 'All system users with comprehensive audit tracking';
COMMENT ON TABLE parent_additional_emails IS 'Additional email addresses with audit trail';
COMMENT ON TABLE children IS 'Children information with audit tracking';
COMMENT ON TABLE classrooms IS 'Classroom definitions with audit trail';
COMMENT ON TABLE enrollments IS 'Central enrollment process with audit trail';
COMMENT ON TABLE form_templates IS 'Form registry with audit trail';
COMMENT ON TABLE class_form_overrides IS 'Classroom form overrides with audit trail';
COMMENT ON TABLE student_form_assignments IS 'Form assignments with audit trail';
COMMENT ON TABLE form_submissions IS 'Form submissions with audit trail';
COMMENT ON TABLE documents IS 'Document metadata with audit trail';
COMMENT ON TABLE enrollment_approval_audit IS 'Approval audit trail with full audit';
COMMENT ON TABLE enrollment_communications IS 'Communication tracking with audit';
COMMENT ON TABLE waitlist IS 'Waitlist management with audit trail';

-- Add column comments for key audit fields
COMMENT ON COLUMN schools.created_by IS 'User who created this record';
COMMENT ON COLUMN schools.updated_by IS 'User who last updated this record';
COMMENT ON COLUMN schools.is_active IS 'Soft delete flag - FALSE means deleted';

-- =============================================
-- SETUP COMPLETE MESSAGE
-- =============================================

DO $$
BEGIN
    RAISE NOTICE '🎉 Goddard School Database Setup Complete!';
    RAISE NOTICE '✅ All 14 tables created with full audit trails';
    RAISE NOTICE '✅ 40+ performance indexes applied';
    RAISE NOTICE '✅ Business logic functions and triggers installed';
    RAISE NOTICE '✅ Row Level Security enabled';
    RAISE NOTICE '✅ Soft delete system operational';
    RAISE NOTICE '🚀 Database is production ready!';
END $$;