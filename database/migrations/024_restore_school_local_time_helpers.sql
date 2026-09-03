-- Repair environments where the school-local audit triggers were applied but
-- the helper functions from migration 016 were not.  Do not repeat the
-- one-time timestamp conversion performed by that migration.

CREATE OR REPLACE FUNCTION public.school_timezone_region(timezone_code TEXT)
RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $$
    SELECT CASE timezone_code
        WHEN 'EST' THEN 'America/New_York' WHEN 'CST' THEN 'America/Chicago'
        WHEN 'MST' THEN 'America/Denver' WHEN 'PST' THEN 'America/Los_Angeles'
        WHEN 'AKST' THEN 'America/Anchorage' WHEN 'HST' THEN 'Pacific/Honolulu'
        WHEN 'IST' THEN 'Asia/Kolkata' WHEN 'GMT' THEN 'Europe/London'
        WHEN 'CET' THEN 'Europe/Berlin' WHEN 'EET' THEN 'Europe/Helsinki'
        WHEN 'GST' THEN 'Asia/Dubai' WHEN 'PKT' THEN 'Asia/Karachi'
        WHEN 'BST' THEN 'Asia/Dhaka' WHEN 'ICT' THEN 'Asia/Bangkok'
        WHEN 'CST_CN' THEN 'Asia/Shanghai' WHEN 'JST' THEN 'Asia/Tokyo'
        WHEN 'KST' THEN 'Asia/Seoul' WHEN 'WIB' THEN 'Asia/Jakarta'
        WHEN 'WITA' THEN 'Asia/Makassar' WHEN 'WIT' THEN 'Asia/Jayapura'
        WHEN 'AEST' THEN 'Australia/Sydney' WHEN 'ACST' THEN 'Australia/Adelaide'
        WHEN 'AWST' THEN 'Australia/Perth' WHEN 'NZST' THEN 'Pacific/Auckland'
        WHEN 'BRT' THEN 'America/Sao_Paulo' WHEN 'ART' THEN 'America/Argentina/Buenos_Aires'
        WHEN 'CLT' THEN 'America/Santiago' WHEN 'SAST' THEN 'Africa/Johannesburg'
        WHEN 'EAT' THEN 'Africa/Nairobi' WHEN 'WAT' THEN 'Africa/Lagos'
        ELSE 'America/Los_Angeles'
    END;
$$;

CREATE OR REPLACE FUNCTION public.school_local_now(target_school_id UUID)
RETURNS TIMESTAMP LANGUAGE SQL STABLE AS $$
    SELECT timezone(public.school_timezone_region(s.timezone), NOW())::timestamp
    FROM public.schools s
    WHERE s.id = target_school_id;
$$;

CREATE OR REPLACE FUNCTION public.to_school_local_time(
    target_school_id UUID,
    instant TIMESTAMPTZ
)
RETURNS TIMESTAMP LANGUAGE SQL STABLE AS $$
    SELECT timezone(public.school_timezone_region(s.timezone), instant)::timestamp
    FROM public.schools s
    WHERE s.id = target_school_id;
$$;
