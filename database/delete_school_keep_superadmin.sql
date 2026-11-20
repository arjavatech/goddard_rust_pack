-- ============================================================
-- SAFE SCHOOL DELETION SCRIPT (KEEP SUPERADMIN VARIANT)
-- Removes ALL data associated with a single school EXCEPT the
-- SuperAdmin user account, which is preserved in:
--   • schools             (record kept)
--   • public.users        (SuperAdmin row kept)
--   • auth.users          (SuperAdmin auth entry kept)
--   • public.user_invitations (SuperAdmin invite row kept)
--
-- Use this when resetting a school's data but the SuperAdmin
-- needs to retain their login to re-configure the school.
--
-- HOW TO USE:
--   1. Open Supabase SQL Editor
--   2. Replace 'YOUR-SCHOOL-UUID-HERE' with the actual school UUID
--   3. Run — the entire block is one atomic transaction;
--      any error rolls everything back automatically.
-- ============================================================

DO $$
DECLARE
    _sid      UUID   := 'YOUR-SCHOOL-UUID-HERE';
    _uids     UUID[];   -- all non-SuperAdmin user IDs
    _sa_id    UUID;     -- SuperAdmin user ID (preserved)
    _sa_email TEXT;     -- SuperAdmin email (used to exclude user_invitations row)
