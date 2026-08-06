-- Migration 010: Create employee form submissions table (independent from parent form_submissions)

CREATE TABLE IF NOT EXISTS employee_form_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id),
    employee_id UUID NOT NULL REFERENCES employees(id),
    employee_form_assignment_id UUID NOT NULL REFERENCES employee_form_assignments(id),
    employee_form_template_id UUID NOT NULL REFERENCES employee_form_templates(id),
    fillout_submission_id VARCHAR(255) NOT NULL,
    form_data JSONB,
    metadata JSONB,
    status VARCHAR(50) DEFAULT 'pending',
    revision_number INTEGER DEFAULT 1,
    revision_reason VARCHAR(500),
    edit_link TEXT,
    pdf_link TEXT,
    submitted_at TIMESTAMP,
    processed_at TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP,
    CONSTRAINT unique_employee_fillout_submission UNIQUE (fillout_submission_id)
);

CREATE INDEX IF NOT EXISTS idx_efs_school_id ON employee_form_submissions(school_id);
CREATE INDEX IF NOT EXISTS idx_efs_employee_id ON employee_form_submissions(employee_id);
CREATE INDEX IF NOT EXISTS idx_efs_assignment_id ON employee_form_submissions(employee_form_assignment_id);
CREATE INDEX IF NOT EXISTS idx_efs_fillout_id ON employee_form_submissions(fillout_submission_id);
