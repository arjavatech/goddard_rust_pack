-- Redesign Drive from per-user folders to two shared group drives:
-- 'admin'  = shared by all Admins in a school
-- 'owner'  = shared by all Owners and SuperAdmins in a school

-- Add drive_type column to classify items into the two shared drives
ALTER TABLE drive_items
  ADD COLUMN drive_type TEXT NOT NULL DEFAULT 'admin'
    CHECK (drive_type IN ('admin', 'owner'));

-- Add modified_by tracking (denormalized name for fast reads without JOIN)
ALTER TABLE drive_items
  ADD COLUMN modified_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  ADD COLUMN modified_by_name TEXT;

-- Back-fill: existing items were all admin items; set modified_by = original creator
UPDATE drive_items d
SET modified_by_user_id = d.user_id,
    modified_by_name    = u.first_name || ' ' || u.last_name
FROM users u
WHERE u.id = d.user_id;

-- Replace old (school_id, user_id, folder_id) index with new access pattern
DROP INDEX IF EXISTS idx_drive_items_browse;
CREATE INDEX idx_drive_items_browse ON drive_items(school_id, drive_type, folder_id);
