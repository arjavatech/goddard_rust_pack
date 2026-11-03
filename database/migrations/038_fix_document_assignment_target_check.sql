-- Drop the original check constraint that only allowed student or employee targets.
-- Migration 037 added 'document_request_assignments_check' which already covers admin,
-- but forgot to drop this old one, causing admin-only assignments to fail.
ALTER TABLE document_request_assignments
  DROP CONSTRAINT IF EXISTS document_assignment_target_check;
