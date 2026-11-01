-- Add drive_enabled feature flag to schools
ALTER TABLE schools ADD COLUMN IF NOT EXISTS drive_enabled BOOLEAN NOT NULL DEFAULT false;

-- Unified table for drive folders and files
CREATE TABLE IF NOT EXISTS drive_items (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id   UUID NOT NULL,
    user_id     UUID NOT NULL,
    folder_id   UUID REFERENCES drive_items(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    item_type   TEXT NOT NULL CHECK (item_type IN ('folder', 'file')),
    s3_key      TEXT,
    mime_type   TEXT,
    file_size   BIGINT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT file_requires_s3 CHECK (item_type = 'folder' OR s3_key IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_drive_items_browse ON drive_items(school_id, user_id, folder_id);
CREATE INDEX IF NOT EXISTS idx_drive_items_folder ON drive_items(folder_id);

-- Add manage_drive permission to all existing active admins
UPDATE users
SET permissions = COALESCE(permissions, '[]'::jsonb) || '["manage_drive"]'::jsonb
WHERE role = 'Admin'
  AND (is_active = true OR is_active IS NULL)
  AND NOT (permissions @> '["manage_drive"]'::jsonb);
