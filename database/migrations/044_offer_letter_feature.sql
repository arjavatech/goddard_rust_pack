-- Offer Letter Feature
-- Adds school-level feature flag, employee offer_status, and offer_letters table

ALTER TABLE schools ADD COLUMN IF NOT EXISTS offer_letter_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE employees ADD COLUMN IF NOT EXISTS offer_status VARCHAR(50) DEFAULT NULL;

CREATE TABLE IF NOT EXISTS offer_letters (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id       UUID NOT NULL REFERENCES schools(id),
    employee_id     UUID NOT NULL REFERENCES employees(id) ON DELETE CASCADE,

    first_name      VARCHAR(100) NOT NULL,
    last_name       VARCHAR(100) NOT NULL,
    email           VARCHAR(255) NOT NULL,
    phone           VARCHAR(50),
    role            VARCHAR(200) NOT NULL,
    salary          DECIMAL(12,2),
    salary_currency VARCHAR(10) DEFAULT 'USD',
    joining_date    DATE,
    additional_notes TEXT,

    fillout_form_id       VARCHAR(255),
    fillout_submission_id VARCHAR(255),
    fillout_edit_link     TEXT,
    fillout_pdf_link      TEXT,
    form_data             JSONB,
    prefill_url           TEXT,

    status          VARCHAR(50) NOT NULL DEFAULT 'draft',

    employee_responded_at     TIMESTAMP,
    employee_rejection_reason TEXT,

    reviewed_by            UUID REFERENCES users(id),
    admin_reviewed_at      TIMESTAMP,
    admin_rejection_reason TEXT,

    sent_at    TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP,

    CONSTRAINT unique_offer_per_employee UNIQUE (employee_id)
);

CREATE INDEX IF NOT EXISTS idx_offer_letters_school_id    ON offer_letters(school_id);
CREATE INDEX IF NOT EXISTS idx_offer_letters_employee_id  ON offer_letters(employee_id);

-- School-level Fillout form used for all offer letters at this school
ALTER TABLE schools
  ADD COLUMN IF NOT EXISTS offer_letter_fillout_form_id VARCHAR(255) DEFAULT NULL;
