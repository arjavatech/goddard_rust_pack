-- Salary is now a custom field configured per school via offer_letter_fields
ALTER TABLE offer_letters DROP COLUMN IF EXISTS salary;
