-- Fix: mark_invite_used_on_password_set fires too early
--
-- The previous trigger fired on ANY encrypted_password update, including when
-- Supabase's Admin generate_link API internally updates encrypted_password during
-- recovery link generation. This caused used_at to be stamped before the user
-- ever set a real password, making the activation link show "already used".
--
-- Fix: only stamp used_at when encrypted_password is an actual bcrypt hash
-- (starts with $2). Supabase internal operations set it to non-bcrypt values.

CREATE OR REPLACE FUNCTION public.mark_invite_used_on_password_set()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    IF NEW.encrypted_password LIKE '$2%' THEN
        UPDATE public.user_invitations
        SET used_at = NOW()
        WHERE user_email = NEW.email
          AND used_at IS NULL;
    END IF;
    RETURN NEW;
END;
$$;
