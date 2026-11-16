-- Offer Letter Schema Simplification
-- Removes hardcoded offer detail columns that are now handled via custom_fields JSONB.
-- salary is retained as a standard column; role, salary_currency, joining_date,
-- and additional_notes move to custom_fields (configured per school via offer_letter_fields).

ALTER TABLE offer_letters DROP COLUMN IF EXISTS role;
ALTER TABLE offer_letters DROP COLUMN IF EXISTS salary_currency;
ALTER TABLE offer_letters DROP COLUMN IF EXISTS joining_date;
ALTER TABLE offer_letters DROP COLUMN IF EXISTS additional_notes;
