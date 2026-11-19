-- ============================================================
-- SAFE SCHOOL DELETION SCRIPT
-- Removes ALL data associated with a single school, including
-- public schema records and Supabase auth.users entries.
--
-- HOW TO USE:
--   1. Open Supabase SQL Editor
--   2. Replace 'YOUR-SCHOOL-UUID-HERE' with the actual school UUID
--   3. Run — the entire block is one atomic transaction;
--      any error rolls everything back automatically.
--
-- SAFETY:
--   • Every DELETE is scoped exclusively to the target school_id
--     or to user IDs that strictly belong to that school.
--   • No data from any other school is touched.
--   • Self-referential FKs (users.created_by, drive_items.folder_id)
--     are nulled before deletion to avoid constraint violations.
--   • Deleting auth.users automatically cascades to
--     auth.identities, auth.sessions, auth.refresh_tokens,
--     auth.mfa_factors (Supabase manages these with ON DELETE CASCADE).
-- ============================================================

DO $$
DECLARE
    _sid  UUID   := 'YOUR-SCHOOL-UUID-HERE';
    _uids UUID[];
BEGIN
    -- Verify the school exists before proceeding
    IF NOT EXISTS (SELECT 1 FROM schools WHERE id = _sid) THEN
        RAISE EXCEPTION 'School not found: %', _sid;
    END IF;

    -- Capture all user IDs belonging to this school
    -- (required for tables that reference users without a school_id column)
    SELECT ARRAY_AGG(id) INTO _uids FROM users WHERE school_id = _sid;
    RAISE NOTICE 'Deleting school % — % user(s) found', _sid, COALESCE(ARRAY_LENGTH(_uids, 1), 0);

    -- ── Fix self-references ────────────────────────────────────────────
    -- users.created_by and drive_items.folder_id are non-CASCADE self-refs;
    -- they must be nulled before the rows they point to are deleted.
    UPDATE users       SET created_by = NULL WHERE school_id = _sid;
    UPDATE drive_items SET folder_id  = NULL WHERE school_id = _sid;

    -- ── 1. Document pipeline ──────────────────────────────────────────
    -- document_request_assignments.latest_submission_id → document_submissions(id)
    -- is a circular non-CASCADE FK; null it first to break the cycle.
    UPDATE document_request_assignments
        SET latest_submission_id = NULL
        WHERE school_id = _sid;
    DELETE FROM document_audit_events        WHERE school_id = _sid;
    DELETE FROM document_submissions         WHERE school_id = _sid;
    DELETE FROM document_request_assignments WHERE school_id = _sid;
    DELETE FROM document_requests            WHERE school_id = _sid;

    -- ── 2. Survey pipeline ────────────────────────────────────────────
    -- survey_response_answers has no school_id; go through survey_responses
    DELETE FROM survey_response_answers
        WHERE response_id IN (
            SELECT sr.id
            FROM   survey_responses sr
            JOIN   surveys s ON sr.survey_id = s.id
            WHERE  s.school_id = _sid
        );
    DELETE FROM survey_responses WHERE school_id = _sid;
    -- survey_questions has no school_id; go through surveys
    DELETE FROM survey_questions
        WHERE survey_id IN (SELECT id FROM surveys WHERE school_id = _sid);
    DELETE FROM surveys WHERE school_id = _sid;

    -- ── 3. Conference pipeline ────────────────────────────────────────
    -- conference sub-tables have no school_id; go through conferences
    DELETE FROM conference_bookings
        WHERE conference_id IN (SELECT id FROM conferences WHERE school_id = _sid);
    DELETE FROM conference_slots
        WHERE conference_id IN (SELECT id FROM conferences WHERE school_id = _sid);
    DELETE FROM conference_classrooms
        WHERE conference_id IN (SELECT id FROM conferences WHERE school_id = _sid);
    DELETE FROM conferences WHERE school_id = _sid;

    -- ── 4. Drive items ────────────────────────────────────────────────
    -- folder_id self-reference already nulled above; safe to delete all at once
    DELETE FROM drive_items WHERE school_id = _sid;

    -- ── 5. Offer letters ──────────────────────────────────────────────
    -- offer_letters.reviewed_by → users is non-CASCADE;
    -- must be deleted before users (step 11)
    DELETE FROM offer_letters WHERE school_id = _sid;

    -- ── 6. Employee form pipeline ─────────────────────────────────────
    DELETE FROM employee_form_submissions WHERE school_id = _sid;
    DELETE FROM employee_form_assignments WHERE school_id = _sid;
    DELETE FROM employees                 WHERE school_id = _sid;
    DELETE FROM employee_form_templates   WHERE school_id = _sid;

    -- ── 7. Notifications & TapTime ────────────────────────────────────
    -- notification_push_outbox and device_tokens have no school_id;
    -- scope them via the captured _uids array
    DELETE FROM notification_push_outbox WHERE user_id = ANY(_uids);
    DELETE FROM taptime_user_mappings    WHERE school_id = _sid;
    DELETE FROM notifications            WHERE school_id = _sid;
    DELETE FROM device_tokens            WHERE user_id   = ANY(_uids);

    -- ── 8. Student / enrollment pipeline ─────────────────────────────
    DELETE FROM form_submissions         WHERE school_id = _sid;
    DELETE FROM student_form_assignments WHERE school_id = _sid;
    DELETE FROM class_transitions        WHERE school_id = _sid;
    DELETE FROM documents                WHERE school_id = _sid;
    DELETE FROM enrollments              WHERE school_id = _sid;

    -- ── 9. School structure ───────────────────────────────────────────
    DELETE FROM class_form_overrides WHERE school_id = _sid;
    DELETE FROM form_templates       WHERE school_id = _sid;
    DELETE FROM classrooms           WHERE school_id = _sid;
    DELETE FROM children             WHERE school_id = _sid;

    -- ── 10. Misc school-scoped data ───────────────────────────────────
    DELETE FROM requests             WHERE school_id = _sid;
    DELETE FROM help_center_messages WHERE school_id = _sid;
    DELETE FROM user_invitations     WHERE school_id = _sid;

    -- ── 11. Public users ──────────────────────────────────────────────
    -- All non-CASCADE FK references to users have been removed above.
    DELETE FROM users WHERE school_id = _sid;

    -- ── 12. Auth users ────────────────────────────────────────────────
    -- Supabase auth.users CASCADE-deletes auth.identities, auth.sessions,
    -- auth.refresh_tokens, and auth.mfa_factors automatically.
    IF _uids IS NOT NULL THEN
        DELETE FROM auth.users WHERE id = ANY(_uids);
    END IF;

    -- ── 13. School record ─────────────────────────────────────────────
    DELETE FROM schools WHERE id = _sid;

    RAISE NOTICE 'School % and all associated data deleted successfully.', _sid;
END $$;
