-- Migration 008: Create employees table for employee-specific profile data

CREATE TABLE IF NOT EXISTS employees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    school_id UUID NOT NULL REFERENCES schools(id),
    phone VARCHAR(50),
    address TEXT,
    employee_type VARCHAR(100),
    joined_on DATE,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP,
    CONSTRAINT unique_employee_per_school UNIQUE (user_id, school_id)
);

CREATE INDEX IF NOT EXISTS idx_employees_school_id ON employees(school_id);
CREATE INDEX IF NOT EXISTS idx_employees_user_id ON employees(user_id);
