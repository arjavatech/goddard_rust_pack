-- Upgrade environments where an earlier experimental taptime_user_mappings
-- table already exists. Migration 020 uses CREATE TABLE IF NOT EXISTS, which
-- does not add columns to that existing table.
--
-- Existing legacy rows are retained but marked unmapped because they do not
-- contain the explicit Goddard-user identity required by manual mapping.

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS goddard_user_id UUID REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS taptime_emp_id UUID;

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS user_role TEXT;

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS mapped_by UUID;

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active';

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS last_push_at TIMESTAMPTZ;

ALTER TABLE taptime_user_mappings
    ADD COLUMN IF NOT EXISTS last_push_error TEXT;

UPDATE taptime_user_mappings
SET status = 'unmapped', updated_at = NOW()
WHERE goddard_user_id IS NULL OR taptime_emp_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ux_taptime_mapping_goddard_user_active
    ON taptime_user_mappings (school_id, goddard_user_id)
    WHERE status = 'active' AND goddard_user_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ux_taptime_mapping_taptime_employee_active
    ON taptime_user_mappings (school_id, taptime_emp_id)
    WHERE status = 'active' AND taptime_emp_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS ix_taptime_mapping_school_active
    ON taptime_user_mappings (school_id)
    WHERE status = 'active';
