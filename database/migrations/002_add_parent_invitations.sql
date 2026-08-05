-- Migration: Add user_invitations table for 7-day invite link support
-- Covers all invite types: parent, admin, teacher, superadmin.
-- The backend validates this token and issues a fresh Supabase signup link on click.

CREATE TABLE IF NOT EXISTS user_invitations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token       UUID UNIQUE NOT NULL DEFAULT gen_random_uuid(),
    user_email  VARCHAR(255) NOT NULL,
    role        VARCHAR(50) NOT NULL DEFAULT 'Parent',
    school_id   UUID REFERENCES schools(id) ON DELETE CASCADE,
    expires_at  TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '7 days'),
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_invitations_token ON user_invitations(token);
CREATE INDEX IF NOT EXISTS idx_user_invitations_email ON user_invitations(user_email);
