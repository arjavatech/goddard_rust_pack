-- Migration 016: Goddard-side Tap-Time integration state.
-- Tap-Time remains the time-entry source of truth; this schema stores links,
-- safe synchronization work, and audit metadata only.

CREATE TABLE IF NOT EXISTS tap_time_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL UNIQUE REFERENCES schools(id) ON DELETE CASCADE,
    tap_company_id UUID NOT NULL UNIQUE,
    tap_company_name TEXT NOT NULL,
    tap_timezone TEXT,
    status VARCHAR(30) NOT NULL CHECK (status IN ('active', 'disconnected', 'error')),
    connected_by UUID NOT NULL REFERENCES users(id),
    connected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disconnected_by UUID REFERENCES users(id),
    disconnected_at TIMESTAMPTZ,
    last_health_check_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tap_time_employee_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    employee_id UUID NOT NULL UNIQUE REFERENCES employees(id) ON DELETE CASCADE,
    tap_company_id UUID NOT NULL,
    tap_employee_id UUID UNIQUE,
    sync_status VARCHAR(30) NOT NULL CHECK (sync_status IN ('pending', 'synced', 'pending_pin', 'failed', 'inactive')),
    last_synced_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Administrators are users, not rows in the Goddard employees table.  Keep a
-- separate stable mapping so their Tap-Time record can carry is_admin = 1/2.
CREATE TABLE IF NOT EXISTS tap_time_user_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    tap_company_id UUID NOT NULL,
    tap_employee_id UUID UNIQUE,
    sync_status VARCHAR(30) NOT NULL CHECK (sync_status IN ('pending', 'synced', 'pending_pin', 'failed', 'inactive')),
    last_synced_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tap_time_sync_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    employee_id UUID REFERENCES employees(id) ON DELETE CASCADE,
    operation VARCHAR(50) NOT NULL CHECK (operation IN ('employee_upsert', 'employee_deactivate')),
    payload JSONB NOT NULL,
    idempotency_key UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    status VARCHAR(30) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS tap_time_audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    actor_user_id UUID REFERENCES users(id),
    action VARCHAR(100) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID,
    tap_entity_id UUID,
    request_id UUID,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tap_time_employee_links_school
    ON tap_time_employee_links (school_id, sync_status);
CREATE INDEX IF NOT EXISTS idx_tap_time_user_links_school
    ON tap_time_user_links (school_id, sync_status);
CREATE INDEX IF NOT EXISTS idx_tap_time_sync_outbox_ready
    ON tap_time_sync_outbox (status, next_attempt_at)
    WHERE status IN ('pending', 'failed');
CREATE INDEX IF NOT EXISTS idx_tap_time_audit_events_school
    ON tap_time_audit_events (school_id, created_at DESC);
