# In-App Notifications

Authoritative spec for the bell-icon + drawer in-app notification system used by both admin
and parent surfaces. Implementation lives in:

- `lambda/goddard/src/models/notification.rs`
- `lambda/goddard/src/dao/notification_dao.rs`
- `lambda/goddard/src/services/notification_service.rs`
- `lambda/goddard/src/controllers/notification_controller.rs`
- `lambda/goddard/src/services/fcm_service.rs`
- `lambda/goddard/src/dao/device_token_dao.rs`
- `lambda/goddard/src/dao/notification_push_outbox_dao.rs`
- `database/migrations/004_notifications.sql`
- `database/migrations/019_notification_push_outbox.sql`

Companion to `docs/EMAIL_NOTIFICATIONS.md`. Email + in-app are sibling channels fired off the
same event hooks; either channel can fail without affecting the other or the originating API
call.

## Event matrix

| Event | In-app → parent | In-app → school admins | Email today? |
|---|:---:|:---:|:---:|
| Form approved | ✅ | — | ✅ |
| Form rejected | ✅ | — | ✅ |
| Form assigned (single / bulk / school-wide / class-wide) | ✅ | — | ✅ |
| Form submitted (parent completes a Fillout form) | — | ✅ | — |
| Child added (additional child for existing parent) | ✅ | ✅ | ✅ |
| Child archived (`status="archive"` or `"archived"`) | ✅ | ✅ | ✅ |
| Parent invited (new parent flow) | — | ✅ | (invite email) |
| Parent deactivated | ✅ | ✅ | ✅ |
| Admin user added | — | ✅ (other admins) | (invite email) |
| Classroom added / deleted | — | ✅ | — |
| Form template added / deleted | — | ✅ | — |

"School admins" = all `users` rows with `role IN ('Admin','SuperAdmin')` AND
`school_id = X` AND `is_active = true`.

## Architecture

- Notification rows remain the source of truth for the bell and drawer. Browser push never
  replaces the stored in-app notification.
- Web push is FCM-only. A registration belongs to a **user + browser/device token**, not to a
  `users` column, so one account can use multiple browsers and tokens can rotate safely.
- The Profile page asks for permission only after an explicit user action. FCM foreground
  messages refresh the bell; the service worker is the sole background renderer.
- Browser push is limited to action-required form and document events. Informational events
  remain in the in-app bell only.
- `notification_push_outbox` is written in the same transaction as an eligible notification.
  The scheduled worker leases entries with `FOR UPDATE SKIP LOCKED`, sends through FCM, and
  retries transient failures with exponential backoff. This avoids Lambda background tasks
  being frozen after an API response.
- REST refreshes on login, foreground FCM arrival, and when the user opens the bell drawer.
  There is no polling and no WebSocket.
- Every read and token deletion is scoped to the middleware-injected `auth.user_id`.

## Schema

```sql
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    related_entity_id UUID,
    related_entity_type TEXT,
    action_url TEXT,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notif_user_unread ON notifications(user_id, is_read, created_at DESC);
CREATE INDEX idx_notif_user_created ON notifications(user_id, created_at DESC);
```

### `notification_type` values

| Value | Used for |
|---|---|
| `form_approved` | Parent: form approved by admin |
| `form_rejected` | Parent: form rejected (`body` includes reviewer notes) |
| `form_assigned` | Parent: new form assigned to child |
| `form_submitted` | Admins: parent completed a form, needs review |
| `child_added` | Parent + admins: additional child added |
| `child_archived` | Parent + admins: child archived |
| `parent_invited` | Admins: new parent invited |
| `parent_deactivated` | Parent + admins: parent deactivated |
| `admin_added` | Other admins: new admin user added to school |
| `classroom_added` | Admins: classroom created |
| `classroom_deleted` | Admins: classroom deleted |
| `form_template_added` | Admins: new form template added |
| `form_template_deleted` | Admins: form template deleted |

Stored as `TEXT` (not a PG enum) so adding a new type is a backend code change with no
migration.

## REST API

All endpoints behind `jwt_or_api_key_middleware`. Every endpoint scopes by `auth.user_id`.

### `GET /notifications?filter=all|unread|read&limit=20&offset=0`
List the current user's notifications, newest first.
- `filter` defaults to `all`.
- `limit` clamps to `[1, 100]`, default `20`.
- `offset` default `0`.
- Returns:
```json
{
  "items": [ { "id": "...", "notification_type": "form_approved", "title": "...", "body": "...",
               "related_entity_id": "...", "related_entity_type": "form_assignment",
               "action_url": "/dashboard", "is_read": false, "read_at": null,
               "created_at": "2026-06-21T..." } ],
  "total": 47,
  "unread_count": 12
}
```

### `GET /notifications/unread-count`
Fast endpoint for the bell badge.
- Returns `{ "count": 12 }`.

### `PATCH /notifications/:id/read`
Mark a single notification as read. Idempotent. Returns `204 No Content`.

### `PATCH /notifications/mark-all-read`
Mark every unread notification belonging to the current user as read. Returns
`{ "updated": <n> }`.

