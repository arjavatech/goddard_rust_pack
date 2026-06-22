-- Migration: 004_notifications
-- Adds in-app notifications table used by the bell-icon + drawer feature.
-- See docs/IN_APP_NOTIFICATIONS.md for the full spec.

CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    related_entity_id UUID,
    related_entity_type TEXT,
    action_url TEXT,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Hot path: list a user's notifications newest first, filtered by read/unread.
CREATE INDEX IF NOT EXISTS idx_notif_user_unread
    ON notifications(user_id, is_read, created_at DESC);

-- Used by the unfiltered list query.
CREATE INDEX IF NOT EXISTS idx_notif_user_created
    ON notifications(user_id, created_at DESC);
