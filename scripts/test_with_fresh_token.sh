#!/bin/bash
# Complete test with fresh Supabase login

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

echo "=== Testing Email Resend with Fresh JWT ==="
echo ""

# Step 1: Prompt for credentials
read -p "Enter email: " USER_EMAIL
read -sp "Enter password: " USER_PASSWORD
echo ""
echo ""

# Step 2: Login to Supabase and get fresh JWT
echo "1. Logging in to Supabase..."
LOGIN_RESPONSE=$(curl -s -w "\n%{http_code}" \
  "$SUPABASE_URL/auth/v1/token?grant_type=password" \
  -H "Content-Type: application/json" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -d "{
    \"email\": \"$USER_EMAIL\",
    \"password\": \"$USER_PASSWORD\"
  }")

LOGIN_CODE=$(echo "$LOGIN_RESPONSE" | tail -1)
LOGIN_BODY=$(echo "$LOGIN_RESPONSE" | sed '$d')

if [ "$LOGIN_CODE" != "200" ]; then
    echo "❌ Login failed!"
    echo "Response: $LOGIN_BODY"
    exit 1
fi

# Extract access token
ACCESS_TOKEN=$(echo "$LOGIN_BODY" | jq -r '.access_token')
USER_ID=$(echo "$LOGIN_BODY" | jq -r '.user.id')
USER_ROLE=$(echo "$LOGIN_BODY" | jq -r '.user.user_metadata.role')

echo "✅ Login successful!"
echo "User ID: $USER_ID"
echo "Role: $USER_ROLE"
echo ""

# Step 3: Test resend-confirmation endpoint
echo "2. Testing /enrollments/resend-confirmation endpoint..."
RESEND_RESPONSE=$(curl -s -w "\n%{http_code}" \
  "https://goddard.fly.dev/enrollments/resend-confirmation" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -X POST \
  -d '{"parent_id": "f0931979-aaa9-4926-a5ad-8004ae2479e7"}')

RESEND_CODE=$(echo "$RESEND_RESPONSE" | tail -1)
RESEND_BODY=$(echo "$RESEND_RESPONSE" | sed '$d')

echo "HTTP Status: $RESEND_CODE"
echo "Response:"
echo "$RESEND_BODY" | jq '.' 2>/dev/null || echo "$RESEND_BODY"
echo ""

if [ "$RESEND_CODE" == "200" ]; then
    echo "✅ Email resend successful!"
    echo ""
    echo "📧 Check the following email inbox:"
    EMAIL=$(echo "$RESEND_BODY" | jq -r '.parent_details.email')
    echo "   Email: $EMAIL"
    echo ""
    echo "Expected: Password reset/recovery email (since user is already confirmed)"
    echo "Check: Main inbox AND spam folder"
elif [ "$RESEND_CODE" == "401" ]; then
    echo "❌ Authentication failed - JWT might still be invalid"
elif [ "$RESEND_CODE" == "403" ]; then
    echo "❌ Forbidden - User does not have Admin role"
    echo "Current role: $USER_ROLE"
    echo "Required: Admin or SuperAdmin"
else
    echo "❌ Request failed"
fi

echo ""
echo "=== Test Complete ==="
