-- Purchased school services. All services are opt-in and database-managed.
ALTER TABLE schools
    ADD COLUMN IF NOT EXISTS parent_management_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS employee_management_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS expense_management_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS taptime_enabled BOOLEAN NOT NULL DEFAULT FALSE;
