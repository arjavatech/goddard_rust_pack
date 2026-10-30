ALTER TABLE users ADD COLUMN IF NOT EXISTS permissions JSONB DEFAULT '[]'::jsonb;

-- Preserve existing admin access: give all current admins every permission
UPDATE users
SET permissions = '["manage_parents","manage_employees","manage_expenses","manage_taptime","manage_conferences","manage_surveys"]'::jsonb
WHERE role = 'Admin' AND (is_active = true OR is_active IS NULL);
