-- Trigger function: marks user_invitations.used_at when the user sets their own password.
-- Fires ONLY on encrypted_password UPDATE (not on INSERT / session creation / generate_link).
CREATE OR REPLACE FUNCTION public.mark_invite_used_on_password_set()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    UPDATE public.user_invitations
    SET used_at = NOW()
    WHERE user_email = NEW.email
      AND used_at IS NULL;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS on_auth_user_password_change ON auth.users;

CREATE TRIGGER on_auth_user_password_change
    AFTER UPDATE OF encrypted_password ON auth.users
    FOR EACH ROW
    WHEN (OLD.encrypted_password IS DISTINCT FROM NEW.encrypted_password)
    EXECUTE FUNCTION public.mark_invite_used_on_password_set();