BEGIN
    -- Verify the school exists before proceeding
    IF NOT EXISTS (SELECT 1 FROM schools WHERE id = _sid) THEN
        RAISE EXCEPTION 'School not found: %', _sid;
    END IF;

    -- Capture SuperAdmin ID and email so we can exclude them everywhere
    SELECT id, email INTO _sa_id, _sa_email FROM users WHERE school_id = _sid AND role = 'SuperAdmin' LIMIT 1;

    -- Capture all non-SuperAdmin user IDs for tables without school_id
    SELECT ARRAY_AGG(id) INTO _uids
    FROM users
    WHERE school_id = _sid AND role != 'SuperAdmin';

    RAISE NOTICE 'Deleting school % — % non-SuperAdmin user(s) found (SuperAdmin % preserved)',
        _sid,
        COALESCE(ARRAY_LENGTH(_uids, 1), 0),
        COALESCE(_sa_id::TEXT, 'none');

    -- ── Fix self-references ────────────────────────────────────────────
    UPDATE users SET created_by = NULL WHERE school_id = _sid;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'drive_items') THEN
        UPDATE drive_items SET folder_id = NULL WHERE school_id = _sid;
    END IF;

    -- ── 1. Document pipeline ──────────────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'document_requests') THEN
        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'document_request_assignments') THEN
            UPDATE document_request_assignments SET latest_submission_id = NULL WHERE school_id = _sid;
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'document_audit_events') THEN
            DELETE FROM document_audit_events WHERE school_id = _sid;
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'document_submissions') THEN
            DELETE FROM document_submissions WHERE school_id = _sid;
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'document_request_assignments') THEN
            DELETE FROM document_request_assignments WHERE school_id = _sid;
        END IF;

        DELETE FROM document_requests WHERE school_id = _sid;
    END IF;

    -- ── 2. Survey pipeline ────────────────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'surveys') THEN
        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'survey_response_answers') THEN
            DELETE FROM survey_response_answers
                WHERE response_id IN (
                    SELECT sr.id FROM survey_responses sr
                    JOIN surveys s ON sr.survey_id = s.id
                    WHERE s.school_id = _sid
                );
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'survey_responses') THEN
            DELETE FROM survey_responses WHERE school_id = _sid;
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'survey_questions') THEN
            DELETE FROM survey_questions WHERE survey_id IN (SELECT id FROM surveys WHERE school_id = _sid);
        END IF;

        DELETE FROM surveys WHERE school_id = _sid;
    END IF;

    -- ── 3. Conference pipeline ────────────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'conferences') THEN
        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'conference_bookings') THEN
            DELETE FROM conference_bookings WHERE conference_id IN (SELECT id FROM conferences WHERE school_id = _sid);
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'conference_slots') THEN
            DELETE FROM conference_slots WHERE conference_id IN (SELECT id FROM conferences WHERE school_id = _sid);
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'conference_classrooms') THEN
            DELETE FROM conference_classrooms WHERE conference_id IN (SELECT id FROM conferences WHERE school_id = _sid);
        END IF;

        DELETE FROM conferences WHERE school_id = _sid;
    END IF;

    -- ── 4. Drive items ────────────────────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'drive_items') THEN
        DELETE FROM drive_items WHERE school_id = _sid;
    END IF;

    -- ── 5. Offer letters ──────────────────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'offer_letters') THEN
        DELETE FROM offer_letters WHERE school_id = _sid;
    END IF;

    -- ── 6. Employee form pipeline ─────────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'employee_form_templates') THEN
        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'employee_form_submissions') THEN
            DELETE FROM employee_form_submissions WHERE school_id = _sid;
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'employee_form_assignments') THEN
            DELETE FROM employee_form_assignments WHERE school_id = _sid;
        END IF;

        IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'employees') THEN
            DELETE FROM employees WHERE school_id = _sid;
        END IF;

        DELETE FROM employee_form_templates WHERE school_id = _sid;
    ELSIF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'employees') THEN
        DELETE FROM employees WHERE school_id = _sid;
    END IF;

    -- ── 7. Notifications & TapTime ────────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'notification_push_outbox') THEN
        DELETE FROM notification_push_outbox WHERE user_id = ANY(_uids);
    END IF;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'taptime_user_mappings') THEN
        DELETE FROM taptime_user_mappings WHERE school_id = _sid;
    END IF;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'notifications') THEN
        DELETE FROM notifications WHERE school_id = _sid;
    END IF;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'device_tokens') THEN
        DELETE FROM device_tokens WHERE user_id = ANY(_uids);
    END IF;

    -- ── 8. Student / enrollment pipeline ─────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'form_submissions') THEN
        DELETE FROM form_submissions WHERE school_id = _sid;
    END IF;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'student_form_assignments') THEN
        DELETE FROM student_form_assignments WHERE school_id = _sid;
    END IF;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'class_transitions') THEN
        DELETE FROM class_transitions WHERE school_id = _sid;
    END IF;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'documents') THEN
        DELETE FROM documents WHERE school_id = _sid;
    END IF;

    DELETE FROM enrollments WHERE school_id = _sid;

    -- ── 9. School structure ───────────────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'class_form_overrides') THEN
        DELETE FROM class_form_overrides WHERE school_id = _sid;
    END IF;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'form_templates') THEN
        DELETE FROM form_templates WHERE school_id = _sid;
    END IF;

    DELETE FROM classrooms WHERE school_id = _sid;
    DELETE FROM children   WHERE school_id = _sid;

    -- ── 10. Misc school-scoped data ───────────────────────────────────
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'requests') THEN
        DELETE FROM requests WHERE school_id = _sid;
    END IF;

    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'help_center_messages') THEN
        DELETE FROM help_center_messages WHERE school_id = _sid;
    END IF;

    -- user_invitations: delete all except the SuperAdmin's row (keyed by user_email)
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'user_invitations') THEN
        IF _sa_email IS NOT NULL THEN
            DELETE FROM user_invitations WHERE school_id = _sid AND user_email != _sa_email;
        ELSE
            DELETE FROM user_invitations WHERE school_id = _sid;
        END IF;
    END IF;

    -- ── 11. Public users — skip SuperAdmin ────────────────────────────
    DELETE FROM users WHERE school_id = _sid AND role != 'SuperAdmin';

    -- ── 12. Auth users — skip SuperAdmin ─────────────────────────────
    -- _uids already excludes the SuperAdmin, so this is safe.
    IF _uids IS NOT NULL THEN
        DELETE FROM auth.users WHERE id = ANY(_uids);
    END IF;

    -- ── 13. School record ─────────────────────────────────────────────
    -- NOTE: schools record is intentionally kept so the SuperAdmin can
    -- log back in and re-configure the school from scratch.
    -- Uncomment the line below only if you also want to drop the school.
    -- DELETE FROM schools WHERE id = _sid;

    RAISE NOTICE 'School % data cleared. SuperAdmin (%) preserved. School record kept.', _sid, COALESCE(_sa_id::TEXT, 'none');
END $$;
