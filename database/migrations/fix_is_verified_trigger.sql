-- ==========================================
-- MIGRATION: Fix is_verified for Admin users
-- Date: 2025-11-27
-- Description: Update handle_new_auth_user() trigger to respect is_verified
--              from metadata instead of always setting false for Admin role
-- ==========================================

-- Update the auth user sync function to respect is_verified from metadata
CREATE OR REPLACE FUNCTION public.handle_new_auth_user()
RETURNS TRIGGER AS $$
BEGIN
    -- Insert new user into public.users table using auth user data
    INSERT INTO public.users (
        id,
        school_id,
        first_name,
        last_name,
        email,
        role,
        is_verified,
        created_at,
        metadata,
        is_active
    ) VALUES (
        NEW.id,
        COALESCE(
            (NEW.raw_user_meta_data->>'school_id')::UUID,
            NULL
        ),
        COALESCE(
            NEW.raw_user_meta_data->>'first_name',
            ''
        ),
        COALESCE(
            NEW.raw_user_meta_data->>'last_name',
            ''
        ),
        NEW.email,
        COALESCE(
            NEW.raw_user_meta_data->>'role',
            'Parent'
        ),
        -- Respect is_verified from metadata if provided, otherwise default based on role
        COALESCE(
            (NEW.raw_user_meta_data->>'is_verified')::boolean,
            CASE
                WHEN COALESCE(NEW.raw_user_meta_data->>'role', 'Parent') IN ('Parent', 'primary-parent', 'secondary-parent')
                THEN true
                ELSE false
            END
        ),
        NOW(),
        NEW.raw_user_meta_data,
        true
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ==========================================
-- MIGRATION COMPLETE
-- ==========================================
-- After running this migration:
-- 1. Admin users created via /auth/invite-create will have is_verified = true
-- 2. Admin users created via /auth/invite-create-enhanced will have is_verified = true
-- 3. Parent users will still default to is_verified = true
-- 4. Other roles without explicit is_verified will default to false


ALTER TABLE users
ADD COLUMN phone_number VARCHAR(20) DEFAULT NULL;
