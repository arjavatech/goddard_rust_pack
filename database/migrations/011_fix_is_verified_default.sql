-- Change column default so future rows default to verified
ALTER TABLE users ALTER COLUMN is_verified SET DEFAULT true;

-- Unblock existing users (employees and others) stuck with is_verified = false
UPDATE users SET is_verified = true WHERE is_verified = false OR is_verified IS NULL;
