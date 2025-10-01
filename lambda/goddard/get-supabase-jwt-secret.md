# Getting Supabase JWT Secret

To verify Supabase JWTs in your Rust application, you need the **JWT Secret** from your Supabase project.

## Steps to Get JWT Secret:

1. **Go to Supabase Dashboard**: https://supabase.com/dashboard
2. **Select your project**: `fxsjcrwsnnowlovcnddz`
3. **Navigate to**: Project Settings (⚙️) → API
4. **Copy the JWT Secret**: Look for "JWT Secret" field (it's different from anon/service_role keys)

## Update .env File:

Once you have the JWT secret, update your `.env` file:

```bash
JWT_SECRET=<paste-your-supabase-jwt-secret-here>
```

## Why This is Needed:

The JWT token you provided:
```
eyJhbGciOiJIUzI1NiIsImtpZCI6IjFCMkJrWW5jeGpENHVkbzUiLCJ0eXAiOiJKV1QifQ.eyJpc3MiOiJodHRwczovL2Z4c2pjcndzbm5vd2xvdmNuZGR6LnN1cGFiYXNlLmNvL2F1dGgvdjEiLCJzdWIiOiIzYTAwMmQwMC1hNTBkLTRjNWQtYjEyNi1lMTdhYTNiY2Q3MTEiLCJhdWQiOiJhdXRoZW50aWNhdGVkIiwiZXhwIjoxNzU5MzAzODM5LCJpYXQiOjE3NTkzMDAyMzksImVtYWlsIjoibWFuaS5hcmphdmErMDFAZ21haWwuY29tIiwicGhvbmUiOiIiLCJhcHBfbWV0YWRhdGEiOnsicHJvdmlkZXIiOiJlbWFpbCIsInByb3ZpZGVycyI6WyJlbWFpbCJdfSwidXNlcl9tZXRhZGF0YSI6eyJlbWFpbCI6Im1hbmkuYXJqYXZhKzAxQGdtYWlsLmNvbSIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJmaXJzdF9uYW1lIjoiQXJqYXZhIiwibGFzdF9uYW1lIjoiVGVjaG5vbG9naWVzIEJyaWdodC1CcmFpbnMiLCJwaG9uZV92ZXJpZmllZCI6ZmFsc2UsInJvbGUiOiJBZG1pbiIsInNjaG9vbF9pZCI6Ijg4NjNhMzNjLWFiOTUtNGFiOS04YjQ3LTI0ZTMzYmM1Mjg0OSIsInN1YiI6IjNhMDAyZDAwLWE1MGQtNGM1ZC1iMTI2LWUxN2FhM2JjZDcxMSJ9LCJyb2xlIjoiYXV0aGVudGljYXRlZCIsImFhbCI6ImFhbDEiLCJhbXIiOlt7Im1ldGhvZCI6InBhc3N3b3JkIiwidGltZXN0YW1wIjoxNzU5MzAwMjM5fV0sInNlc3Npb25faWQiOiI1NmE3NjIxZS1lOWI0LTQwZjgtODc2OC1kYjE2N2U2NDZlNDciLCJpc19hbm9ueW1vdXMiOmZhbHNlfQ.exb8TifEU61cFyx8ckk71liNIeXGpQMcObCtvXVczX0
```

Is signed using HMAC-SHA256 (HS256) algorithm with Supabase's JWT secret.

## What I've Updated:

1. **Created Supabase JWT Claims Structure**: Added `SupabaseJwtClaims` and `SupabaseUserMetadata` to handle Supabase's token format
2. **Updated JWT Middleware**: Modified `jwt_or_api_key_middleware` and `jwt_or_api_key_admin_only` to:
   - First try decoding as Supabase JWT
   - Fall back to custom JWT format if Supabase format fails
   - Extract user data from `user_metadata` field
   - Parse role and school_id from the nested structure
3. **Maintained Backward Compatibility**: Your existing custom JWT format will still work
4. **Kept Dual Auth**: Both JWT and API Key authentication methods are supported

## Testing Once JWT_SECRET is Set:

```bash
# Test with Supabase JWT token
curl -X GET http://localhost:9000/parent/3a002d00-a50d-4c5d-b126-e17aa3bcd711 \
  -H "Authorization: Bearer <your-supabase-jwt-token>"
```

## Current Decoded Token Data:

From your token, I can see:
- **User ID (sub)**: `3a002d00-a50d-4c5d-b126-e17aa3bcd711`
- **Email**: `mani.arjava+01@gmail.com`
- **Role**: `Admin` (from user_metadata)
- **School ID**: `8863a33c-ab95-4ab9-8b47-24e33bc52849` (from user_metadata)
- **Issuer**: `https://fxsjcrwsnnowlovcnddz.supabase.co/auth/v1`
- **Expires**: 2025-10-01 (timestamp: 1759303839)
