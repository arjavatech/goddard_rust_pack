-- Durable delivery queue for FCM web push.
-- Notification rows remain the source of truth for the in-app bell.  Each queue
-- row is a single notification-to-browser delivery attempt and can be retried
-- safely by the scheduled worker without creating duplicate in-app records.

CREATE TABLE IF NOT EXISTS notification_push_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notification_id UUID NOT NULL REFERENCES notifications(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_token TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'processing', 'sent', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_until TIMESTAMPTZ,
    sent_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (notification_id, device_token)
);

CREATE INDEX IF NOT EXISTS idx_notification_push_outbox_ready
    ON notification_push_outbox(status, next_attempt_at)
    WHERE status IN ('pending', 'processing');
