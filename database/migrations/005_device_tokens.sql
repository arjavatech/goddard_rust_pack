-- Per-device FCM registration tokens.
-- One row per (user, browser/device). Token is globally unique because FCM mints
-- one token per Service Worker registration regardless of which user signs in;
-- ON CONFLICT (token) lets a re-login on the same browser claim the row.

CREATE TABLE IF NOT EXISTS device_tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token        TEXT NOT NULL UNIQUE,
    platform     TEXT NOT NULL DEFAULT 'web',
    user_agent   TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_device_tokens_user ON device_tokens(user_id);
