-- Keep the public column in sync with the Auth metadata used by Supabase.
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS phone_number VARCHAR(50);

-- Preserve an existing column value; only fill missing values from metadata.
UPDATE public.users
SET phone_number = NULLIF(BTRIM(metadata->>'phone_number'), '')
WHERE NULLIF(BTRIM(phone_number), '') IS NULL
  AND NULLIF(BTRIM(metadata->>'phone_number'), '') IS NOT NULL;

CREATE OR REPLACE FUNCTION public.handle_new_auth_user()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO public.users (
        id, school_id, first_name, last_name, email, role, is_verified,
        created_at, phone_number, metadata, is_active
    ) VALUES (
        NEW.id,
        (NEW.raw_user_meta_data->>'school_id')::UUID,
        COALESCE(NEW.raw_user_meta_data->>'first_name', ''),
        COALESCE(NEW.raw_user_meta_data->>'last_name', ''),
        NEW.email,
        COALESCE(NEW.raw_user_meta_data->>'role', 'Parent'),
        COALESCE(
            (NEW.raw_user_meta_data->>'is_verified')::boolean,
            CASE
                WHEN COALESCE(NEW.raw_user_meta_data->>'role', 'Parent') IN ('Parent', 'primary-parent', 'secondary-parent') THEN true
                ELSE false
            END
        ),
        NOW(),
        NULLIF(BTRIM(NEW.raw_user_meta_data->>'phone_number'), ''),
        NEW.raw_user_meta_data,
        true
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
