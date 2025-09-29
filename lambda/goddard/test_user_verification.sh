#!/bin/bash

# Test script for user verification check in /users/me endpoint

echo "Testing /users/me endpoint with user verification check..."
echo

# Replace this with a valid JWT token for testing
# This should be a token for a user that exists in your database
JWT_TOKEN="YOUR_JWT_TOKEN_HERE"

# Test the endpoint
echo "Testing with JWT token..."
curl -X GET \
  http://localhost:9000/users/me \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -v

echo
echo "Test complete!"
echo
echo "Expected behavior:"
echo "- If user is verified (is_verified = true): Return user data"
echo "- If user is not verified (is_verified = false): Return 403 Forbidden with message 'User verification failed. Please verify your account.'"