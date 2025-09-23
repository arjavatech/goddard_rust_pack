#!/bin/bash

# Test the Fillout webhook with the new flat payload structure
# All fields are passed directly in the payload (not nested)

echo "Testing Fillout webhook with flat payload structure..."
echo "================================================"

# The webhook now accepts a flat JSON structure where all fields are at the root level
# The service will extract the required IDs and treat the rest as form data

curl -X POST https://goddardlambda.serverlessapollo.com/form-submissions/webhook \
  -H "Content-Type: application/json" \
  -H "X-API-Key: test-owner-key-2024" \
  -d '{
    "school_id": "5ea2e4e9-03b6-4145-813d-da4b1b8f0a46",
    "enrollment_id": "17865420-6ac4-4cbf-be39-8ae72e7f0362",
    "student_form_assignment_id": "55fc165a-6d04-44c4-8452-912f933067d2",
    "form_template_id": "1932164c-e9f7-4c9c-bc56-de0101e185de",
    "fillout_submission_id": "fillout_'$(date +%s)'",
    "student_name": "Arun Paiyan",
    "parent_name": "John Doe",
    "parent_email": "parent@example.com",
    "parent_phone": "555-123-4567",
    "emergency_contact_name": "Jane Doe",
    "emergency_contact_phone": "555-987-6543",
    "allergies": "None",
    "medications": "None",
    "medical_conditions": "None",
    "physician_name": "Dr. Smith",
    "physician_phone": "555-444-3333",
    "insurance_provider": "Blue Cross",
    "insurance_policy_number": "BC123456789",
    "authorized_pickup_1": "Grandmother - Mary Doe",
    "authorized_pickup_2": "Uncle - Bob Doe",
    "photo_permission": true,
    "field_trip_permission": true,
    "medication_administration_permission": false,
    "submission_timestamp": "'$(date -u +"%Y-%m-%dT%H:%M:%SZ")'",
    "form_version": "2.0",
    "ip_address": "192.168.1.100",
    "user_agent": "Fillout/1.0"
  }' -v

echo ""
echo "================================================"
echo "Test complete!"
echo ""
echo "Note: The webhook now processes a flat JSON structure where:"
echo "  - school_id, enrollment_id, student_form_assignment_id, form_template_id are extracted"
echo "  - These IDs are removed from the payload before saving as form_data"
echo "  - All other fields become part of the form_data"
echo "  - Metadata is automatically generated with webhook context"