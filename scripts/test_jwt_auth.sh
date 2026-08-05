#!/bin/bash
# Test JWT authentication for resend-confirmation endpoint

JWT_TOKEN="eyJhbGciOiJIUzI1NiIsImtpZCI6IjFCMkJrWW5jeGpENHVkbzUiLCJ0eXAiOiJKV1QifQ.eyJpc3MiOiJodHRwczovL2Z4c2pjcndzbm5vd2xvdmNuZGR6LnN1cGFiYXNlLmNvL2F1dGgvdjEiLCJzdWIiOiJjYmMxZTc3MS1iNDFiLTRmYzUtYjliNS1hNjUyYmU0YmNiMzAiLCJhdWQiOiJhdXRoZW50aWNhdGVkIiwiZXhwIjoxNzcxMTQ5NTg3LCJpYXQiOjE3NzEwNjMxODcsImVtYWlsIjoiYXJ1bmt1bWFyLmFyamF2YUBnbWFpbC5jb20iLCJwaG9uZSI6IiIsImFwcF9tZXRhZGF0YSI6eyJwcm92aWRlciI6ImVtYWlsIiwicHJvdmlkZXJzIjpbImVtYWlsIl19LCJ1c2VyX21ldGFkYXRhIjp7ImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJmaXJzdF9uYW1lIjoiQXJ1biIsImlzX3ZlcmlmaWVkIjp0cnVlLCJsYXN0X25hbWUiOiJLdW1hciIsInBhc3N3b3JkX3NldCI6dHJ1ZSwicGhvbmVfbnVtYmVyIjoiKzEyMzQ1Njc4OTAiLCJyb2xlIjoiU3VwZXJBZG1pbiIsInNjaG9vbF9pZCI6ImZhYzlkZTVjLWQ5MDItNDIzMy1hYzYwLTU4Njc2MzgxNzZiOSIsInNjaG9vbF9uYW1lIjoiR29kZGFyZCBTY2hvb2xzLCBMeW5ud29vZCJ9LCJyb2xlIjoiYXV0aGVudGljYXRlZCIsImFhbCI6ImFhbDEiLCJhbXIiOlt7Im1ldGhvZCI6InBhc3N3b3JkIiwidGltZXN0YW1wIjoxNzcxMDYzMTg3fV0sInNlc3Npb25faWQiOiI0Mjg3Nzc1Ni0zMDAwLTQ1YWYtYjU5Yi0wMTExYzg0MGJkNjMiLCJpc19hbm9ueW1vdXMiOmZhbHNlfQ.Z_I-Ji8s6lZqHVgfZ2EmmMkJzZmldK_YV0vXHpA34zs"

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

echo "=== Testing JWT Authentication ==="
echo ""

echo "1. Decoding JWT payload (without verification)..."
JWT_PAYLOAD=$(echo "$JWT_TOKEN" | cut -d'.' -f2 | base64 -d 2>/dev/null | jq '.')
echo "$JWT_PAYLOAD"
echo ""

echo "2. Testing Supabase JWT verification endpoint..."
VERIFY_RESPONSE=$(curl -s -w "\n%{http_code}" "$SUPABASE_URL/auth/v1/user" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "apikey: $SUPABASE_ANON_KEY")

VERIFY_CODE=$(echo "$VERIFY_RESPONSE" | tail -1)
VERIFY_BODY=$(echo "$VERIFY_RESPONSE" | sed '$d')

echo "Status Code: $VERIFY_CODE"
echo "Response:"
echo "$VERIFY_BODY" | jq '.' 2>/dev/null || echo "$VERIFY_BODY"
echo ""

if [ "$VERIFY_CODE" != "200" ]; then
    echo "❌ JWT verification failed at Supabase!"
    echo ""
    echo "Possible causes:"
    echo "  1. JWT token expired"
    echo "  2. Invalid token signature"
    echo "  3. Supabase ANON_KEY mismatch"
    echo "  4. Token not issued by this Supabase project"
    echo ""
else
    echo "✅ JWT verification successful at Supabase"
    echo ""
fi

echo "3. Testing API endpoint with JWT Bearer token..."
API_RESPONSE=$(curl -s -w "\n%{http_code}" "https://goddard.fly.dev/enrollments/resend-confirmation" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -X POST \
  -d '{"parent_id": "f0931979-aaa9-4926-a5ad-8004ae2479e7"}')

API_CODE=$(echo "$API_RESPONSE" | tail -1)
API_BODY=$(echo "$API_RESPONSE" | sed '$d')

echo "Status Code: $API_CODE"
echo "Response:"
echo "$API_BODY" | jq '.' 2>/dev/null || echo "$API_BODY"
echo ""

if [ "$API_CODE" == "200" ]; then
    echo "✅ API call successful!"
elif [ "$API_CODE" == "401" ]; then
    echo "❌ Authentication failed"
elif [ "$API_CODE" == "403" ]; then
    echo "❌ Forbidden - User does not have required role"
else
    echo "❌ API call failed with status $API_CODE"
fi

echo ""
echo "=== Diagnostic Complete ==="
