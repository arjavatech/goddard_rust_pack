-- Optional, private PDF reference files for student and employee form templates.
ALTER TABLE form_templates
    ADD COLUMN IF NOT EXISTS pdf_storage_key TEXT,
    ADD COLUMN IF NOT EXISTS pdf_file_name TEXT,
    ADD COLUMN IF NOT EXISTS pdf_content_type TEXT,
    ADD COLUMN IF NOT EXISTS pdf_file_size_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS pdf_uploaded_at TIMESTAMP;

ALTER TABLE employee_form_templates
    ADD COLUMN IF NOT EXISTS pdf_storage_key TEXT,
    ADD COLUMN IF NOT EXISTS pdf_file_name TEXT,
    ADD COLUMN IF NOT EXISTS pdf_content_type TEXT,
    ADD COLUMN IF NOT EXISTS pdf_file_size_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS pdf_uploaded_at TIMESTAMP;
