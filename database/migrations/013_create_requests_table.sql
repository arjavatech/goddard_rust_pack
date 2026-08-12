-- Migration 013: Create requests table for procurement feature
-- Single table handles both the request workflow and expense history.
-- source = 'request' → went through the Pending → In Progress → Completed workflow
-- source = 'manual'  → superadmin direct expense entry (status = 'Completed' immediately)

CREATE TABLE IF NOT EXISTS requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id),
    requester_id UUID,
    requester_name VARCHAR(255) NOT NULL,
    requester_role VARCHAR(50),               -- 'employee' | 'admin' | 'superadmin'
    item VARCHAR(500) NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    category VARCHAR(100),
    scope VARCHAR(50),                        -- 'classroom' | 'teacher' | 'school'
    classroom_id UUID,
    classroom_name VARCHAR(255),
    teacher_id UUID,
    teacher_name VARCHAR(255),
    product_link TEXT,
    product_image TEXT,
    notes TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'Pending',  -- 'Pending' | 'In Progress' | 'Completed'
    source VARCHAR(50) NOT NULL DEFAULT 'request',   -- 'request' | 'manual'
    amount_spent FLOAT8,
    payment_method VARCHAR(100),
    purchase_date DATE,
    payment_notes TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_requests_school_status ON requests(school_id, status);
CREATE INDEX IF NOT EXISTS idx_requests_requester ON requests(requester_id);
CREATE INDEX IF NOT EXISTS idx_requests_created ON requests(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_requests_source ON requests(source);
