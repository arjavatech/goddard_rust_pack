-- School News Pad: newsletter content, audience snapshots, and scheduled reminders.
CREATE TABLE IF NOT EXISTS newsletters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK (char_length(trim(title)) > 0),
    content_blocks JSONB NOT NULL DEFAULT '[]'::jsonb,
    rendered_html TEXT NOT NULL DEFAULT '',
    audience_scope TEXT NOT NULL CHECK (audience_scope IN ('school', 'classrooms')),
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'scheduled', 'published', 'archived')),
    scheduled_at TIMESTAMPTZ,
    school_timezone TEXT NOT NULL DEFAULT 'UTC',
    reminder_offsets_days SMALLINT[] NOT NULL DEFAULT '{}',
    published_at TIMESTAMPTZ,
    archived_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    CHECK ((status = 'scheduled') = (scheduled_at IS NOT NULL)),
    CHECK (audience_scope = 'school' OR audience_scope = 'classrooms')
);

CREATE TABLE IF NOT EXISTS newsletter_audiences (
    newsletter_id UUID NOT NULL REFERENCES newsletters(id) ON DELETE CASCADE,
    classroom_id UUID NOT NULL REFERENCES classrooms(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (newsletter_id, classroom_id)
);

CREATE TABLE IF NOT EXISTS newsletter_recipients (
    newsletter_id UUID NOT NULL REFERENCES newsletters(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    applicable_children JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (newsletter_id, user_id)
);

CREATE TABLE IF NOT EXISTS newsletter_reminder_deliveries (
    newsletter_id UUID NOT NULL REFERENCES newsletters(id) ON DELETE CASCADE,
    offset_days SMALLINT NOT NULL CHECK (offset_days IN (3, 7, 15)),
    delivered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (newsletter_id, offset_days)
);

CREATE INDEX IF NOT EXISTS idx_newsletters_school_status_published
    ON newsletters(school_id, status, published_at DESC)
    WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_newsletters_due
    ON newsletters(status, scheduled_at)
    WHERE status = 'scheduled' AND is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_newsletter_recipients_user
    ON newsletter_recipients(user_id, newsletter_id);

ALTER TABLE newsletters ENABLE ROW LEVEL SECURITY;
ALTER TABLE newsletter_audiences ENABLE ROW LEVEL SECURITY;
ALTER TABLE newsletter_recipients ENABLE ROW LEVEL SECURITY;
ALTER TABLE newsletter_reminder_deliveries ENABLE ROW LEVEL SECURITY;

-- The current production schema does not install the audit helper functions
-- described in database/README.md.  Keep this migration self-contained; API
-- writes set the creator fields and explicit UPDATE statements set timestamps.
