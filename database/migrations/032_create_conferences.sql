-- Conference Booking System
-- Admin creates conferences with classrooms and time slots; parents book slots

CREATE TABLE conferences (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    school_id        UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    title            VARCHAR(255) NOT NULL,
    description      TEXT NOT NULL,
    conference_date  DATE NOT NULL,
    image_url        TEXT,
    booking_close_at TIMESTAMPTZ,
    created_by       UUID REFERENCES users(id),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Which classrooms participate in a conference
CREATE TABLE conference_classrooms (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    conference_id UUID NOT NULL REFERENCES conferences(id) ON DELETE CASCADE,
    classroom_id  UUID NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
    UNIQUE(conference_id, classroom_id)
);

-- Time slots per classroom (same clock time can appear in multiple classrooms independently)
CREATE TABLE conference_slots (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    conference_id UUID NOT NULL REFERENCES conferences(id) ON DELETE CASCADE,
    classroom_id  UUID NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
    start_time    TIME NOT NULL,
    end_time      TIME NOT NULL,
    capacity      INT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Parent bookings (one slot per child per classroom per conference)
CREATE TABLE conference_bookings (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    conference_id UUID NOT NULL REFERENCES conferences(id) ON DELETE CASCADE,
    slot_id       UUID NOT NULL REFERENCES conference_slots(id) ON DELETE CASCADE,
    parent_id     UUID NOT NULL REFERENCES users(id),
    child_id      UUID NOT NULL REFERENCES children(id),
    classroom_id  UUID NOT NULL REFERENCES classrooms(id),
    school_id     UUID NOT NULL REFERENCES schools(id),
    status        VARCHAR(50) NOT NULL DEFAULT 'booked',
    booked_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(conference_id, classroom_id, child_id)
);

CREATE INDEX idx_conferences_school_id ON conferences(school_id);
CREATE INDEX idx_conference_classrooms_conference_id ON conference_classrooms(conference_id);
CREATE INDEX idx_conference_slots_conference_id ON conference_slots(conference_id);
CREATE INDEX idx_conference_slots_classroom_id ON conference_slots(classroom_id);
CREATE INDEX idx_conference_bookings_conference_id ON conference_bookings(conference_id);
CREATE INDEX idx_conference_bookings_parent_id ON conference_bookings(parent_id);
CREATE INDEX idx_conference_bookings_slot_id ON conference_bookings(slot_id);
