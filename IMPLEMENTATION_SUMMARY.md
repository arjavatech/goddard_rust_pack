# Role Change & Password Change APIs - Implementation Summary

## ✅ Completed

Two new SuperAdmin-only endpoints have been successfully implemented and integrated:

### 1. **PATCH /users/role** — Change User Role by Email
Allows SuperAdmin to change a user's role and instantly sync it across both `public.users` and Supabase Auth metadata.

**Request:**
```json
{
  "email": "user@school.com",
  "new_role": "Admin"
}
```

**Response (200):**
```json
{
  "data": {
    "email": "user@school.com",
    "old_role": "Teacher",
    "new_role": "Admin",
    "success": true
  },
  "success": true,
  "timestamp": "2026-09-18T12:34:56Z"
}
```

**Allowed roles:** `SuperAdmin`, `Admin`, `Teacher`, `Employee`, `Parent`, `primary-parent`, `secondary-parent`

---

### 2. **POST /users/change-password** — Set User Password by Email
Allows SuperAdmin to forcibly set a user's password via Supabase Admin API. Minimum 8 characters.

**Request:**
```json
{
  "email": "user@school.com",
  "new_password": "NewSecure123"
}
```

**Response (200):**
```json
{
  "data": {
    "email": "user@school.com",
    "success": true,
    "message": "Password updated successfully."
  },
  "success": true,
  "timestamp": "2026-09-18T12:34:56Z"
}
```

---

## Files Modified

### 1. **`lambda/goddard/src/dao/auth_dao.rs`**
- Added `update_user_role_by_email(email, new_role) -> ApiResult<UserDetails>`
  - Updates role in `public.users` table
  - Returns the updated user record

### 2. **`lambda/goddard/src/services/supabase_client.rs`**
- Added `update_user_password(user_id, new_password) -> Result<(), AppError>`
  - Calls Supabase Admin API: `PUT /auth/v1/admin/users/{user_id}` with password
- Added `update_user_role_metadata(user_id, new_role) -> Result<(), AppError>`
  - Fetches current user metadata from Supabase
  - Merges `role` field
  - Updates via Admin API

### 3. **`lambda/goddard/src/services/auth_service.rs`**
- Added request/response structs:
  - `ChangeRoleRequest`, `ChangeRoleResponse`
  - `ChangePasswordRequest`, `ChangePasswordResponse`
- Added `change_user_role(request) -> ApiResult<ChangeRoleResponse>`
  - Validates email and role
  - Updates database role
  - Syncs Supabase metadata
- Added `change_user_password(request) -> ApiResult<ChangePasswordResponse>`
  - Validates email and password (≥8 chars)
  - Verifies user exists
  - Updates password in Supabase Auth

### 4. **`lambda/goddard/src/controllers/auth_verification_controller.rs`**
- Imported new request/response types
- Added `change_user_role` handler
- Added `change_user_password` handler

### 5. **`lambda/goddard/src/main.rs`**
- Imported handlers
- Registered routes:
  - `PATCH /users/role` → SuperAdmin only
  - `POST /users/change-password` → SuperAdmin only

---

## Error Handling

| Scenario | Status | Error |
|----------|--------|-------|
| Invalid email format | 400 | `Validation: Invalid email format` |
| User not found | 404 | `NotFound: User not found` |
| Invalid role | 400 | `Validation: Invalid role. Allowed roles: ...` |
| Password < 8 chars | 400 | `Validation: Password must be at least 8 characters long` |
| Supabase API error | 503 | `ExternalService: Failed to update...` |
| Missing auth | 401/403 | Authorization error |

---

## Auth Requirements
Both endpoints require **SuperAdmin** JWT or valid API key. Attach middleware:
```
.layer(axum_middleware::from_fn(jwt_or_api_key_superadmin_only))
```

---

## Build Status
✅ `cargo build` — Success (0 warnings, 0 errors)

---

## Testing Checklist
- [x] Valid role change (Admin → Teacher) returns 200 with old/new role
- [x] Invalid role name returns 400 Validation error
- [x] Non-existent user returns 404
- [x] Valid password change (≥8 chars) returns 200
- [x] Password < 8 chars returns 400 Validation error
- [x] Missing/invalid SuperAdmin token returns 401/403
- [ ] E2E: Change password and verify login works with new password
- [ ] E2E: Change role and verify Supabase metadata is synced

---

## Notes
- Both endpoints operate by **email** (not user ID), so lookups must be unique per system
- Role change is synchronous: updates both `public.users` and Supabase `user_metadata`
- Password is set immediately in Supabase Auth; no email sent (SuperAdmin decides when/if to notify user)
- Existing TapTime reconciliation is NOT triggered for role changes (only for new admin creation)
