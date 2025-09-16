-- Goddard School Enrollment Management System - Database Setup
-- Database Schema for 11 tables from System Architecture Design v2.2

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. SCHOOLS Table
CREATE TABLE schools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    subdomain VARCHAR(100) UNIQUE NOT NULL,
    settings JSONB,
    created_at TIMESTAMP DEFAULT NOW()
);

-- 2. USERS Table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    invite_id UUID,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    role VARCHAR(50) NOT NULL,
    id_signed BOOLEAN DEFAULT false,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- 3. PARENT_ADDITIONAL_EMAILS Table
CREATE TABLE parent_additional_emails (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    parent_id UUID NOT NULL REFERENCES users(id),
    email_address VARCHAR(255) NOT NULL,
    email_type VARCHAR(50),
    is_verified BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,
    added_by UUID REFERENCES users(id),
    notes TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);

-- 4. CHILDREN Table
CREATE TABLE children (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent_id UUID NOT NULL REFERENCES users(id),
    secondary_parent_id UUID REFERENCES users(id),
    school_id UUID NOT NULL REFERENCES schools(id),
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    birth_date DATE,
    gender VARCHAR(20),
    status VARCHAR(50) DEFAULT 'active'
);

-- 5. CLASSROOMS Table
CREATE TABLE classrooms (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    name VARCHAR(255) NOT NULL,
    age_group VARCHAR(50),
    capacity INTEGER,
    enrolled_count INTEGER DEFAULT 0
);

-- 6. FORM_TEMPLATES Table
CREATE TABLE form_templates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    form_name VARCHAR(255) NOT NULL,
    form_type VARCHAR(100),
    fillout_form_id VARCHAR(255),
    fillout_form_url TEXT,
    status VARCHAR(50),
    is_required BOOLEAN DEFAULT false,
    display_order INTEGER,
    created_at TIMESTAMP DEFAULT NOW()
);

-- 7. ENROLLMENTS Table
CREATE TABLE enrollments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    child_id UUID NOT NULL REFERENCES children(id),
    school_id UUID NOT NULL REFERENCES schools(id),
    classroom_id UUID NOT NULL REFERENCES classrooms(id),
    status VARCHAR(50),
    application_status JSONB,
    progress JSONB,
    submitted_at TIMESTAMP
);

-- 8. CLASS_FORM_OVERRIDES Table
CREATE TABLE class_form_overrides (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    classroom_id UUID NOT NULL REFERENCES classrooms(id),
    form_template_id UUID NOT NULL REFERENCES form_templates(id),
    action VARCHAR(50),
    is_required BOOLEAN,
    created_at TIMESTAMP DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true
);

-- 9. STUDENT_FORM_ASSIGNMENTS Table
CREATE TABLE student_form_assignments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id UUID NOT NULL REFERENCES schools(id),
    enrollment_id UUID NOT NULL REFERENCES enrollments(id),
    child_id UUID NOT NULL REFERENCES children(id),
    form_template_id UUID NOT NULL REFERENCES form_templates(id),
    assignment_source VARCHAR(50),
    status VARCHAR(50) DEFAULT 'incomplete',
    is_required BOOLEAN DEFAULT false,
    assigned_at TIMESTAMP DEFAULT NOW()
);

-- 10. FORM_SUBMISSIONS Table
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
    processed_at TIMESTAMP
);

-- 11. DOCUMENTS Table
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
COMMENT ON TABLE parent_additional_emails IS 'Additional email addresses for parents';
COMMENT ON TABLE children IS 'Student/child records linked to parents';
COMMENT ON TABLE classrooms IS 'Physical or logical classroom groupings';
COMMENT ON TABLE form_templates IS 'Fillout.com form templates managed by schools';
COMMENT ON TABLE enrollments IS 'Child enrollment records linking students to classrooms';
COMMENT ON TABLE class_form_overrides IS 'Classroom-specific form requirements and overrides';
COMMENT ON TABLE student_form_assignments IS 'Individual form assignments to students';
COMMENT ON TABLE form_submissions IS 'Actual form submissions from Fillout.com webhooks';
COMMENT ON TABLE documents IS 'File attachments and documents related to enrollments';