ALTER TABLE users
    ADD COLUMN IF NOT EXISTS phone_number VARCHAR(50),
    ADD COLUMN IF NOT EXISTS relation_type VARCHAR(10)
        CHECK (relation_type IS NULL OR relation_type IN ('FATHER', 'MOTHER'));
