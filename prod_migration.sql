-- =============================================
-- Production Database Migration
-- Sync with dev_db.sql
-- Date: 2026-02-10
-- =============================================

-- ============================================
-- MISSING TABLE: class_transitions
-- ============================================

CREATE TABLE class_transitions (
    id UUID DEFAULT uuid_generate_v4() NOT NULL,
    enrollment_id UUID NOT NULL,
    child_id UUID NOT NULL,
    school_id UUID NOT NULL,
    from_classroom_id UUID NOT NULL,
    to_classroom_id UUID NOT NULL,
    changed_by UUID,
    reason TEXT,
    transitioned_at TIMESTAMP DEFAULT now(),
    created_at TIMESTAMP DEFAULT now(),
    is_active BOOLEAN DEFAULT true
);

-- Add primary key
ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_pkey PRIMARY KEY (id);

-- Add foreign keys
ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_changed_by_fkey 
    FOREIGN KEY (changed_by) REFERENCES users (id);

ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_child_id_fkey 
    FOREIGN KEY (child_id) REFERENCES children (id);

ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_enrollment_id_fkey 
    FOREIGN KEY (enrollment_id) REFERENCES enrollments (id);

ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_from_classroom_id_fkey 
    FOREIGN KEY (from_classroom_id) REFERENCES classrooms (id);

ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_school_id_fkey 
    FOREIGN KEY (school_id) REFERENCES schools (id);

ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_to_classroom_id_fkey 
    FOREIGN KEY (to_classroom_id) REFERENCES classrooms (id);

-- ============================================
-- MISSING FUNCTION: track_classroom_transition
-- ============================================

CREATE OR REPLACE FUNCTION public.track_classroom_transition()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    current_user_id UUID;
    recent_transition_count INTEGER;
BEGIN
    -- Only track if classroom actually changed
    IF OLD.classroom_id IS DISTINCT FROM NEW.classroom_id THEN

        -- Check if a transition was just updated in the last 2 seconds (edit sync scenario)
        SELECT COUNT(*) INTO recent_transition_count
        FROM class_transitions
        WHERE enrollment_id = NEW.id
        AND from_classroom_id = OLD.classroom_id
        AND to_classroom_id = NEW.classroom_id
        AND created_at > NOW() - INTERVAL '2 seconds';

        -- Skip if duplicate found
        IF recent_transition_count > 0 THEN
            RETURN NEW;
        END IF;

        -- Always create transition record (removed form submission check)
        BEGIN
            current_user_id := current_setting('app.current_user_id', true)::UUID;
        EXCEPTION WHEN OTHERS THEN
            current_user_id := NULL;
        END;

        INSERT INTO class_transitions (
            enrollment_id,
            child_id,
            school_id,
            from_classroom_id,
            to_classroom_id,
            changed_by,
            transitioned_at
        ) VALUES (
            NEW.id,
            NEW.child_id,
            NEW.school_id,
            OLD.classroom_id,
            NEW.classroom_id,
            current_user_id,
            NOW()
        );
    END IF;
    RETURN NEW;
END;
$function$;

-- ============================================
-- MISSING TRIGGER: trigger_track_classroom_transition
-- ============================================

CREATE TRIGGER trigger_track_classroom_transition
    AFTER UPDATE ON enrollments
    FOR EACH ROW
    EXECUTE FUNCTION track_classroom_transition();

-- ============================================
-- VERIFICATION QUERIES
-- ============================================

-- Verify table exists
SELECT 'Table class_transitions exists: ' || 
    CASE WHEN COUNT(*) > 0 THEN 'YES ✓' ELSE 'NO ✗' END
FROM information_schema.tables
WHERE table_schema = 'public' 
AND table_name = 'class_transitions';

-- Verify function exists
SELECT 'Function track_classroom_transition exists: ' || 
    CASE WHEN COUNT(*) > 0 THEN 'YES ✓' ELSE 'NO ✗' END
FROM pg_proc 
WHERE proname = 'track_classroom_transition' 
AND pronamespace = 'public'::regnamespace;

-- Verify trigger exists
SELECT 'Trigger trigger_track_classroom_transition exists: ' || 
    CASE WHEN COUNT(*) > 0 THEN 'YES ✓' ELSE 'NO ✗' END
FROM information_schema.triggers
WHERE trigger_name = 'trigger_track_classroom_transition'
AND trigger_schema = 'public';

-- Count constraints
SELECT 'class_transitions constraints: ' || COUNT(*)::TEXT
FROM information_schema.table_constraints
WHERE table_schema = 'public' 
AND table_name = 'class_transitions';
