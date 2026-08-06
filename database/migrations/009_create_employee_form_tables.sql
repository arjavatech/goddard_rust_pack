-- Migration 009: Create employee form templates and assignments tables

CREATE TABLE IF NOT EXISTS employee_form_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id),
    form_name VARCHAR(255) NOT NULL,
    form_type VARCHAR(100),
    fillout_form_id VARCHAR(255),
    due_date DATE,
    status VARCHAR(50) DEFAULT 'active',
    is_required BOOLEAN DEFAULT false,
    display_order INTEGER,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS unique_active_employee_form_name_per_school
    ON employee_form_templates(school_id, form_name) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_eft_school_id ON employee_form_templates(school_id);

CREATE TABLE IF NOT EXISTS employee_form_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id),
    employee_id UUID NOT NULL REFERENCES employees(id),
    user_id UUID NOT NULL REFERENCES users(id),
    employee_form_template_id UUID NOT NULL REFERENCES employee_form_templates(id),
    assignment_source VARCHAR(50) DEFAULT 'manual',
    status VARCHAR(50) DEFAULT 'incomplete',
    is_required BOOLEAN DEFAULT false,
    assigned_by UUID REFERENCES users(id),
    assigned_at TIMESTAMP DEFAULT NOW(),
    approved_by UUID REFERENCES users(id),
    approved_on TIMESTAMP,
    notes TEXT,
    recent_edit_link TEXT,
    recent_pdf_link TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP,
    CONSTRAINT unique_employee_form_assignment UNIQUE (employee_id, employee_form_template_id)
);

CREATE INDEX IF NOT EXISTS idx_efa_school_id ON employee_form_assignments(school_id);
CREATE INDEX IF NOT EXISTS idx_efa_employee_id ON employee_form_assignments(employee_id);
CREATE INDEX IF NOT EXISTS idx_efa_template_id ON employee_form_assignments(employee_form_template_id);
