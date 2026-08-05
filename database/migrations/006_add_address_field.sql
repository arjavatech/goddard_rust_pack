-- Migration 006: Add address field to users table for primary parent address storage
ALTER TABLE users ADD COLUMN IF NOT EXISTS address TEXT;
