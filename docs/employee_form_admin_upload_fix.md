# Admin Form PDF Upload Fix: Single-Step Approval for Both Student & Employee Forms

## Summary

Unified admin manual PDF upload behavior for both student and employee forms to use a single-step approval process. When an admin uploads a PDF for either form type, it now immediately sets `status = 'approved'` with approval metadata, eliminating the need for a separate review/approval step.

## Problem (Before)

### Employee Forms
When an admin manually uploaded a PDF:
1. Backend set `status = 'manually_uploaded'`
2. Form appeared in admin review queue (`GET /employee-form-assignments/review-queue`)
3. Form appeared in employee's "form review section" on the portal
4. Admin had to explicitly approve/reject a second time

### Student Forms
When an admin manually uploaded a PDF:
1. Backend set `status = 'manually_uploaded'`
2. Form appeared in admin review queue (`GET /student-form-assignments/review-queue`)
3. Parent saw form in review section
4. Admin had to explicitly approve/reject in a second step

This created an unnecessary two-step workflow when the upload action itself represented the approval.

## Solution

Unified both form types to use a single-step workflow:
1. Admin uploads PDF → `status = 'approved'` immediately
2. Populate `approved_by` with the admin's user ID
3. Populate `approved_on` with current timestamp
4. Form does **not** appear in review queue
5. Form shows as approved on parent/employee portal

When a PDF is removed, approval fields are cleared and status reverts to `incomplete`.

## Files Changed

### Employee Forms

#### 1. `lambda/goddard/src/dao/employee_form_assignment_dao.rs`

**`complete_manual_pdf_upload` method**:
- Added `approved_by: Uuid` parameter
- Changed SQL to set `status = 'approved'`
- Added `approved_by = $8, approved_on = NOW()` to UPDATE

**`remove_manual_pdf` method**:
- Added `approved_by = NULL, approved_on = NULL` to UPDATE SET
- Ensures removal clears approval metadata

#### 2. `lambda/goddard/src/services/employee_service.rs`

**`complete_employee_manual_pdf_upload` method**:
- Added `uploader_id: Uuid` parameter
- Passes it to DAO as `approved_by`

**`upload_employee_manual_pdf` method**:
- Added `uploader_id: Uuid` parameter
- Passes it through to `complete_manual_pdf_upload`

**`get_assignments_by_employee` and `get_assignments_by_school` methods**:
- Removed the line `a.approved_on = None;`
- This was clearing `approved_on` because it was being set to `'manually_uploaded'`
- Now that `approved_on` is correctly populated in the DB, no clearing needed
- S3 URL override for `recent_pdf_link` is still applied (unchanged)

#### 3. `lambda/goddard/src/controllers/employee_controller.rs`

**`employee_manual_pdf_complete_upload` controller**:
- Passes `auth.user_id` to service method

**`upload_employee_manual_pdf` controller**:
- Passes `auth.user_id` to service method

### Student Forms

#### 4. `lambda/goddard/src/dao/student_form_assignment_dao.rs`

**`complete_manual_pdf_upload` method**:
- Added `approved_by: Uuid` parameter
- Changed SQL to set `status = 'approved'`
- Added `approved_by = $9, approved_on = $7` to UPDATE (uses same timestamp as `updated_at`)

**`remove_manual_pdf` method**:
- Added `approved_by = NULL, approved_on = NULL` to UPDATE SET
- Ensures removal clears approval metadata

#### 5. `lambda/goddard/src/services/student_form_assignment_service.rs`

**`complete_manual_pdf_upload` method**:
- Added `uploader_id: Uuid` parameter
- Passes it to DAO as `approved_by`

**`upload_manual_pdf` method**:
- Added `uploader_id: Uuid` parameter
- Passes it through to `complete_manual_pdf_upload`

#### 6. `lambda/goddard/src/controllers/student_form_assignment_manual_pdf_controller.rs`

**`student_manual_pdf_complete_upload` controller**:
- Passes `auth.user_id` to service method

**`upload_student_manual_pdf` controller**:
- Passes `auth.user_id` to service method

## Behavior Comparison

### Employee Forms

| Scenario | Before | After |
|----------|--------|-------|
| Admin uploads PDF | status = `manually_uploaded`, `approved_on = null` | status = `approved`, `approved_on = <timestamp>` |
| Admin visits review queue | Sees uploaded form, must approve again | Form does not appear |
| Employee views portal | Form shows in review section | Form shows as approved |
| Admin removes PDF | status → `incomplete` | status → `incomplete`, approval cleared |

### Student Forms

| Scenario | Before | After |
|----------|--------|-------|
| Admin uploads PDF | status = `manually_uploaded`, `approved_on = null` | status = `approved`, `approved_on = <timestamp>` |
| Admin visits review queue | Sees uploaded form, must approve again | Form does not appear |
| Parent views portal | Form shows in review section | Form shows as approved |
| Admin removes PDF | status → `incomplete` | status → `incomplete`, approval cleared |

## Database Constraint

The existing `CHECK (status IN (..., 'manually_uploaded', ...))` is preserved for backward compatibility:
- Historical records may have `manually_uploaded` status
- The constraint remains valid, though new uploads will be `approved`

## Implementation Notes

### Unified Approach
Both student and employee forms now follow the same single-step approval pattern:
1. Admin uploads PDF → immediate approval
2. `approved_by` is set to the uploader's user ID
3. `approved_on` is set to the current timestamp
4. Form bypasses review queue entirely
5. Future changes to status behavior need only be made in one pattern

### Why Single-Step Makes Sense
The act of uploading a PDF represents an admin's deliberate action to fulfill/complete a form. There is no need for a second approval pass — the upload itself **is** the approval decision.

## Testing

### Employee Forms
1. ✓ Code compiles without errors
2. Upload a PDF from admin for an employee form
3. Verify `GET /employee-form-assignments?employee_id=X` shows `status: "approved"` with correct `approved_on` timestamp
4. Verify form does **not** appear in `GET /employee-form-assignments/review-queue`
5. Verify employee portal does **not** show form in review section
6. Remove the PDF from admin
7. Verify form reverts to `status: "incomplete"` with `approved_by: null`, `approved_on: null`

### Student Forms
1. Upload a PDF from admin for a student form
2. Verify `GET /student-form-assignments?enrollment_id=X` shows `status: "approved"` with correct `approved_on` timestamp
3. Verify form does **not** appear in `GET /student-form-assignments/review-queue`
4. Verify parent portal does **not** show form in review section
5. Remove the PDF from admin
6. Verify form reverts to `status: "incomplete"` with `approved_by: null`, `approved_on: null`
