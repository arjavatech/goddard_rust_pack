-- Apply school-local-time storage to surveys and conferences, matching the
-- pattern established in migration 016 for users, children, etc.
-- Both surveys.published_at and users.created_at must be in the same
-- "currency" (local wall-clock time stored as UTC) for the survey filter
-- `users.created_at <= surveys.published_at` to work correctly.

-- ── Surveys backfill (existing rows: real UTC → school local time) ────────────

UPDATE surveys s SET
    published_at = CASE WHEN s.published_at IS NULL THEN NULL
                        ELSE to_school_local_time(s.school_id, s.published_at) END,
    expires_at   = CASE WHEN s.expires_at IS NULL THEN NULL
                        ELSE to_school_local_time(s.school_id, s.expires_at)   END,
    created_at   = to_school_local_time(s.school_id, s.created_at),
    updated_at   = CASE WHEN s.updated_at IS NULL THEN NULL
                        ELSE to_school_local_time(s.school_id, s.updated_at)   END;

-- ── Surveys triggers ──────────────────────────────────────────────────────────

-- Reuse existing function for created_at / updated_at
DROP TRIGGER IF EXISTS set_school_local_audit_time_trigger ON surveys;
CREATE TRIGGER set_school_local_audit_time_trigger BEFORE INSERT OR UPDATE ON surveys
FOR EACH ROW EXECUTE FUNCTION set_school_local_audit_time();

-- published_at: override Rust Utc::now() with school local now
-- expires_at:   convert the future UTC value from the backend to local equivalent
CREATE OR REPLACE FUNCTION set_survey_local_event_times()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := school_local_now(NEW.school_id);
    IF NEW.published_at IS NOT NULL
       AND (TG_OP = 'INSERT' OR NEW.published_at IS DISTINCT FROM OLD.published_at) THEN
        NEW.published_at := local_now;
    END IF;
    IF NEW.expires_at IS NOT NULL
       AND (TG_OP = 'INSERT' OR NEW.expires_at IS DISTINCT FROM OLD.expires_at) THEN
        NEW.expires_at := to_school_local_time(NEW.school_id, NEW.expires_at);
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS set_survey_local_event_times_trigger ON surveys;
CREATE TRIGGER set_survey_local_event_times_trigger BEFORE INSERT OR UPDATE ON surveys
FOR EACH ROW EXECUTE FUNCTION set_survey_local_event_times();

-- ── Conferences backfill ──────────────────────────────────────────────────────

UPDATE conferences c SET
    booking_close_at = CASE WHEN c.booking_close_at IS NULL THEN NULL
                            ELSE to_school_local_time(c.school_id, c.booking_close_at) END,
    created_at = to_school_local_time(c.school_id, c.created_at),
    updated_at = CASE WHEN c.updated_at IS NULL THEN NULL
                      ELSE to_school_local_time(c.school_id, c.updated_at) END;

UPDATE conference_bookings cb SET
    booked_at  = to_school_local_time(cb.school_id, cb.booked_at),
    updated_at = CASE WHEN cb.updated_at IS NULL THEN NULL
                      ELSE to_school_local_time(cb.school_id, cb.updated_at) END;

-- ── Conference triggers ───────────────────────────────────────────────────────

-- Reuse existing function for created_at / updated_at
DROP TRIGGER IF EXISTS set_school_local_audit_time_trigger ON conferences;
CREATE TRIGGER set_school_local_audit_time_trigger BEFORE INSERT OR UPDATE ON conferences
FOR EACH ROW EXECUTE FUNCTION set_school_local_audit_time();

-- booking_close_at: convert UTC value from backend to local time
CREATE OR REPLACE FUNCTION set_conference_local_booking_close_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.booking_close_at IS NOT NULL
       AND (TG_OP = 'INSERT' OR NEW.booking_close_at IS DISTINCT FROM OLD.booking_close_at) THEN
        NEW.booking_close_at := to_school_local_time(NEW.school_id, NEW.booking_close_at);
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS set_conference_local_booking_close_time_trigger ON conferences;
CREATE TRIGGER set_conference_local_booking_close_time_trigger BEFORE INSERT OR UPDATE ON conferences
FOR EACH ROW EXECUTE FUNCTION set_conference_local_booking_close_time();

-- conference_bookings uses booked_at instead of created_at, needs its own trigger
CREATE OR REPLACE FUNCTION set_conference_booking_local_time()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE local_now TIMESTAMP;
BEGIN
    local_now := school_local_now(NEW.school_id);
    IF TG_OP = 'INSERT' THEN NEW.booked_at := local_now; END IF;
    NEW.updated_at := local_now;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS set_conference_booking_local_time_trigger ON conference_bookings;
CREATE TRIGGER set_conference_booking_local_time_trigger BEFORE INSERT OR UPDATE ON conference_bookings
FOR EACH ROW EXECUTE FUNCTION set_conference_booking_local_time();
