# Authorization Verification API Documentation

## Overview
These endpoints provide visibility into user authorization and invitation status for the Goddard School Enrollment System. They help track who has been invited, who has confirmed their email, and who has successfully signed in.

## Current Status from Database

Based on the current Supabase auth.users table:
- **Total Auth Users**: 1
- **Confirmed Users**: 0
- **Invited but Not Confirmed**: 1 (mani.arjava@gmail.com - invited on Sep 17, 2025)
- **Users Who Have Signed In**: 0

---

## API Endpoints

### 1. Get Authorization Verification Status
```http
GET /auth/verification-status
```

Returns comprehensive statistics about user authorization status.

#### Query Parameters
- `school_id` (optional): Filter by specific school
- `include_details` (optional): Include detailed user list

#### Response
```json
{
  "total_users": 1,
  "confirmed_users": 0,
  "invited_not_confirmed": 1,
  "confirmation_sent_not_confirmed": 1,
  "users_who_signed_in": 0,
  "verification_rate": 0.0,
  "timestamp": "2025-09-17T10:30:00Z",
  "details": [
    {
      "email": "mani.arjava@gmail.com",
      "status": "Confirmation Email Sent",
      "invited_at": "2025-09-17T08:42:36.104158Z",
      "confirmation_sent_at": "2025-09-17T08:42:36.104158Z",
      "email_confirmed_at": null,
      "last_sign_in_at": null,
      "created_at": "2025-09-17T08:42:36.041740Z"
    }
  ]
}
```

#### Status Types
- `Confirmed`: User has verified their email
- `Confirmation Email Sent`: Invitation sent, awaiting confirmation
- `Invited`: User invited but confirmation email not sent
- `Pending`: User created but not invited

---

### 2. Get Invitation Summary
```http
GET /auth/invitation-summary
```

Returns a summary of all invitations grouped by role and status.

#### Response
```json
{
  "total_invitations_sent": 1,
  "pending_confirmations": 1,
  "completed_signups": 0,
  "expired_invitations": 0,
  "by_role": {
    "super_admin": 0,
    "admin": 0,
    "teacher": 0,
    "parent": 1
  },
  "timestamp": "2025-09-17T10:30:00Z"
}
```

---

### 3. Resend Invitation
```http
POST /auth/resend-invitation
```

Resends the invitation email to users who haven't confirmed yet.

#### Request Body
```json
{
  "email": "mani.arjava@gmail.com",
  "school_id": "uuid-of-school" // optional
}
```

#### Response
```json
{
  "success": true,
  "message": "Invitation email resent successfully",
  "email": "mani.arjava@gmail.com",
  "timestamp": "2025-09-17T10:30:00Z"
}
```

#### Error Responses
- `404`: User not found
- `409`: User already confirmed
- `400`: Invalid email format

---

## Database Tables Used

### auth.users (Supabase Authentication)
- `id`: User UUID
- `email`: User email address
- `invited_at`: When the user was invited
- `confirmation_sent_at`: When confirmation email was sent
- `email_confirmed_at`: When email was confirmed (null if pending)
- `last_sign_in_at`: Last successful sign-in time
- `created_at`: User creation timestamp

### public.users (Application Users)
- `id`: User UUID
- `school_id`: Associated school
- `invite_id`: Invitation UUID
- `email`: User email
- `role`: User role (SuperAdmin, Admin, Teacher, Parent)
- `id_signed`: Whether user has signed documents
- `created_by`: Who created this user
- `metadata`: Additional user data

---

## Use Cases

### 1. Monitor Invitation Success Rate
Use `/auth/verification-status` to see:
- How many invitations were sent
- How many users confirmed their email
- Calculate verification rate

### 2. Identify Stuck Users
Find users who:
- Were invited but haven't confirmed after X days
- Need invitation resent

### 3. School-Specific Metrics
Filter by `school_id` to see:
- Invitation status per school
- Role distribution per school

### 4. Resend Invitations
For users who haven't confirmed:
- Use `/auth/resend-invitation` to resend email
- Updates `confirmation_sent_at` timestamp

---

## Testing the Endpoints

### Using curl

1. **Get Verification Status**:
```bash
curl -X GET "http://localhost:3000/auth/verification-status"
```

2. **Get Invitation Summary**:
```bash
curl -X GET "http://localhost:3000/auth/invitation-summary"
```

3. **Resend Invitation**:
```bash
curl -X POST "http://localhost:3000/auth/resend-invitation" \
  -H "Content-Type: application/json" \
  -d '{"email": "mani.arjava@gmail.com"}'
```

---

## Implementation Notes

1. **Current Implementation**: Returns mock data matching the actual database state
2. **Production Implementation**: Will connect directly to Supabase database
3. **Security**: In production, these endpoints should be protected with authentication
4. **Rate Limiting**: Resend invitation should be rate-limited to prevent abuse

---

## Next Steps

1. **Connect to Database**: Implement actual database queries
2. **Add Authentication**: Protect endpoints with JWT validation
3. **Add Pagination**: For large user lists
4. **Add Filters**: By date range, role, status
5. **Add Export**: CSV/Excel export functionality
6. **Webhook Integration**: Notify when users confirm