## Browser push registration

### `POST /device-tokens`
Registers this authenticated browser/device:
```json
{ "token": "fcm-registration-token", "platform": "web" }
```

### `DELETE /device-tokens`
Removes the current user's browser/device registration:
```json
{ "token": "fcm-registration-token" }
```

The token is sent in the body rather than the URL so it is not exposed through browser history
or access logs.

## FCM delivery worker

`database/migrations/019_notification_push_outbox.sql` adds the durable queue. The API Lambda
never waits for a browser push: it commits the bell row and any eligible device-token deliveries
in one transaction. `goddard-<stage>-notification-push-worker` runs once per minute through an
EventBridge rule and processes up to 50 leased rows per invocation.

- success → `sent` with `sent_at`;
- invalid/unregistered token → deleted from `device_tokens`, delivery marked `failed`;
- temporary FCM/network error → retried after 30s, 60s, 120s … up to one hour, with at most 8
  attempts;
- FCM configuration is read only from `FCM_PROJECT_ID`, `FCM_CLIENT_EMAIL`, and
  `FCM_PRIVATE_KEY`, supplied to both Lambdas by `scripts/deploy-aws-dev.sh` /
  `scripts/deploy-aws-prod.sh`.

## Trigger wiring

In every existing service method below, immediately AFTER the DB mutation succeeds we call
the relevant `notification_service.notify_user(...)` and / or
`notification_service.notify_school_admins(...)`. The row is durable even if FCM cannot reach
a device; provider failures are logged and do not fail the originating business operation.

| File | Function | Calls |
|---|---|---|
| `services/student_form_assignment_service.rs` | `review_student_form_assignment` (approved) | `notify_user(form_approved)` |
| same | (rejected) | `notify_user(form_rejected)` (notes embedded in body) |
| same | `create_student_form_assignment`, `bulk_assign_forms`, `assign_form_to_school_students`, `assign_form_to_class_students` | `notify_user(form_assigned)` per new assignment |
| `services/form_submission_service.rs` | wherever a form transitions to `completed` (Fillout webhook) | `notify_school_admins(form_submitted)` |
| `services/enrollment_service.rs` | `add_child` | `notify_user(child_added)` + `notify_school_admins(child_added)` |
| same | `deactivate_parent` | `notify_user(parent_deactivated)` + `notify_school_admins(parent_deactivated)` |
| same | `update_child_status` (archive branch only) | `notify_user(child_archived)` + `notify_school_admins(child_archived)` |
| same | `create_parent_invite` | `notify_school_admins(parent_invited)` |
| `services/auth_service.rs` | admin invite creation | `notify_school_admins(admin_added)` |
| `services/classroom_service.rs` | `create_classroom` / `delete_classroom` | `notify_school_admins(classroom_added / classroom_deleted)` |
| `services/form_template_service.rs` | `create_form_template` / `delete_form_template` | `notify_school_admins(form_template_added / form_template_deleted)` |

## Frontend integration (high level)

See the matching FE PR. Summary:

- `NotificationsProvider` owns the REST list and FCM foreground handler. It does not open a
  WebSocket, run a timer, or call `new Notification()` directly.
- The service worker is the only background browser-notification renderer.
- `<NotificationBell />` lives in three layouts: `Header.tsx` (parent), `AdminLayout.tsx`,
  `SuperAdminLayout.tsx`. Bell shows the badge (capped at "99+").
- Click → opens `<NotificationDrawer />` (Radix Dialog as a right-side panel, 420px wide
  desktop, full-screen on mobile).
- Drawer mirrors the WhatsApp design reference: tabs (All / Unread / Read), grouped sections
  (Today / Yesterday / Earlier), "Mark all as read" link.
- Single source of truth for icon + accent color per `notification_type` lives in
  `notificationMeta.ts` on the FE — backend stores only the discriminator string.

## Manual test plan

1. Apply migration to dev Supabase.
2. cURL each endpoint with a real JWT and a fake user, confirm tenant isolation.
3. Trigger each event end-to-end:
   - Approve / reject a form → parent sees the matching notification.
   - Assign a form (single / bulk / school / class) → each parent sees one
     `form_assigned`.
   - Add a child → parent + every admin sees `child_added`.
   - Archive a child (`{"status":"archive"}`) → parent + admins see `child_archived`.
   - Deactivate a parent → parent + admins.
   - Invite a parent → admins.
   - Add an admin → other admins.
   - Create / delete classroom and form template → admins.
   - Submit a form via the Fillout webhook → admins.
4. Enable browser notifications from Profile. Verify a form/document action sends one FCM
   browser notification in the background and refreshes the bell in the foreground.
5. Sign out and verify the current device token is deregistered using `DELETE /device-tokens`.

## Out of scope (deferred)

- Supabase Realtime channel (replaces polling).
- Per-user notification preferences (mute / snooze).
- Notification grouping ("5 forms assigned to Kavin").
- Notification audit log (who-marked-what-when beyond `read_at`).
