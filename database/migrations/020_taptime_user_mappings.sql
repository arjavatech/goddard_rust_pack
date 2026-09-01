-- Manual, school-scoped links between Goddard identities and TapTime employees.
-- This is intentionally not an automatic-sync queue: a link exists only after a
-- SuperAdmin explicitly confirms it in the Goddard portal.
CREATE TABLE IF NOT EXISTS taptime_user_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    goddard_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    taptime_emp_id UUID NOT NULL,
    user_role TEXT NOT NULL CHECK (user_role IN ('Employee', 'Admin')),
    mapped_by UUID NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'unmapped')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_push_at TIMESTAMPTZ,
    last_push_error TEXT
);

-- A Goddard user and a TapTime employee may each have only one active link per school.
CREATE UNIQUE INDEX IF NOT EXISTS ux_taptime_mapping_goddard_user_active
    ON taptime_user_mappings (school_id, goddard_user_id) WHERE status = 'active';
CREATE UNIQUE INDEX IF NOT EXISTS ux_taptime_mapping_taptime_employee_active
    ON taptime_user_mappings (school_id, taptime_emp_id) WHERE status = 'active';
CREATE INDEX IF NOT EXISTS ix_taptime_mapping_school_active
    ON taptime_user_mappings (school_id) WHERE status = 'active';
