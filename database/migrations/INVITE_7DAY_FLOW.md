# 7-Day Parent Invite Link System

## Problem
Supabase OTP signup links expire quickly (default ~1 hour) and are single-use.
Parents who receive invite emails in the evening often find the link expired by morning.

## Solution
A custom `parent_invitations` table stores a UUID token valid for 7 days.
When a parent clicks the invite link, our backend validates the 7-day token and
issues a **fresh** Supabase signup URL via the `generate_link` API, then 302-redirects
the parent directly to it. No frontend changes required.

---

## Database

### `user_invitations` table (migration: `002_add_parent_invitations.sql`)
Covers all roles: Parent, Admin, Teacher, SuperAdmin.

| Column | Type | Notes |
|---|---|---|
| `id` | UUID PK | auto-generated |
| `token` | UUID UNIQUE | sent in invite email link |
| `user_email` | VARCHAR(255) | invitee's email |
| `role` | VARCHAR(50) | Parent / Admin / Teacher / SuperAdmin |
| `school_id` | UUID FK | references `schools(id)` |
| `expires_at` | TIMESTAMPTZ | `NOW() + 7 days` |
| `used_at` | TIMESTAMPTZ | (reserved for future use) |
| `created_at` | TIMESTAMPTZ | auto-set |

---

## Flow

### Sending an Invite
1. Admin calls `POST /enrollments/parent-invite`
2. Backend creates user in Supabase (`POST /auth/v1/admin/users`) — no email
3. Backend stores a token in `parent_invitations` (expires in 7 days)
4. Backend sends a branded email via **Resend** with the link:
   `{API_BASE_URL}/enrollments/activate/{token}`

### Parent Clicks the Link
`GET /enrollments/activate/{token}`

| Scenario | Result |
|---|---|
| Token not found | 404 — "Invalid invite link" |
| Token expired (> 7 days) | 410 — "Invite has expired. Please contact your school admin." |
| Token valid, user not yet confirmed | 307 → fresh Supabase signup URL → parent sets password |
| Token valid, user already registered | 307 → `{FRONTEND_URL}/login?message=already_registered` |

---

## Environment Variables

| Variable | Purpose | Default |
|---|---|---|
| `API_BASE_URL` | Backend URL used in invite email links | `https://api.goddard-app.com` |
| `FRONTEND_URL` | Frontend URL for post-signup redirect | `https://dev.goddard-web.pages.dev` |
| `RESEND_API_KEY` | Resend API key for sending invite emails | *(required)* |

---

## Key Files
| File | Role |
|---|---|
| `database/migrations/002_add_parent_invitations.sql` | DB migration |
| `src/dao/enrollment_dao.rs` | `create_invite_token()`, `get_invite_by_token()` |
| `src/services/supabase_client.rs` | `generate_signup_link()`, `send_parent_invite_email()`, `create_user_only_in_supabase()` |
| `src/services/enrollment_service.rs` | `activate_invite()`, updated `create_auth_user()` |
| `src/controllers/enrollment_controller.rs` | `activate_invite` handler |
| `src/main.rs` | Route: `GET /enrollments/activate/:token` |
