-- TapTime identity belongs to the existing Goddard user.  This replaces the
-- temporary taptime_user_mappings table used by the retired manual mapper.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS taptime_employee_id UUID,
    ADD COLUMN IF NOT EXISTS taptime_pin VARCHAR(10);

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_taptime_pin_numeric;
ALTER TABLE users
    ADD CONSTRAINT users_taptime_pin_numeric
    CHECK (taptime_pin IS NULL OR taptime_pin ~ '^[0-9]{4,10}$');

CREATE UNIQUE INDEX IF NOT EXISTS ux_users_taptime_employee_id
    ON users (taptime_employee_id)
    WHERE taptime_employee_id IS NOT NULL;

-- Preserve existing confirmed mappings before the obsolete table is removed.
DO $$
BEGIN
    IF to_regclass('public.taptime_user_mappings') IS NOT NULL THEN
        UPDATE users u
        SET taptime_employee_id = m.taptime_emp_id,
            updated_at = NOW()
        FROM taptime_user_mappings m
        WHERE m.goddard_user_id = u.id
          AND m.status = 'active'
          AND m.taptime_emp_id IS NOT NULL
          AND u.taptime_employee_id IS NULL;
    END IF;
END $$;

-- Drop only after the update above, so already-connected staff remain linked.
DROP TABLE IF EXISTS taptime_user_mappings;
