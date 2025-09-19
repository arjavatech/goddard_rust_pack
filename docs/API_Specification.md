# API Planning Specification - Goddard School Enrollment System

## Authentication & Authorization Framework

### API Key Authorization (Owner Operations)
- Header: `X-API-Key: <owner_api_key>`
- Used for: School creation, SuperAdmin operations
- Validation: Direct comparison with `OWNER_API_KEY` environment variable

### JWT Token Authorization (User Operations)
- Header: `Authorization: Bearer <jwt_token>`
- Contains: `user_id`, `school_id`, `role`, `email`
- Used for: All user-specific operations

---

## 1. School Management APIs

### 1.1 Create School (Protected - API Key)
```
POST /schools
X-API-Key: <owner_api_key>
Content-Type: application/json

Request Body:
{
  "name": "The Goddard School - Downtown",
  "subdomain": "downtown-goddard",
  "settings": {
    "timezone": "America/New_York",
    "max_enrollment": 200,
    "age_groups": ["infant", "toddler", "preschool"]
  }
}

Response (201):
{
  "id": "uuid",
  "name": "The Goddard School - Downtown",
  "subdomain": "downtown-goddard",
  "settings": {...},
  "created_at": "2024-01-15T10:30:00Z"
}
```

### 1.2 Get All Schools (Public)
```
GET /schools
Content-Type: application/json

Response (200):
[
  {
    "id": "uuid",
    "name": "The Goddard School - Downtown",
    "subdomain": "downtown-goddard"
  },
  {
    "id": "uuid",
    "name": "The Goddard School - Uptown",
    "subdomain": "uptown-goddard"
  }
]
```

**Database Query:**
```sql
SELECT id, name, subdomain
FROM schools
WHERE (is_active = true OR is_active IS NULL)
ORDER BY created_at DESC
```

### 1.3 Update School (Protected - API Key Only)
```
PUT /schools
X-API-Key: <owner_api_key>
Content-Type: application/json

Request Body:
{
  "id": "uuid",
  "name": "The Goddard School - Downtown Updated",
  "subdomain": "downtown-goddard-new",
  "settings": {
    "timezone": "America/New_York",
    "max_enrollment": 250,
    "age_groups": ["infant", "toddler", "preschool", "pre-k"]
  }
}

Authorization Logic:
- Extract `X-API-Key` header
- Compare with `OWNER_API_KEY` environment variable
- Allow if keys match exactly
- Reject with 401 if missing/invalid API key

Response (200):
{
  "id": "uuid",
  "name": "The Goddard School - Downtown Updated",
  "subdomain": "downtown-goddard-new",
  "settings": {...},
  "updated_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid school_id
- 401: Invalid API key
- 404: School not found
- 422: Validation errors (e.g., subdomain already exists)
```

**Database Operations:**
```sql
-- Update school record
UPDATE schools
SET name = $2,
    subdomain = $3,
    settings = $4,
    updated_at = NOW()
WHERE id = $1 AND (is_active = true OR is_active IS NULL)
RETURNING id, name, subdomain, settings, updated_at;
```

### 1.4 Delete School (Protected - API Key Only)
```
DELETE /schools/:id
X-API-Key: <owner_api_key>
Content-Type: application/json

Authorization Logic:
- Extract `X-API-Key` header
- Compare with `OWNER_API_KEY` environment variable
- Allow if keys match exactly
- Reject with 401 if missing/invalid API key

Response (200):
{
  "message": "School successfully deleted",
  "school_id": "uuid"
}

Error Responses:
- 400: Invalid school_id
- 401: Invalid API key
- 404: School not found
```

**Database Operations:**
```sql
-- Soft delete school
UPDATE schools
SET is_active = false, updated_at = NOW()
WHERE id = $1;
```

---

## 3. Classroom Management APIs

### 3.1 Create Classroom (Protected - Admin/SuperAdmin)
```
POST /classrooms
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "school_id": "uuid",
  "class_name": "Toddler Room A"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Reject with 403 if insufficient permissions

Response (201):
{
  "id": "uuid",
  "school_id": "uuid",
  "name": "Toddler Room A",
  "age_group": null,
  "capacity": null,
  "enrolled_count": 0,
  "created_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid school_id
- 403: Insufficient permissions
- 422: Validation errors
```

**Database Operations:**
```sql
-- Create classroom record
INSERT INTO classrooms (id, school_id, name, age_group, capacity, enrolled_count)
VALUES (gen_random_uuid(), $1, $2, null, null, 0)
RETURNING id, school_id, name, age_group, capacity, enrolled_count, created_at;
```

### 3.2 Get All Classrooms by School (Protected - School Context)
```
GET /classrooms?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR jwt.school_id === query.school_id
- Reject with 403 if school access denied

Response (200):
[
  {
    "id": "uuid",
    "class_name": "Toddler Room A"
  },
  {
    "id": "uuid",
    "class_name": "Preschool Room B"
  },
  {
    "id": "uuid",
    "class_name": "Infant Care Center"
  }
]

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Database Query:**
```sql
SELECT id, name as class_name
FROM classrooms
WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
ORDER BY name ASC;
```

### 3.3 Update Classroom (Protected - Admin/SuperAdmin)
```
PUT /classrooms
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "school_id": "uuid",
  "class_id": "uuid",
  "class_name": "Updated Toddler Room A"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Verify classroom belongs to school
- Reject with 403 if insufficient permissions

Response (200):
{
  "id": "uuid",
  "school_id": "uuid",
  "name": "Updated Toddler Room A",
  "updated_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid school_id or class_id
- 403: Insufficient permissions
- 404: Classroom not found
- 422: Validation errors
```

**Database Operations:**
```sql
-- Update classroom name
UPDATE classrooms
SET name = $3, updated_at = NOW()
WHERE id = $2 AND school_id = $1 AND (is_active = true OR is_active IS NULL)
RETURNING id, school_id, name, updated_at;
```

### 3.4 Delete Classroom (Protected - Admin/SuperAdmin)
```
DELETE /classrooms?classroom_id=uuid&school_id=uuid
Authorization: Bearer <jwt_token>

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === query.school_id)
- Soft delete by setting is_active = false
- Reject with 403 if insufficient permissions

Response (200):
{
  "message": "Classroom successfully deleted",
  "classroom_id": "uuid",
  "school_id": "uuid"
}

Error Responses:
- 400: Missing or invalid classroom_id or school_id parameters
- 403: Insufficient permissions
- 404: Classroom not found
```

**Database Operations:**
```sql
-- Soft delete specific classroom
UPDATE classrooms
SET is_active = false, updated_at = NOW()
WHERE id = $1 AND school_id = $2;
```

---

## 4. Form Templates Management APIs

### 4.1 Create Form Template (Protected - Admin/SuperAdmin)
```
POST /form-templates
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "school_id": "uuid",
  "form_name": "Student Registration Form",
  "fillout_form_id": "www.google.com"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Reject with 403 if insufficient permissions

Response (201):
{
  "id": "uuid",
  "school_id": "uuid",
  "form_name": "Student Registration Form",
  "form_type": "school_form",
  "fillout_form_id": "www.google.com",
  "fillout_form_url": null,
  "status": "school_default",
  "is_required": null,
  "display_order": null,
  "created_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid school_id
- 403: Insufficient permissions
- 422: Validation errors
```

**Database Operations:**
```sql
-- Create form template record
INSERT INTO form_templates (id, school_id, form_name, form_type, fillout_form_id, status)
VALUES (gen_random_uuid(), $1, $2, 'school_form', $3, 'school_default')
RETURNING id, school_id, form_name, form_type, fillout_form_id, fillout_form_url, status, is_required, display_order, created_at;
```

### 4.2 Get All Form Templates by School (Protected - School Context)
```
GET /form-templates?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR jwt.school_id === query.school_id
- Reject with 403 if school access denied

Response (200):
[
  {
    "id": "uuid",
    "school_id": "uuid",
    "form_name": "Student Registration Form",
    "form_type": "school_form",
    "fillout_form_id": "fillout_123",
    "fillout_form_url": "https://forms.fillout.com/t/abc123",
    "status": "school_default",
    "is_required": true,
    "display_order": 1,
    "created_at": "2024-01-15T10:30:00Z"
  },
  {
    "id": "uuid",
    "school_id": "uuid",
    "form_name": "Medical History Form",
    "form_type": "school_form",
    "fillout_form_id": null,
    "fillout_form_url": null,
    "status": "active",
    "is_required": false,
    "display_order": 2,
    "created_at": "2024-01-14T09:20:00Z"
  }
]

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Database Query:**
```sql
SELECT id, school_id, form_name, form_type, fillout_form_id, fillout_form_url,
       status, is_required, display_order, created_at
FROM form_templates
WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
ORDER BY display_order ASC, created_at DESC;
```

### 4.3 Update Form Template (Protected - Admin/SuperAdmin)
```
PUT /form-templates
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "id": "uuid",
  "school_id": "uuid",
  "form_name": "Updated Student Registration Form",
  "form_type": "school_form",
  "fillout_form_id": "fillout_123",
  "fillout_form_url": "https://forms.fillout.com/t/abc123",
  "status": "active",
  "is_required": true,
  "display_order": 1
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Verify form template belongs to school
- Reject with 403 if insufficient permissions

Response (200):
{
  "id": "uuid",
  "school_id": "uuid",
  "form_name": "Updated Student Registration Form",
  "form_type": "school_form",
  "fillout_form_id": "fillout_123",
  "fillout_form_url": "https://forms.fillout.com/t/abc123",
  "status": "active",
  "is_required": true,
  "display_order": 1,
  "updated_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid form_id or school_id
- 403: Insufficient permissions
- 404: Form template not found
- 422: Validation errors
```

**Database Operations:**
```sql
-- Update form template (all fields can be updated)
UPDATE form_templates
SET form_name = $3,
    form_type = $4,
    fillout_form_id = $5,
    fillout_form_url = $6,
    status = $7,
    is_required = $8,
    display_order = $9,
    updated_at = NOW()
WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)
RETURNING id, school_id, form_name, form_type, fillout_form_id, fillout_form_url, status, is_required, display_order, updated_at;
```

### 4.4 Delete Form Template (Protected - Admin/SuperAdmin)
```
DELETE /form-templates?form_id=uuid&school_id=uuid
Authorization: Bearer <jwt_token>

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === query.school_id)
- Soft delete by setting is_active = false
- Reject with 403 if insufficient permissions

Response (200):
{
  "message": "Form template successfully deleted",
  "form_id": "uuid",
  "school_id": "uuid"
}

Error Responses:
- 400: Missing or invalid form_id or school_id parameters
- 403: Insufficient permissions
- 404: Form template not found
```

**Database Operations:**
```sql
-- Soft delete form template
UPDATE form_templates
SET is_active = false, updated_at = NOW()
WHERE id = $1 AND school_id = $2;
```

### 4.5 Get Form Templates Grouped by Status (Protected - School Context)
```
GET /form-templates/by-status?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR jwt.school_id === query.school_id
- Reject with 403 if school access denied

Response (200):
{
  "all": {
    "admission_form": 1,
    "enrollment_form": 3,
    "parent_handbook": 4,
    "authorization_form": 2
  },
  "active": {
    "admission_form": 1,
    "enrollment_form": 2
  },
  "archive": {
    "parent_handbook": 1
  },
  "default": {
    "enrollment_form": 1,
    "parent_handbook": 3,
    "authorization_form": 2
  },
  "available": {}
}

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Database Query (Form Templates by Status):**
```sql
-- Get form template counts grouped by status
SELECT
    ft.status,
    ft.form_name,
    COUNT(ft.id) as template_count
FROM form_templates ft
WHERE ft.school_id = $1
    AND (ft.is_active = true OR ft.is_active IS NULL)
GROUP BY ft.status, ft.form_name
ORDER BY ft.status, ft.form_name;

-- Alternative: Get all form templates with status grouping
SELECT
    ft.id,
    ft.form_name,
    ft.status,
    ft.form_url,
    ft.created_at
FROM form_templates ft
WHERE ft.school_id = $1
    AND (ft.is_active = true OR ft.is_active IS NULL)
ORDER BY ft.status, ft.form_name;
```

**Business Logic Flow:**
1. **Validate Request**: Check school_id is provided and valid UUID
2. **Authorization Check**: Verify user has access to this school's data
3. **Query Form Templates**: Get all active form templates for the school grouped by status
4. **Count by Status**: Count form templates by name within each individual status
5. **Calculate Combined All**: Sum all status group counts for each form_name to create 'all' group
6. **Format Response**: Build JSON response with status groupings where 'all' combines other groups
7. **Return Grouped Data**: Form templates grouped by status with 'all' as combination of all statuses

**Status Grouping Logic:**
- **all**: Combined sum of all form templates from active, archive, default, and available statuses
- **active**: Form templates with status = 'active'
- **archive**: Form templates with status = 'archive'
- **default**: Form templates with status = 'school_default'
- **available**: Form templates with status = 'available' (or other active status)

**Response Structure Explanation:**
- **Key**: Status name (all, active, archive, default, available)
- **Value**: Object with form_name as key and count as value
- **Count**: Number of form templates of that name with the given status
- **All Count**: Sum of counts from all other status groups for each form_name

**Complex Aggregation Query (Combining All Status Groups):**
```sql
-- Single query to get all data with 'all' as combination of other status groups
WITH status_counts AS (
    -- Get counts for each status group
    SELECT
        ft.form_name,
        ft.status,
        COUNT(*) as template_count
    FROM form_templates ft
    WHERE ft.school_id = $1
        AND (ft.is_active = true OR ft.is_active IS NULL)
        AND ft.status IN ('active', 'archive', 'school_default', 'available')
    GROUP BY ft.form_name, ft.status
),
combined_all AS (
    -- Combine all status counts for 'all' group
    SELECT
        form_name,
        SUM(template_count) as total_count
    FROM status_counts
    GROUP BY form_name
)
SELECT
    json_build_object(
        'all', (
            SELECT COALESCE(json_object_agg(form_name, total_count), '{}')
            FROM combined_all
        ),
        'active', (
            SELECT COALESCE(json_object_agg(form_name, template_count), '{}')
            FROM status_counts
            WHERE status = 'active'
        ),
        'archive', (
            SELECT COALESCE(json_object_agg(form_name, template_count), '{}')
            FROM status_counts
            WHERE status = 'archive'
        ),
        'default', (
            SELECT COALESCE(json_object_agg(form_name, template_count), '{}')
            FROM status_counts
            WHERE status = 'school_default'
        ),
        'available', (
            SELECT COALESCE(json_object_agg(form_name, template_count), '{}')
            FROM status_counts
            WHERE status = 'available'
        )
    ) AS result;
```

**Alternative Simpler Query (Step by Step):**
```sql
-- Step 1: Get all form template counts by status
SELECT
    ft.form_name,
    ft.status,
    COUNT(*) as template_count
FROM form_templates ft
WHERE ft.school_id = $1
    AND (ft.is_active = true OR ft.is_active IS NULL)
GROUP BY ft.form_name, ft.status
ORDER BY ft.form_name, ft.status;

-- Step 2: Application logic combines the results
-- For each form_name, sum all status counts to get 'all' total
-- Group individual status counts into respective status objects
```

**Performance Considerations:**
- Use composite indexes on (school_id, status, is_active) for optimal performance
- Consider caching for frequently accessed schools
- GROUP BY operations should be optimized with proper indexes
- JSON aggregation functions may impact performance on large datasets
- May benefit from materialized views for complex aggregations

**Use Cases:**
- Admin dashboard showing form template distribution
- Form template management and organization
- School-level form template analytics
- Status-based form template reporting
- Template lifecycle management

---

## 5. Class Form Overrides Management APIs

### 5.1 Create Class Form Override (Protected - Admin/SuperAdmin)
```
POST /class-form-overrides
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "school_id": "uuid",
  "classroom_id": "uuid",
  "form_template_id": "uuid"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Verify classroom and form template belong to school
- Reject with 403 if insufficient permissions

Response (201):
{
  "id": "uuid",
  "school_id": "uuid",
  "classroom_id": "uuid",
  "form_template_id": "uuid",
  "action": null,
  "is_required": null,
  "is_active": true,
  "created_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid school_id, classroom_id, or form_template_id
- 403: Insufficient permissions
- 404: Classroom or form template not found
- 409: Override already exists for this classroom/form combination
- 422: Validation errors
```

**Database Operations:**
```sql
-- Create class form override record (action, is_required default to null, is_active defaults to true)
INSERT INTO class_form_overrides (
  id, school_id, classroom_id, form_template_id, action, is_required, created_at, is_active
)
VALUES (
  gen_random_uuid(), $1, $2, $3, null, null, NOW(), true
)
RETURNING id, school_id, classroom_id, form_template_id, action, is_required, is_active, created_at;

-- Validation queries
-- Check classroom belongs to school
SELECT id FROM classrooms WHERE id = $2 AND school_id = $1 AND (is_active = true OR is_active IS NULL);

-- Check form template belongs to school
SELECT id FROM form_templates WHERE id = $3 AND school_id = $1 AND (is_active = true OR is_active IS NULL);

-- Check for existing override (to prevent duplicates)
SELECT id FROM class_form_overrides
WHERE school_id = $1 AND classroom_id = $2 AND form_template_id = $3 AND (is_active = true OR is_active IS NULL);
```

### 5.2 Delete Class Form Override (Protected - Admin/SuperAdmin)
```
DELETE /class-form-overrides?id=uuid
Authorization: Bearer <jwt_token>

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Verify override exists and belongs to user's school
- Allow if role === "SuperAdmin" OR (role === "Admin" AND override.school_id === jwt.school_id)
- Soft delete by setting is_active = false
- Reject with 403 if insufficient permissions

Response (200):
{
  "message": "Class form override successfully deleted",
  "id": "uuid",
  "school_id": "uuid",
  "classroom_id": "uuid",
  "form_template_id": "uuid"
}

Error Responses:
- 400: Missing or invalid id parameter
- 403: Insufficient permissions
- 404: Override not found
```

**Database Operations:**
```sql
-- Get override details for authorization check
SELECT school_id, classroom_id, form_template_id
FROM class_form_overrides
WHERE id = $1 AND (is_active = true OR is_active IS NULL);

-- Soft delete class form override
UPDATE class_form_overrides
SET is_active = false, updated_at = NOW()
WHERE id = $1 AND (is_active = true OR is_active IS NULL)
RETURNING id, school_id, classroom_id, form_template_id;
```

**Business Logic Flow:**

#### 5.1 Create Override:
1. **Validate Input**: Check all required fields are provided
2. **Authorization Check**: Verify admin has permission for this school
3. **Validate References**: Ensure classroom and form template exist and belong to school
4. **Check Duplicates**: Prevent creating duplicate overrides
5. **Create Record**: Insert with null action/is_required, is_active=true
6. **Return Response**: Include all created fields with defaults

#### 5.2 Delete Override:
1. **Validate Input**: Check id parameter is provided
2. **Get Override Details**: Query to get school_id for authorization
3. **Authorization Check**: Verify admin has permission for this school
4. **Soft Delete**: Update is_active to false instead of hard delete
5. **Return Response**: Confirm deletion with override details

**Use Cases:**
- **Create**: Add class-specific form requirements or exclusions
- **Delete**: Remove class-specific overrides (revert to school defaults)
- **Audit Trail**: Maintain history of override changes through soft deletion

---

## 6. Student Form Assignments Management APIs

### 6.1 Create Student Form Assignment (Protected - Admin/SuperAdmin)
```
POST /student-form-assignments
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "school_id": "uuid",
  "enrollment_id": "uuid",
  "child_id": "uuid",
  "form_template_id": "uuid",
  "assignment_source": "school_default",
  "status": "incomplete"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Reject with 403 if insufficient permissions

Response (201):
{
  "id": "uuid",
  "school_id": "uuid",
  "enrollment_id": "uuid",
  "child_id": "uuid",
  "form_template_id": "uuid",
  "assignment_source": "school_default",
  "status": "incomplete",
  "is_required": true,
  "assigned_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid school_id, enrollment_id, child_id, or form_template_id
- 403: Insufficient permissions
- 422: Validation errors
```

**Database Operations:**
```sql
-- Create student form assignment record (default status is 'incomplete')
INSERT INTO student_form_assignments (id, school_id, enrollment_id, child_id, form_template_id, assignment_source, status, is_required, assigned_at)
VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, COALESCE($6, 'incomplete'), COALESCE($7, true), NOW())
RETURNING id, school_id, enrollment_id, child_id, form_template_id, assignment_source, status, is_required, assigned_at;
```

### 6.2 Get All Student Form Assignments by School (Protected - School Context)
```
GET /student-form-assignments?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR jwt.school_id === query.school_id
- Reject with 403 if school access denied

Response (200):
[
  {
    "id": "uuid",
    "school_id": "uuid",
    "enrollment_id": "uuid",
    "child_id": "uuid",
    "form_template_id": "uuid",
    "assignment_source": "school_default",
    "status": "completed",
    "is_required": true,
    "assigned_at": "2024-01-15T10:30:00Z"
  },
  {
    "id": "uuid",
    "school_id": "uuid",
    "enrollment_id": "uuid",
    "child_id": "uuid",
    "form_template_id": "uuid",
    "assignment_source": "class_override",
    "status": "in_progress",
    "is_required": false,
    "assigned_at": "2024-01-14T09:20:00Z"
  }
]

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Database Query:**
```sql
SELECT id, school_id, enrollment_id, child_id, form_template_id, assignment_source, status, is_required, assigned_at
FROM student_form_assignments
WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
ORDER BY assigned_at DESC;
```

### 6.3 Update Student Form Assignment (Protected - Admin/SuperAdmin)
```
PUT /student-form-assignments
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "id": "uuid",
  "school_id": "uuid",
  "enrollment_id": "uuid",
  "child_id": "uuid",
  "form_template_id": "uuid",
  "assignment_source": "manual_assignment",
  "status": "completed",
  "is_required": true
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Verify assignment belongs to school
- Reject with 403 if insufficient permissions

Response (200):
{
  "id": "uuid",
  "school_id": "uuid",
  "enrollment_id": "uuid",
  "child_id": "uuid",
  "form_template_id": "uuid",
  "assignment_source": "manual_assignment",
  "status": "completed",
  "is_required": true,
  "assigned_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid assignment_id or school_id
- 403: Insufficient permissions
- 404: Assignment not found
- 422: Validation errors
```

**Database Operations:**
```sql
-- Update student form assignment (status can be changed to track progress)
UPDATE student_form_assignments
SET enrollment_id = $3,
    child_id = $4,
    form_template_id = $5,
    assignment_source = $6,
    status = $7,
    is_required = $8,
    updated_at = NOW()
WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL)
RETURNING id, school_id, enrollment_id, child_id, form_template_id, assignment_source, status, is_required, assigned_at, updated_at;
```

### 6.4 Delete Student Form Assignment (Protected - Admin/SuperAdmin)
```
DELETE /student-form-assignments?assignment_id=uuid&school_id=uuid
Authorization: Bearer <jwt_token>

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === query.school_id)
- Soft delete by setting is_active = false
- Reject with 403 if insufficient permissions

Response (200):
{
  "message": "Student form assignment successfully deleted",
  "assignment_id": "uuid",
  "school_id": "uuid"
}

Error Responses:
- 400: Missing or invalid assignment_id or school_id parameters
- 403: Insufficient permissions
- 404: Assignment not found
```

**Database Operations:**
```sql
-- Soft delete student form assignment
UPDATE student_form_assignments
SET is_active = false, updated_at = NOW()
WHERE id = $1 AND school_id = $2;
```


## 7. Form Submissions Management APIs (Version Control)

### 7.1 Create Form Submission (Webhook from Fillout)
```
POST /form-submissions/webhook
X-API-Key: <api_key>
Content-Type: application/json

Request Body (from Fillout webhook):
{
  "form_id": "fillout_form_456",
  "school_id": "uuid",
  "enrollment_id": "uuid",
  "student_form_assignment_id": "uuid",
  "form_data": {
    "student_name": "John Doe",
    "parent_email": "parent@example.com",
    "additional_fields": "..."
  },
  "metadata": {
    "form_version": "1.2",
    "submission_ip": "192.168.1.1",
    "user_agent": "Mozilla/5.0",
    "fillout_submission_id": "fillout_123"
  }
}

Authorization Logic:
- Validate API key from X-API-Key header
- Extract school_id from form metadata
- Verify student_form_assignment exists

Response (201):
{
  "id": "uuid",
  "school_id": "uuid",
  "enrollment_id": "uuid",
  "student_form_assignment_id": "uuid",
  "form_template_id": "uuid",
  "fillout_submission_id": "fillout_123",
  "form_data": {
    "student_name": "John Doe",
    "parent_email": "parent@example.com"
  },
  "metadata": {
    "form_version": "1.2",
    "submission_ip": "192.168.1.1"
  },
  "submitted_at": "2024-01-15T09:30:00Z",
  "processed_at": "2024-01-15T09:35:00Z"
}

Error Responses:
- 400: Invalid submission data
- 401: Invalid webhook secret
- 404: Student form assignment not found
- 422: Validation errors
```

**Database Operations:**
```sql
-- Create form submission record
INSERT INTO form_submissions (
  id, school_id, enrollment_id, student_form_assignment_id,
  form_template_id, fillout_submission_id, form_data, metadata,
  submitted_at, processed_at
)
VALUES (
  gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, NOW(), NOW()
)
RETURNING id, school_id, enrollment_id, student_form_assignment_id,
         form_template_id, fillout_submission_id, form_data, metadata,
         submitted_at, processed_at;
```

### 7.2 Get Latest Form Submission (Most Recent Version)
```
GET /form-submissions/latest?school_id=uuid&enrollment_id=uuid&form_template_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Parents can view their own child's submissions
- Admins can view all submissions in their school
- SuperAdmins can view all submissions

Response (200):
{
  "id": "uuid",
  "school_id": "uuid",
  "enrollment_id": "uuid",
  "student_form_assignment_id": "uuid",
  "form_template_id": "uuid",
  "fillout_submission_id": "fillout_123",
  "form_data": {
    "student_name": "John Doe",
    "parent_email": "parent@example.com"
  },
  "metadata": {
    "form_version": "1.2",
    "revision_number": 3
  },
  "submitted_at": "2024-01-15T09:30:00Z",
  "processed_at": "2024-01-15T09:35:00Z"
}

Error Responses:
- 400: Missing required parameters
- 403: Insufficient permissions
- 404: No submission found
```

**Database Query:**
```sql
-- Get the most recent submission for specific school, enrollment, and form template
SELECT id, school_id, enrollment_id, student_form_assignment_id,
       form_template_id, fillout_submission_id, form_data, metadata,
       submitted_at, processed_at
FROM form_submissions
WHERE school_id = $1
  AND enrollment_id = $2
  AND form_template_id = $3
  AND (is_active = true OR is_active IS NULL)
ORDER BY submitted_at DESC
LIMIT 1;
```

### 7.3 Get All Form Submission Versions (Version History)
```
GET /form-submissions/versions?school_id=uuid&enrollment_id=uuid&form_template_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Parents can view their own child's submission history
- Admins can view all submission versions in their school
- SuperAdmins can view all submission versions

Response (200):
[
  {
    "id": "uuid",
    "school_id": "uuid",
    "enrollment_id": "uuid",
    "student_form_assignment_id": "uuid",
    "form_template_id": "uuid",
    "fillout_submission_id": "fillout_125",
    "form_data": {
      "student_name": "John Doe",
      "parent_email": "parent@example.com"
    },
    "metadata": {
      "form_version": "1.2",
      "revision_number": 3,
      "revision_reason": "Updated emergency contact"
    },
    "submitted_at": "2024-01-15T14:30:00Z",
    "processed_at": "2024-01-15T14:35:00Z"
  },
  {
    "id": "uuid",
    "school_id": "uuid",
    "enrollment_id": "uuid",
    "student_form_assignment_id": "uuid",
    "form_template_id": "uuid",
    "fillout_submission_id": "fillout_124",
    "form_data": {
      "student_name": "John Doe",
      "parent_email": "parent@example.com"
    },
    "metadata": {
      "form_version": "1.2",
      "revision_number": 2,
      "revision_reason": "Corrected medical information"
    },
    "submitted_at": "2024-01-15T11:20:00Z",
    "processed_at": "2024-01-15T11:25:00Z"
  },
  {
    "id": "uuid",
    "school_id": "uuid",
    "enrollment_id": "uuid",
    "student_form_assignment_id": "uuid",
    "form_template_id": "uuid",
    "fillout_submission_id": "fillout_123",
    "form_data": {
      "student_name": "John Doe",
      "parent_email": "parent@example.com"
    },
    "metadata": {
      "form_version": "1.1",
      "revision_number": 1,
      "revision_reason": "Initial submission"
    },
    "submitted_at": "2024-01-15T09:30:00Z",
    "processed_at": "2024-01-15T09:35:00Z"
  }
]

Error Responses:
- 400: Missing required parameters
- 403: Insufficient permissions
- 404: No submissions found
```

**Database Query:**
```sql
-- Get all submission versions for specific school, enrollment, and form template
SELECT id, school_id, enrollment_id, student_form_assignment_id,
       form_template_id, fillout_submission_id, form_data, metadata,
       submitted_at, processed_at
FROM form_submissions
WHERE school_id = $1
  AND enrollment_id = $2
  AND form_template_id = $3
  AND (is_active = true OR is_active IS NULL)
ORDER BY submitted_at DESC;
```

### 7.4 Get Form Submission by ID
```
GET /form-submissions/{submission_id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Verify submission belongs to user's school or child
- Apply role-based access control

Response (200):
{
  "id": "uuid",
  "school_id": "uuid",
  "enrollment_id": "uuid",
  "student_form_assignment_id": "uuid",
  "form_template_id": "uuid",
  "fillout_submission_id": "fillout_123",
  "form_data": {
    "student_name": "John Doe",
    "parent_email": "parent@example.com",
    "additional_fields": "..."
  },
  "metadata": {
    "form_version": "1.2",
    "submission_ip": "192.168.1.1"
  },
  "submitted_at": "2024-01-15T09:30:00Z",
  "processed_at": "2024-01-15T09:35:00Z"
}

Error Responses:
- 403: Insufficient permissions
- 404: Submission not found
```

**Database Query:**
```sql
SELECT id, school_id, enrollment_id, student_form_assignment_id,
       form_template_id, fillout_submission_id, form_data, metadata,
       submitted_at, processed_at
FROM form_submissions
WHERE id = $1 AND school_id = $2 AND (is_active = true OR is_active IS NULL);
```

### 7.5 Update Form Submission Status (Protected - Admin/SuperAdmin)
```
PUT /form-submissions/{submission_id}/status
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "processed_at": "2024-01-15T09:35:00Z",
  "processing_notes": "Form reviewed and approved"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === submission.school_id)
- Reject with 403 if insufficient permissions

Response (200):
{
  "id": "uuid",
  "processed_at": "2024-01-15T09:35:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 403: Insufficient permissions
- 404: Submission not found
```

**Database Operations:**
```sql
-- Update form submission processing status
UPDATE form_submissions
SET processed_at = $2,
    metadata = jsonb_set(COALESCE(metadata, '{}'), '{processing_notes}', $3::jsonb),
    updated_at = NOW()
WHERE id = $1 AND (is_active = true OR is_active IS NULL)
RETURNING id, processed_at, updated_at;
```

---

## 8. Enrollment Child-Based Management APIs

### 8.1 Parent Invite for Child Enrollment (Protected - Admin/SuperAdmin)
```
POST /enrollments/parent-invite
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "school_id": "uuid",
  "child_first_name": "John",
  "child_last_name": "Doe",
  "child_birth_date": "2018-05-15",
  "gender": "male",
  "class_id": "uuid",
  "parent_email": "parent@example.com",
  "parent_first_name": "Jane",
  "parent_last_name": "Doe"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Verify classroom belongs to school
- Reject with 403 if insufficient permissions

Response (201):
{
  "parent_id": "uuid",
  "child_id": "uuid",
  "enrollment_id": "uuid",
  "assigned_forms_count": 5,
  "invite_id": "uuid",
  "signup_email_sent": true,
  "message": "Parent invite created successfully and signup email sent",
  "details": {
    "parent": {
      "id": "uuid",
      "school_id": "uuid",
      "first_name": "Jane",
      "last_name": "Doe",
      "email": "parent@example.com",
      "role": "Parent",
      "is_verified": false,
      "created_at": "2024-01-15T10:30:00Z"
    },
    "child": {
      "id": "uuid",
      "parent_id": "uuid",
      "school_id": "uuid",
      "first_name": "John",
      "last_name": "Doe",
      "birth_date": "2018-05-15",
      "gender": "male",
      "status": "active",
      "created_at": "2024-01-15T10:30:00Z"
    },
    "enrollment": {
      "id": "uuid",
      "child_id": "uuid",
      "school_id": "uuid",
      "classroom_id": "uuid",
      "status": "not_completed",
      "application_status": null,
      "created_at": "2024-01-15T10:30:00Z"
    },
    "assigned_forms": [
      {
        "id": "uuid",
        "form_template_id": "uuid",
        "form_name": "Student Registration Form",
        "assignment_source": "school_default",
        "status": "incomplete",
        "is_required": true
      },
      {
        "id": "uuid",
        "form_template_id": "uuid",
        "form_name": "Medical History Form",
        "assignment_source": "class_override",
        "status": "incomplete",
        "is_required": true
      }
    ]
  }
}

Error Responses:
- 400: Invalid request data or missing required fields
- 403: Insufficient permissions
- 404: School or classroom not found
- 409: Parent email already exists for this school
- 422: Validation errors
```

**Database Operations (Transaction):**
```sql
-- Step 1: Create parent user record
INSERT INTO users (id, school_id, invite_id, first_name, last_name, email, role, is_verified, metadata)
VALUES (gen_random_uuid(), $1, gen_random_uuid(), $2, $3, $4, 'Parent', false, '{}')
RETURNING id, school_id, invite_id, first_name, last_name, email, role, is_verified, created_at;

-- Step 2: Create child record (status defaults to 'active')
INSERT INTO children (id, parent_id, school_id, first_name, last_name, birth_date, gender, status)
VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, 'active')
RETURNING id, parent_id, school_id, first_name, last_name, birth_date, gender, status, created_at;

-- Step 3: Create enrollment record
INSERT INTO enrollments (id, child_id, school_id, classroom_id, status, application_status, progress, submitted_at)
VALUES (gen_random_uuid(), $1, $2, $3, 'not_completed', null, '{}', NOW())
RETURNING id, child_id, school_id, classroom_id, status, application_status, created_at;

-- Step 4: Get school default forms
SELECT id, form_name, fillout_form_id, fillout_form_url, is_required
FROM form_templates
WHERE school_id = $1 AND status = 'school_default' AND (is_active = true OR is_active IS NULL);

-- Step 5: Get class-specific form overrides
SELECT form_template_id, action, is_required
FROM class_form_overrides
WHERE school_id = $1 AND classroom_id = $2 AND (is_active = true OR is_active IS NULL);

-- Step 6: Consolidate forms and create student form assignments
-- (This logic consolidates school defaults + class overrides)
INSERT INTO student_form_assignments (
  id, school_id, enrollment_id, child_id, form_template_id,
  assignment_source, status, is_required, assigned_at
)
VALUES
  (gen_random_uuid(), $1, $2, $3, $4, 'school_default', 'incomplete', $5, NOW()),
  (gen_random_uuid(), $1, $2, $3, $6, 'class_override', 'incomplete', $7, NOW())
  -- ... (repeated for each consolidated form)
RETURNING id, form_template_id, assignment_source, status, is_required, assigned_at;

-- Step 7: Send signup invitation email
-- Email service call using Resend API
POST /api/emails/send-invitation
{
  "to": "parent@example.com",
  "template": "parent_signup_invitation",
  "data": {
    "invite_id": "uuid",
    "parent_name": "Jane Doe",
    "child_name": "John Doe",
    "school_name": "Goddard School ABC",
    "signup_url": "https://enrollment.goddardschool.com/signup?invite_id=uuid"
  }
}
```

**Business Logic Flow:**
1. **Validate Input**: Check all required fields and school/classroom existence
2. **Create Parent User**: Insert into users table with role='Parent', is_verified=false
3. **Create Child**: Insert into children table linked to parent
4. **Create Enrollment**: Insert into enrollments with status='not_completed'
5. **Get Default Forms**: Query form_templates for school_default forms
6. **Get Class Overrides**: Query class_form_overrides for specific classroom
7. **Consolidate Forms**: Merge default forms with class-specific overrides
8. **Create Form Assignments**: Insert into student_form_assignments with status='incomplete'
9. **Send Sign-up Email**: Send invitation email to parent using invite_id from users table
10. **Return Complete Response**: Include all created records and assigned forms

### 8.2 Resend Parent Confirmation Email (Protected - API Key)
```
POST /enrollments/resend-confirmation
X-API-Key: <owner_api_key>
Content-Type: application/json

Request Body:
{
  "parent_id": "uuid"
}

Authorization Logic:
- Extract X-API-Key header
- Compare with OWNER_API_KEY environment variable
- Allow if keys match exactly
- Reject with 401 if missing/invalid API key
- parent_id must match Supabase auth table ID (not local users table ID)

Response (200):
{
  "parent_id": "uuid",
  "email_sent": true,
  "message": "Confirmation email resent successfully",
  "parent_details": {
    "email": "parent@example.com"
  }
}

Error Responses:
- 400: Invalid parent_id or parent not found in Supabase auth
- 401: Invalid API key
- 422: Validation errors
- 500: Email service error
```

**Business Logic Flow:**
1. **Validate Input**: Check parent_id is provided and valid UUID
2. **API Key Authorization**: Verify request has valid owner API key
3. **Supabase Auth Lookup**: Use parent_id to find user in Supabase auth table directly
4. **Resend Confirmation**: Call Supabase resend endpoint with parent email
5. **Return Success**: Confirm email was sent with parent email details

**Supabase Integration:**
- Use existing SupabaseClient.resend_invitation() method
- parent_id corresponds to Supabase auth user ID (not local users.id)
- No school_id filtering required - works directly with Supabase auth
- Leverages Supabase's built-in email confirmation system

**Email Service Integration:**
```json
POST /emails/send-invitation-reminder
{
  "to": "parent@example.com",
  "subject": "Invitation to Create an Account for The Goddard School Admission",
  "template": "parent_signup_invitation_reminder",
  "data": {
    "parent_name": "Jane Doe",
    "child_name": "John Doe",
    "school_name": "The Goddard School",
    "invite_id": "uuid",
    "signup_url": "https://enrollment.goddardschool.com/signup?invite_id=uuid"
  },
  "html_content": "<html><body style=\"font-family: Arial, sans-serif; line-height: 1; color: #333;\"><div style=\"max-width: 500px; margin: auto; padding: 0px 15px; border: 1px solid #e0e0e0; border-radius: 8px;\"><p>Dear {parent_name},</p><p>We hope this message finds you well. We are pleased to inform you that your request to enroll your son, <strong>{child_name}</strong>, at <strong>The Goddard School</strong> has been received and approved for the next stage of the admission process.<br><br>To facilitate the admission process, we have created a secure and user-friendly online portal. We kindly request you to create an account on our admission website, where you can complete your son's details and proceed with the application.</p><p style=\"text-align: center;\"><a href=\"{signup_url}\" style=\"display: inline-block; padding: 10px 20px; margin: 10px 0; background-color: #4CAF50; color: white; text-decoration: none; border-radius: 5px;\">Create Your Account</a></p><p>Once your account is created, you will be guided through the steps to submit all necessary information and documents. Should you have any questions or require assistance during the process, our support team is available to help.<br><br>Thank you for choosing <strong>The Goddard School</strong> for your son's education. We look forward to welcoming him to our school community.</p><p>Warm regards,<br>Admin Team,<br><strong>The Goddard School</strong></p></div></body></html>"
}
```

**Business Logic Flow:**
1. **Validate Request**: Check parent_id and school_id are provided
2. **Verify Parent Exists**: Query users table to confirm parent exists and is_verified = false
3. **Get Child Details**: Retrieve child information for email personalization
4. **Check Authorization**: Ensure admin has permission to resend for this school
5. **Update Tracking**: Optionally update metadata with last_invite_sent timestamp
6. **Send Reminder Email**: Use same email template as parent-invite with identical subject and content
7. **Return Success Response**: Confirm email was sent with parent/child details

**Use Cases:**
- Parent didn't receive original invitation email
- Parent lost or deleted original invitation
- Invitation email went to spam/junk folder
- Admin wants to send reminder after period of inactivity
- Parent requested new invitation link

### 8.3 Add Additional Child (Protected - Admin/SuperAdmin)
```
POST /enrollments/add-child
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "school_id": "uuid",
  "child_first_name": "Jane",
  "child_last_name": "Doe",
  "child_birth_date": "2020-03-10",
  "gender": "female",
  "class_id": "uuid",
  "parent_id": "uuid"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Verify parent exists in users table with role='Parent'
- Verify classroom belongs to school
- Reject with 403 if insufficient permissions

Response (201):
{
  "child_id": "uuid",
  "enrollment_id": "uuid",
  "assigned_forms_count": 5,
  "message": "Additional child added successfully",
  "details": {
    "parent": {
      "id": "uuid",
      "first_name": "Jane",
      "last_name": "Doe",
      "email": "parent@example.com",
      "is_verified": true
    },
    "child": {
      "id": "uuid",
      "parent_id": "uuid",
      "school_id": "uuid",
      "first_name": "Jane",
      "last_name": "Doe",
      "birth_date": "2020-03-10",
      "gender": "female",
      "status": "active",
      "created_at": "2024-01-15T10:30:00Z"
    },
    "enrollment": {
      "id": "uuid",
      "child_id": "uuid",
      "school_id": "uuid",
      "classroom_id": "uuid",
      "status": "not_completed",
      "application_status": null,
      "created_at": "2024-01-15T10:30:00Z"
    },
    "assigned_forms": [
      {
        "id": "uuid",
        "form_template_id": "uuid",
        "form_name": "Student Registration Form",
        "assignment_source": "school_default",
        "status": "incomplete",
        "is_required": true
      },
      {
        "id": "uuid",
        "form_template_id": "uuid",
        "form_name": "Medical History Form",
        "assignment_source": "class_override",
        "status": "incomplete",
        "is_required": true
      }
    ]
  }
}

Error Responses:
- 400: Invalid request data or missing required fields
- 403: Insufficient permissions
- 404: Parent, school or classroom not found
- 422: Validation errors
```

**Database Operations (Transaction):**
```sql
-- Step 1: Verify parent exists and get parent details
SELECT id, first_name, last_name, email, is_verified
FROM users
WHERE id = $1 AND school_id = $2 AND role = 'Parent';

-- Step 2: Create child record (status defaults to 'active')
INSERT INTO children (id, parent_id, school_id, first_name, last_name, birth_date, gender, status)
VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, 'active')
RETURNING id, parent_id, school_id, first_name, last_name, birth_date, gender, status, created_at;

-- Step 3: Create enrollment record
INSERT INTO enrollments (id, child_id, school_id, classroom_id, status, application_status, progress, submitted_at)
VALUES (gen_random_uuid(), $1, $2, $3, 'not_completed', null, '{}', NOW())
RETURNING id, child_id, school_id, classroom_id, status, application_status, created_at;

-- Step 4: Get school default forms
SELECT id, form_name, fillout_form_id, fillout_form_url, is_required
FROM form_templates
WHERE school_id = $1 AND status = 'school_default' AND (is_active = true OR is_active IS NULL);

-- Step 5: Get class-specific form overrides
SELECT form_template_id, action, is_required
FROM class_form_overrides
WHERE school_id = $1 AND classroom_id = $2 AND (is_active = true OR is_active IS NULL);

-- Step 6: Consolidate forms and create student form assignments
-- (This logic consolidates school defaults + class overrides)
INSERT INTO student_form_assignments (
  id, school_id, enrollment_id, child_id, form_template_id,
  assignment_source, status, is_required, assigned_at
)
VALUES
  (gen_random_uuid(), $1, $2, $3, $4, 'school_default', 'incomplete', $5, NOW()),
  (gen_random_uuid(), $1, $2, $3, $6, 'class_override', 'incomplete', $7, NOW())
  -- ... (repeated for each consolidated form)
RETURNING id, form_template_id, assignment_source, status, is_required, assigned_at;
```

**Business Logic Flow:**
1. **Validate Input**: Check all required fields and verify parent exists
2. **Verify Parent**: Query users table to confirm parent exists with role='Parent'
3. **Create Child**: Insert into children table linked to existing parent
4. **Create Enrollment**: Insert into enrollments with status='not_completed'
5. **Get Default Forms**: Query form_templates for school_default forms
6. **Get Class Overrides**: Query class_form_overrides for specific classroom
7. **Consolidate Forms**: Merge default forms with class-specific overrides
8. **Create Form Assignments**: Insert into student_form_assignments with status='incomplete'
9. **Return Complete Response**: Include child, enrollment and assigned forms (no email sent)

**Key Differences from 8.1:**
- No user creation (parent already exists)
- No invite_id generation
- No email notification sent
- Parent verification status can be either true or false
- Simpler transaction without user table INSERT

### 8.4 Get All Enrollment Form Details by School (Protected - School Context)
```
GET /enrollments/school-forms?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR jwt.school_id === query.school_id
- Reject with 403 if school access denied

Response (200):
[
  {
    "child_id": 4,
    "child_first_name": "Raja",
    "child_last_name": "Ragu",
    "class_name": "Butterfly",
    "primary_email": "pit@gmail.com",
    "form_status": "Incomplete",
    "forms": {
      "1": "admission_form",
      "2": "authorization_form",
      "4": "parent_handbook"
    },
    "additional_parent_email": null
  },
  {
    "child_id": 7,
    "child_first_name": "Mani",
    "child_last_name": "Ragu",
    "class_name": "Butterfly",
    "primary_email": "pitcniece@gmail.com",
    "form_status": "Incomplete",
    "forms": {
      "1": "admission_form",
      "2": "authorization_form",
      "3": "enrollment_form"
    },
    "additional_parent_email": "pitcniece@gmail.com"
  },
  {
    "child_id": 9,
    "child_first_name": "Mani",
    "child_last_name": "PM",
    "class_name": "Butterfly",
    "primary_email": "pitchumaniece@gmail.com",
    "form_status": "Completed",
    "forms": {
      "1": "admission_form",
      "3": "enrollment_form",
      "4": "parent_handbook"
    },
    "additional_parent_email": "ni@arvatech.com"
  }
  // ... more enrollment records
]

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Database Query (Complex Join):**
```sql
-- Main query to get all enrollment form details for a school
SELECT DISTINCT
    c.id AS child_id,
    c.first_name AS child_first_name,
    c.last_name AS child_last_name,
    cl.name AS class_name,
    u1.email AS primary_email,
    u2.email AS additional_parent_email,
    e.status AS form_status,
    -- Aggregate forms as JSON object
    (
        SELECT jsonb_object_agg(
            ft.id::text,
            ft.form_name
        )
        FROM student_form_assignments sfa
        INNER JOIN form_templates ft ON sfa.form_template_id = ft.id
        WHERE sfa.enrollment_id = e.id
        AND sfa.child_id = c.id
        AND (sfa.is_active = true OR sfa.is_active IS NULL)
    ) AS forms
FROM children c
INNER JOIN enrollments e ON c.id = e.child_id
INNER JOIN classrooms cl ON e.classroom_id = cl.id
INNER JOIN users u1 ON c.parent_id = u1.id
LEFT JOIN users u2 ON c.secondary_parent_id = u2.id
WHERE c.school_id = $1
    AND c.status = 'active'
    AND (e.is_active = true OR e.is_active IS NULL)
ORDER BY c.id;
```

**Alternative Query Structure (if jsonb_object_agg not available):**
```sql
-- Step 1: Get enrollment details
SELECT
    c.id AS child_id,
    c.first_name AS child_first_name,
    c.last_name AS child_last_name,
    cl.name AS class_name,
    u1.email AS primary_email,
    u2.email AS additional_parent_email,
    e.status AS form_status,
    e.id AS enrollment_id
FROM children c
INNER JOIN enrollments e ON c.id = e.child_id
INNER JOIN classrooms cl ON e.classroom_id = cl.id
INNER JOIN users u1 ON c.parent_id = u1.id
LEFT JOIN users u2 ON c.secondary_parent_id = u2.id
WHERE c.school_id = $1
    AND c.status = 'active'
    AND (e.is_active = true OR e.is_active IS NULL)
ORDER BY c.id;

-- Step 2: Get forms for each enrollment (executed for each enrollment_id)
SELECT
    ft.id::text AS form_id,
    ft.form_name
FROM student_form_assignments sfa
INNER JOIN form_templates ft ON sfa.form_template_id = ft.id
WHERE sfa.enrollment_id = $1
    AND (sfa.is_active = true OR sfa.is_active IS NULL);
```

**Business Logic Flow:**
1. **Validate Request**: Check school_id is provided and valid
2. **Authorization Check**: Verify user has access to this school's data
3. **Query Enrollments**: Join children, enrollments, classrooms, and users tables
4. **Get Primary Parent Email**: From parent_id → users table
5. **Get Secondary Parent Email**: From secondary_parent_id → users table (if exists)
6. **Get Classroom Name**: From enrollment → classroom_id → classrooms table
7. **Get Form Status**: From enrollments.status field
8. **Aggregate Forms**: Query student_form_assignments for each enrollment
9. **Format Response**: Build JSON response matching exact output format
10. **Return Complete List**: All enrollments with their form details

**Response Field Mappings:**
- `child_id`: children.id
- `child_first_name`: children.first_name
- `child_last_name`: children.last_name
- `class_name`: classrooms.name (via enrollment.classroom_id)
- `primary_email`: users.email (via children.parent_id)
- `additional_parent_email`: users.email (via children.secondary_parent_id, nullable)
- `form_status`: enrollments.status
- `forms`: Aggregated from student_form_assignments → form_templates

**Performance Considerations:**
- Use indexes on school_id, enrollment_id, child_id
- Consider caching for frequently accessed schools
- May need pagination for schools with many enrollments
- Forms aggregation can be done in application layer if DB performance is an issue

### 8.5 Get Class-wise Child Count Details (Protected - School Context)
```
GET /enrollments/class-wise-count?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR jwt.school_id === query.school_id
- Reject with 403 if school access denied

Response (200):
[
  {
    "class_id": 1,
    "class_name": "Butterfly",
    "count": 19,
    "forms": {},
    "default_forms": "[\"admission_form\", \"authorization_form\", \"enrollment_form\", \"parent_handbook\"]"
  },
  {
    "class_id": 2,
    "class_name": "Rainbow",
    "count": 5,
    "forms": {},
    "default_forms": "[\"admission_form\", \"authorization_form\", \"enrollment_form\", \"parent_handbook\"]"
  },
  {
    "class_id": 9,
    "class_name": "Math",
    "count": 4,
    "forms": {},
    "default_forms": "[\"admission_form\", \"authorization_form\", \"enrollment_form\", \"parent_handbook\"]"
  }
]

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Database Query (Complex Multi-step):**
```sql
-- Step 1: Get all classrooms with enrollment counts
SELECT
    c.id AS class_id,
    c.name AS class_name,
    COUNT(e.id) AS count
FROM classrooms c
LEFT JOIN enrollments e ON c.id = e.classroom_id
    AND (e.is_active = true OR e.is_active IS NULL)
WHERE c.school_id = $1
    AND (c.is_active = true OR c.is_active IS NULL)
GROUP BY c.id, c.name
ORDER BY c.id;

-- Step 2: Get school default forms (forms with status = 'school_default')
SELECT
    id,
    form_name
FROM form_templates
WHERE school_id = $1
    AND status = 'school_default'
    AND (is_active = true OR is_active IS NULL)
ORDER BY id;

-- Step 3: Get class-specific form overrides for each classroom
SELECT
    classroom_id,
    form_template_id,
    ft.form_name,
    cfo.action,
    cfo.is_required
FROM class_form_overrides cfo
INNER JOIN form_templates ft ON cfo.form_template_id = ft.id
WHERE cfo.school_id = $1
    AND cfo.classroom_id = $2  -- Execute for each classroom
    AND (cfo.is_active = true OR cfo.is_active IS NULL)
    AND (ft.is_active = true OR ft.is_active IS NULL);
```

**Alternative Single Query Approach:**
```sql
-- Combined query to get all data in one go
SELECT
    c.id AS class_id,
    c.name AS class_name,
    COUNT(DISTINCT e.id) AS count,
    -- Get default forms as JSON array
    (
        SELECT jsonb_agg(ft.form_name ORDER BY ft.id)
        FROM form_templates ft
        WHERE ft.school_id = $1
        AND ft.status = 'school_default'
        AND (ft.is_active = true OR ft.is_active IS NULL)
    ) AS default_forms,
    -- Get class-specific overrides as JSON object
    COALESCE(
        (
            SELECT jsonb_object_agg(
                ft2.id::text,
                jsonb_build_object(
                    'form_name', ft2.form_name,
                    'action', cfo.action,
                    'is_required', cfo.is_required
                )
            )
            FROM class_form_overrides cfo
            INNER JOIN form_templates ft2 ON cfo.form_template_id = ft2.id
            WHERE cfo.school_id = $1
            AND cfo.classroom_id = c.id
            AND (cfo.is_active = true OR cfo.is_active IS NULL)
            AND (ft2.is_active = true OR ft2.is_active IS NULL)
        ),
        '{}'::jsonb
    ) AS forms
FROM classrooms c
LEFT JOIN enrollments e ON c.id = e.classroom_id
    AND (e.is_active = true OR e.is_active IS NULL)
WHERE c.school_id = $1
    AND (c.is_active = true OR c.is_active IS NULL)
GROUP BY c.id, c.name
ORDER BY c.id;
```

**Business Logic Flow:**
1. **Validate Request**: Check school_id is provided and valid
2. **Authorization Check**: Verify user has access to this school's data
3. **Get Classrooms**: Query all classrooms for the school
4. **Count Enrollments**: Count active enrollments for each classroom
5. **Get Default Forms**: Query form_templates with status='school_default'
6. **Get Class Overrides**: Query class_form_overrides for each classroom
7. **Format Response**: Build JSON response matching exact output format
8. **Return Class Data**: All classrooms with counts and form details

**Response Field Details:**
- `class_id`: Integer ID from classrooms table
- `class_name`: Name from classrooms table
- `count`: Number of active enrollments in this classroom
- `forms`: JSON object of class-specific form overrides (currently empty {})
- `default_forms`: JSON string array of school default form names

**Form Logic Explanation:**
- **Default Forms**: Forms with `status = 'school_default'` from form_templates
- **Class Override Forms**: Forms from class_form_overrides that modify defaults for specific classes
- **Forms Object**: Currently returns empty `{}` but can be populated with class-specific overrides

**Performance Considerations:**
- Use indexes on school_id, classroom_id, status fields
- Consider caching default forms since they're the same for all classes
- GROUP BY and COUNT operations should be optimized with proper indexes
- JSON aggregation functions may impact performance on large datasets


### 8.6 Get Enrollment Children with Form Assignments (Protected - School Context)
```
GET /enrollments/children-forms?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR jwt.school_id === query.school_id
- Reject with 403 if school access denied

Response (200):
[
  {
    "child_id": 31,
    "child_first_name": "krithi",
    "child_last_name": "AS",
    "class_name": "Math",
    "primary_email": "logioffical1234@gmail.com",
    "form_status": "Incomplete",
    "forms": {
      "1": "admission_form",
      "2": "authorization_form",
      "3": "enrollment_form",
      "4": "parent_handbook"
    },
    "additional_parent_email": "cxbnm,."
  },
  {
    "child_id": 32,
    "child_first_name": "Rathina",
    "child_last_name": "Surya",
    "class_name": "Rainbow",
    "primary_email": "logioffical1234@gmail.com",
    "form_status": "Incomplete",
    "forms": {
      "1": "admission_form",
      "2": "authorization_form",
      "3": "enrollment_form",
      "4": "parent_handbook"
    },
    "additional_parent_email": null
  }
]

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Database Query (Complex Join with Form Aggregation):**
```sql
-- Main query to get enrollment children with form assignments
SELECT DISTINCT
    c.id AS child_id,
    c.first_name AS child_first_name,
    c.last_name AS child_last_name,
    cl.name AS class_name,
    u1.email AS primary_email,
    u2.email AS additional_parent_email,
    e.status AS form_status,
    -- Aggregate forms as JSON object with form_template_id as key and form_name as value
    (
        SELECT jsonb_object_agg(
            ft.id::text,
            ft.form_name
        )
        FROM student_form_assignments sfa
        INNER JOIN form_templates ft ON sfa.form_template_id = ft.id
        WHERE sfa.enrollment_id = e.id
        AND sfa.child_id = c.id
        AND (sfa.is_active = true OR sfa.is_active IS NULL)
        AND (ft.is_active = true OR ft.is_active IS NULL)
    ) AS forms
FROM enrollments e
INNER JOIN children c ON e.child_id = c.id
INNER JOIN classrooms cl ON e.classroom_id = cl.id
INNER JOIN users u1 ON c.parent_id = u1.id
LEFT JOIN users u2 ON c.secondary_parent_id = u2.id
WHERE e.school_id = $1
    AND (e.is_active = true OR e.is_active IS NULL)
    AND c.status = 'active'
    AND (c.is_active = true OR c.is_active IS NULL)
    AND (cl.is_active = true OR cl.is_active IS NULL)
ORDER BY c.id;
```

**Alternative Multi-Step Query Approach:**
```sql
-- Step 1: Get enrollment and children data
SELECT
    e.id AS enrollment_id,
    c.id AS child_id,
    c.first_name AS child_first_name,
    c.last_name AS child_last_name,
    cl.name AS class_name,
    u1.email AS primary_email,
    u2.email AS additional_parent_email,
    e.status AS form_status
FROM enrollments e
INNER JOIN children c ON e.child_id = c.id
INNER JOIN classrooms cl ON e.classroom_id = cl.id
INNER JOIN users u1 ON c.parent_id = u1.id
LEFT JOIN users u2 ON c.secondary_parent_id = u2.id
WHERE e.school_id = $1
    AND (e.is_active = true OR e.is_active IS NULL)
    AND c.status = 'active'
    AND (c.is_active = true OR c.is_active IS NULL)
    AND (cl.is_active = true OR cl.is_active IS NULL)
ORDER BY c.id;

-- Step 2: Get forms for each enrollment (executed for each enrollment_id)
SELECT
    ft.id::text AS form_id,
    ft.form_name
FROM student_form_assignments sfa
INNER JOIN form_templates ft ON sfa.form_template_id = ft.id
WHERE sfa.enrollment_id = $1
    AND sfa.child_id = $2
    AND (sfa.is_active = true OR sfa.is_active IS NULL)
    AND (ft.is_active = true OR ft.is_active IS NULL);
```

**Business Logic Flow:**
1. **Validate Request**: Check school_id is provided and valid
2. **Authorization Check**: Verify user has access to this school's data
3. **Filter Enrollments**: Get all active enrollments for the school
4. **Fetch Child Details**: Join with children table for names
5. **Get Primary Parent Email**: From children.parent_id → users.email
6. **Get Secondary Parent Email**: From children.secondary_parent_id → users.email (if exists)
7. **Get Classroom Name**: From enrollments.classroom_id → classrooms.name
8. **Get Form Status**: From enrollments.status field
9. **Aggregate Forms**: Query student_form_assignments → form_templates for each child
10. **Return Complete List**: All enrollments with children and form details

**Data Flow Process:**
1. **enrollments** table → filter by school_id and active status
2. **children** table → get child names via enrollment.child_id
3. **users** table → get parent emails via children.parent_id and secondary_parent_id
4. **classrooms** table → get class name via enrollment.classroom_id
5. **student_form_assignments** → get assigned forms per enrollment/child
6. **form_templates** → get form names from form IDs

**Response Field Mappings:**
- `child_id`: children.id
- `child_first_name`: children.first_name
- `child_last_name`: children.last_name
- `class_name`: classrooms.name (via enrollment.classroom_id)
- `primary_email`: users.email (via children.parent_id)
- `additional_parent_email`: users.email (via children.secondary_parent_id, nullable)
- `form_status`: enrollments.status
- `forms`: Aggregated JSON object from student_form_assignments → form_templates

**Performance Considerations:**
- Use indexes on school_id, child_id, enrollment_id, parent_id, secondary_parent_id
- Consider caching for frequently accessed schools
- JSON aggregation in database vs application layer based on performance
- May need pagination for schools with large enrollments
- Filter by active status across all joined tables

### 8.7 Get Parent Details by School (Protected - School Context)
```
GET /parents/details?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR jwt.school_id === query.school_id
- Reject with 403 if school access denied

Response (200):
[
  {
    "parent_id": "uuid-123",
    "parent_email": "parent1@example.com",
    "id_signed": true,
    "created_at": "2024-01-15T10:30:00Z"
  },
  {
    "parent_id": "uuid-456",
    "parent_email": "parent2@example.com",
    "id_signed": false,
    "created_at": "2024-01-20T14:45:00Z"
  },
  {
    "parent_id": "uuid-789",
    "parent_email": "parent3@example.com",
    "id_signed": true,
    "created_at": "2024-01-25T09:15:00Z"
  }
]

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Business Logic Flow:**
1. **Validate Request**: Check school_id is provided and valid UUID
2. **Authorization Check**: Verify user has access to this school's data
3. **Get Local Parents**: Query users table filtered by school_id and role = 'Parent'
4. **Get Auth Details**: For each parent_id, query Supabase auth table as User UID
5. **Check Sign-in Status**: If last_sign_in_at is empty/null then id_signed = false, otherwise true
6. **Return Parent List**: List of parents with email, created_at from auth table, and calculated id_signed

**Database Operations:**
```sql
-- Step 1: Get parents from local users table
SELECT id, email
FROM users
WHERE school_id = $1
    AND role = 'Parent'
    AND (is_active = true OR is_active IS NULL)
ORDER BY created_at DESC;

-- Step 2: For each parent_id, query Supabase auth table
-- This is done via Supabase Admin API call for each parent_id as User UID
GET /auth/v1/admin/users/{parent_id}
```

**Supabase Auth Integration:**
```sql
-- Supabase auth.users table structure (accessed via Admin API)
{
  "id": "uuid",                    -- matches our users.id
  "email": "parent@example.com",   -- auth email
  "created_at": "2024-01-15T10:30:00Z",
  "last_sign_in_at": "2024-01-20T15:45:00Z" or null,
  ...
}
```

**Response Logic:**
- `parent_id`: From local users.id (same as Supabase auth.users.id)
- `parent_email`: From Supabase auth.users.email
- `created_at`: From Supabase auth.users.created_at
- `id_signed`:
  - `false` if auth.users.last_sign_in_at is null or empty
  - `true` if auth.users.last_sign_in_at has a value

**Key Implementation Details:**
- Uses parent_id from local users table as User UID for Supabase auth lookup
- Each parent requires individual Supabase Admin API call
- Filters by role = 'Parent' (not 'primary-parent')
- All data except id_signed comes from Supabase auth table
- id_signed is calculated based on last_sign_in_at field presence

**Use Cases:**
- Admin dashboard to view all parents in a school with auth status
- Parent sign-in tracking and verification
- Onboarding status monitoring
- Email communication lists with verification status
- School enrollment reporting with auth metrics

**Performance Considerations:**
- Multiple Supabase API calls (one per parent) - consider caching
- Use batch requests if Supabase supports it
- Consider pagination for schools with many parents
- Cache results for frequently accessed schools
- Monitor Supabase API rate limits

### 8.3 Add Additional Child to Existing Parent (Protected - Admin/SuperAdmin)
```
POST /enrollments/add-child
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "school_id": "uuid",
  "child_first_name": "Jane",
  "child_last_name": "Doe",
  "child_birth_date": "2020-03-10",
  "gender": "female",
  "class_id": "uuid",
  "parent_id": "uuid"
}

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === request.school_id)
- Verify parent exists in users table with role='Parent'
- Verify classroom belongs to school
- Reject with 403 if insufficient permissions

Response (201):
{
  "child_id": "uuid",
  "enrollment_id": "uuid",
  "assigned_forms_count": 5,
  "message": "Additional child added successfully",
  "details": {
    "parent": {
      "id": "uuid",
      "first_name": "Jane",
      "last_name": "Doe",
      "email": "parent@example.com",
      "is_verified": true
    },
    "child": {
      "id": "uuid",
      "parent_id": "uuid",
      "school_id": "uuid",
      "first_name": "Jane",
      "last_name": "Doe",
      "birth_date": "2020-03-10",
      "gender": "female",
      "status": "active",
      "created_at": "2024-01-15T10:30:00Z"
    },
    "enrollment": {
      "id": "uuid",
      "child_id": "uuid",
      "school_id": "uuid",
      "classroom_id": "uuid",
      "status": "not_completed",
      "application_status": null,
      "created_at": "2024-01-15T10:30:00Z"
    },
    "assigned_forms": [
      {
        "id": "uuid",
        "form_template_id": "uuid",
        "form_name": "Student Registration Form",
        "assignment_source": "school_default",
        "status": "incomplete",
        "is_required": true
      },
      {
        "id": "uuid",
        "form_template_id": "uuid",
        "form_name": "Medical History Form",
        "assignment_source": "class_override",
        "status": "incomplete",
        "is_required": true
      }
    ]
  }
}

Error Responses:
- 400: Invalid request data or missing required fields
- 403: Insufficient permissions
- 404: Parent, school or classroom not found
- 422: Validation errors
```

**Database Operations (Transaction):**
```sql
-- Step 1: Verify parent exists and get parent details
SELECT id, first_name, last_name, email, is_verified
FROM users
WHERE id = $1 AND school_id = $2 AND role = 'Parent';

-- Step 2: Verify classroom belongs to school
SELECT id FROM classrooms
WHERE id = $3 AND school_id = $2 AND (is_active = true OR is_active IS NULL);

-- Step 3: Create child record (status defaults to 'active')
INSERT INTO children (id, parent_id, school_id, first_name, last_name, birth_date, gender, status)
VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, 'active')
RETURNING id, parent_id, school_id, first_name, last_name, birth_date, gender, status, created_at;

-- Step 4: Create enrollment record
INSERT INTO enrollments (id, child_id, school_id, classroom_id, status, application_status, progress, submitted_at)
VALUES (gen_random_uuid(), $1, $2, $3, 'not_completed', null, '{}', NOW())
RETURNING id, child_id, school_id, classroom_id, status, application_status, created_at;

-- Step 5: Get school default forms
SELECT id, form_name, fillout_form_id, fillout_form_url, is_required
FROM form_templates
WHERE school_id = $1 AND status = 'school_default' AND (is_active = true OR is_active IS NULL);

-- Step 6: Get class-specific form overrides
SELECT form_template_id, action, is_required
FROM class_form_overrides
WHERE school_id = $1 AND classroom_id = $2 AND (is_active = true OR is_active IS NULL);

-- Step 7: Consolidate forms and create student form assignments
-- (This logic consolidates school defaults + class overrides)
INSERT INTO student_form_assignments (
  id, school_id, enrollment_id, child_id, form_template_id,
  assignment_source, status, is_required, assigned_at
)
VALUES
  (gen_random_uuid(), $1, $2, $3, $4, 'school_default', 'incomplete', $5, NOW()),
  (gen_random_uuid(), $1, $2, $3, $6, 'class_override', 'incomplete', $7, NOW())
  -- ... (repeated for each consolidated form)
RETURNING id, form_template_id, assignment_source, status, is_required, assigned_at;
```

**Business Logic Flow:**
1. **Validate Input**: Check all required fields and verify parent exists
2. **Verify Parent**: Query users table to confirm parent exists with role='Parent'
3. **Verify Classroom**: Ensure classroom belongs to the school
4. **Create Child**: Insert into children table linked to existing parent
5. **Create Enrollment**: Insert into enrollments with status='not_completed'
6. **Get Default Forms**: Query form_templates for school_default forms
7. **Get Class Overrides**: Query class_form_overrides for specific classroom
8. **Consolidate Forms**: Merge default forms with class-specific overrides
9. **Create Form Assignments**: Insert into student_form_assignments with status='incomplete'
10. **Return Complete Response**: Include child, enrollment and assigned forms (no auth user creation or email sent)

**Key Differences from 8.1:**
- No auth user creation (parent already exists in Supabase auth)
- No user creation in users table (parent already exists)
- No invite_id generation
- No email notification sent
- Parent verification status can be either true or false
- Simpler transaction without user table INSERT
- Uses existing parent_id directly from request
- Same form assignment logic as 8.1

**Use Cases:**
- Existing parent wants to enroll additional child
- Sibling enrollment for families already in the system
- Secondary child enrollment without creating new parent account
- Administrative child addition to existing parent profiles

**Performance Considerations:**
- Lighter transaction than 8.1 (no user creation or email sending)
- Same form assignment complexity
- Uses existing parent verification status
- No external service calls (Supabase auth or email)

---

## 9. Reports and Analytics APIs

### 9.1 School Enrollment Summary Report (Protected - Admin/SuperAdmin)
```
GET /reports/enrollment-summary?school_id=uuid
Authorization: Bearer <jwt_token>
Content-Type: application/json

Authorization Logic:
- Extract user_id, role, school_id from JWT
- Allow if role === "SuperAdmin" OR (role === "Admin" AND jwt.school_id === query.school_id)
- Reject with 403 if school access denied

Response (200):
{
  "school_id": "uuid",
  "school_name": "The Goddard School - Downtown",
  "total_children": 150,
  "total_enrollments": 145,
  "active_enrollments": 140,
  "pending_enrollments": 5,
  "classrooms": [
    {
      "classroom_id": "uuid",
      "classroom_name": "Infant Room A",
      "capacity": 12,
      "enrolled_count": 10,
      "available_spots": 2
    },
    {
      "classroom_id": "uuid",
      "classroom_name": "Toddler Room B",
      "capacity": 15,
      "enrolled_count": 15,
      "available_spots": 0
    }
  ],
  "form_completion_stats": {
    "total_forms_assigned": 580,
    "completed_forms": 520,
    "pending_forms": 60,
    "completion_rate": "89.7%"
  },
  "generated_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Missing or invalid school_id parameter
- 403: Access denied to school
- 404: School not found
```

**Database Query (Complex Aggregation):**
```sql
-- Get comprehensive enrollment summary for a school
WITH enrollment_stats AS (
  SELECT
    e.status,
    COUNT(*) as count
  FROM enrollments e
  WHERE e.school_id = $1 AND (e.is_active = true OR e.is_active IS NULL)
  GROUP BY e.status
),
classroom_stats AS (
  SELECT
    c.id as classroom_id,
    c.name as classroom_name,
    c.capacity,
    COUNT(e.id) as enrolled_count,
    (c.capacity - COUNT(e.id)) as available_spots
  FROM classrooms c
  LEFT JOIN enrollments e ON c.id = e.classroom_id
    AND (e.is_active = true OR e.is_active IS NULL)
    AND e.status = 'active'
  WHERE c.school_id = $1 AND (c.is_active = true OR c.is_active IS NULL)
  GROUP BY c.id, c.name, c.capacity
),
form_stats AS (
  SELECT
    COUNT(*) as total_assigned,
    COUNT(CASE WHEN sfa.status = 'completed' THEN 1 END) as completed,
    COUNT(CASE WHEN sfa.status != 'completed' THEN 1 END) as pending
  FROM student_form_assignments sfa
  WHERE sfa.school_id = $1 AND (sfa.is_active = true OR sfa.is_active IS NULL)
)
SELECT
  s.id as school_id,
  s.name as school_name,
  (SELECT COUNT(*) FROM children WHERE school_id = $1 AND status = 'active') as total_children,
  (SELECT COUNT(*) FROM enrollments WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)) as total_enrollments,
  COALESCE((SELECT count FROM enrollment_stats WHERE status = 'active'), 0) as active_enrollments,
  COALESCE((SELECT count FROM enrollment_stats WHERE status = 'pending'), 0) as pending_enrollments,
  json_agg(
    json_build_object(
      'classroom_id', cs.classroom_id,
      'classroom_name', cs.classroom_name,
      'capacity', cs.capacity,
      'enrolled_count', cs.enrolled_count,
      'available_spots', cs.available_spots
    )
  ) as classrooms,
  (
    SELECT json_build_object(
      'total_forms_assigned', fs.total_assigned,
      'completed_forms', fs.completed,
      'pending_forms', fs.pending,
      'completion_rate', ROUND((fs.completed::numeric / NULLIF(fs.total_assigned, 0) * 100), 1) || '%'
    )
    FROM form_stats fs
  ) as form_completion_stats
FROM schools s
CROSS JOIN classroom_stats cs
WHERE s.id = $1
GROUP BY s.id, s.name;
```

**Business Logic Flow:**
1. **Validate Request**: Check school_id is provided and valid UUID
2. **Authorization Check**: Verify admin has access to this school's data
3. **Aggregate Data**: Collect enrollment, classroom, and form completion statistics
4. **Calculate Metrics**: Compute completion rates and availability
5. **Format Response**: Build comprehensive report with all statistics
6. **Return Report**: Complete enrollment summary with analytics

**Use Cases:**
- Admin dashboard overview
- Enrollment capacity planning
- Form completion tracking
- School performance monitoring
- Regulatory compliance reporting

---

## 10. Missing APIs - Implementation Required

### Parent Portal APIs

#### 10.1 Get User Context (Enhancement)
```
GET /users/me
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
{
  "user_id": "uuid",
  "email": "parent@example.com",
  "role": "Parent",
  "parent_id": "uuid",
  "school_id": "uuid",
  "first_name": "John",
  "last_name": "Doe"
}

Error Responses:
- 401: Unauthorized
```

**Database Query:**
```sql
SELECT u.id as user_id, u.email, u.role,
       p.id as parent_id, p.school_id,
       p.first_name, p.last_name
FROM users u
LEFT JOIN parents p ON u.id = p.user_id
WHERE u.id = $1;
```

#### 10.2 Get Parent's Children
```
GET /parents/{parent_id}/children
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
[
  {
    "child_id": "uuid",
    "first_name": "Emma",
    "last_name": "Doe",
    "dob": "2020-05-15",
    "age": 4,
    "class_name": "Preschool A",
    "enrollment_id": "uuid"
  }
]

Error Responses:
- 401: Unauthorized
- 403: Access denied
- 404: Parent not found
```

**Database Query:**
```sql
SELECT c.id as child_id, c.first_name, c.last_name, c.dob,
       DATE_PART('year', AGE(c.dob)) as age,
       cl.name as class_name, e.id as enrollment_id
FROM children c
JOIN enrollments e ON c.id = e.child_id
JOIN classrooms cl ON e.classroom_id = cl.id
WHERE c.parent_id = $1 AND c.is_active = true
ORDER BY c.first_name;
```

#### 10.3 Get Child Profile
```
GET /parents/{parent_id}/children/{child_id}/profile
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
{
  "child_id": "uuid",
  "first_name": "Emma",
  "last_name": "Doe",
  "dob": "2020-05-15",
  "age": 4,
  "class_name": "Preschool A",
  "enrollment_id": "uuid",
  "enrollment_progress": {
    "total_forms": 8,
    "completed_forms": 6,
    "completion_percentage": 75
  }
}

Error Responses:
- 401: Unauthorized
- 403: Access denied
- 404: Child not found
```

**Database Query:**
```sql
WITH form_stats AS (
  SELECT COUNT(*) as total_forms,
         COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_forms
  FROM student_form_assignments
  WHERE child_id = $2
)
SELECT c.id as child_id, c.first_name, c.last_name, c.dob,
       DATE_PART('year', AGE(c.dob)) as age,
       cl.name as class_name, e.id as enrollment_id,
       fs.total_forms, fs.completed_forms,
       ROUND((fs.completed_forms::numeric / NULLIF(fs.total_forms, 0)) * 100) as completion_percentage
FROM children c
JOIN enrollments e ON c.id = e.child_id
JOIN classrooms cl ON e.classroom_id = cl.id
CROSS JOIN form_stats fs
WHERE c.id = $2 AND c.parent_id = $1;
```

#### 10.4 Get Child's Assigned Forms
```
GET /parents/{parent_id}/children/{child_id}/forms
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
[
  {
    "assignment_id": "uuid",
    "form_template_id": "uuid",
    "title": "Emergency Contact Form",
    "status": "pending",
    "due_date": "2024-02-01",
    "last_updated": "2024-01-15T10:30:00Z",
    "launch_url": "https://forms.fillout.com/t/emergency-contact"
  }
]

Error Responses:
- 401: Unauthorized
- 403: Access denied
- 404: Child not found
```

**Database Query:**
```sql
SELECT sfa.id as assignment_id, sfa.form_template_id,
       ft.title, sfa.status, sfa.due_date,
       sfa.updated_at as last_updated, ft.form_url as launch_url
FROM student_form_assignments sfa
JOIN form_templates ft ON sfa.form_template_id = ft.id
WHERE sfa.child_id = $2
  AND EXISTS (SELECT 1 FROM children WHERE id = $2 AND parent_id = $1)
ORDER BY sfa.due_date ASC;
```

### Admin Portal APIs

#### 10.5 Get Classroom Details
```
GET /classrooms/{id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
{
  "id": "uuid",
  "name": "Preschool A",
  "capacity": 20,
  "age_group": "3-4 years",
  "teachers": ["Ms. Smith", "Ms. Johnson"],
  "notes": "Morning session classroom",
  "enrolled_count": 18,
  "available_spots": 2
}

Error Responses:
- 401: Unauthorized
- 403: Access denied (Admin/SuperAdmin only)
- 404: Classroom not found
```

**Database Query:**
```sql
SELECT c.id, c.name, c.capacity, c.age_group,
       c.teachers, c.notes,
       COUNT(e.id) as enrolled_count,
       (c.capacity - COUNT(e.id)) as available_spots
FROM classrooms c
LEFT JOIN enrollments e ON c.id = e.classroom_id AND e.status = 'active'
WHERE c.id = $1
GROUP BY c.id;
```

#### 10.6 Get Classroom Forms
```
GET /classrooms/{id}/forms
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
[
  {
    "form_template_id": "uuid",
    "title": "Health Assessment Form",
    "status": "active",
    "due_date": "2024-02-15",
    "assigned_by": "admin@school.com",
    "assigned_at": "2024-01-10T08:00:00Z"
  }
]

Error Responses:
- 401: Unauthorized
- 403: Access denied (Admin/SuperAdmin only)
- 404: Classroom not found
```

**Database Query:**
```sql
SELECT cf.form_template_id, ft.title, cf.status,
       cf.due_date, cf.assigned_by, cf.created_at as assigned_at
FROM classroom_forms cf
JOIN form_templates ft ON cf.form_template_id = ft.id
WHERE cf.classroom_id = $1 AND cf.is_active = true
ORDER BY cf.due_date ASC;
```

#### 10.7 Assign Form to Classroom
```
POST /classrooms/{id}/forms
Authorization: Bearer <jwt_token>
Content-Type: application/json

Request Body:
{
  "form_template_id": "uuid",
  "due_date": "2024-02-15",
  "notes": "Required for all students"
}

Response (201):
{
  "id": "uuid",
  "classroom_id": "uuid",
  "form_template_id": "uuid",
  "status": "active",
  "due_date": "2024-02-15",
  "assigned_by": "admin@school.com",
  "created_at": "2024-01-15T10:30:00Z"
}

Error Responses:
- 400: Invalid request
- 401: Unauthorized
- 403: Access denied (Admin/SuperAdmin only)
- 404: Classroom or form template not found
```

#### 10.8 Remove Form from Classroom
```
DELETE /classrooms/{id}/forms/{form_id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
{
  "message": "Form assignment removed successfully"
}

Error Responses:
- 401: Unauthorized
- 403: Access denied (Admin/SuperAdmin only)
- 404: Assignment not found
```

#### 10.9 Get Parent Profile
```
GET /parents/{parent_id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
{
  "id": "uuid",
  "first_name": "John",
  "last_name": "Doe",
  "email": "john.doe@example.com",
  "phone_numbers": {
    "primary": "555-0100",
    "secondary": "555-0101"
  },
  "mailing_address": {
    "street": "123 Main St",
    "city": "Springfield",
    "state": "IL",
    "zip": "62701"
  },
  "linked_children": [
    {
      "child_id": "uuid",
      "first_name": "Emma",
      "last_name": "Doe",
      "classroom": "Preschool A"
    }
  ]
}

Error Responses:
- 401: Unauthorized
- 403: Access denied (Admin/SuperAdmin only)
- 404: Parent not found
```

**Database Query:**
```sql
SELECT p.id, p.first_name, p.last_name, p.email,
       p.primary_phone as primary_phone,
       p.secondary_phone as secondary_phone,
       p.address_street, p.address_city, p.address_state, p.address_zip,
       json_agg(
         json_build_object(
           'child_id', c.id,
           'first_name', c.first_name,
           'last_name', c.last_name,
           'classroom', cl.name
         )
       ) as linked_children
FROM parents p
LEFT JOIN children c ON p.id = c.parent_id
LEFT JOIN enrollments e ON c.id = e.child_id
LEFT JOIN classrooms cl ON e.classroom_id = cl.id
WHERE p.id = $1
GROUP BY p.id;
```

#### 10.10 Get Child Demographics
```
GET /children/{child_id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

Response (200):
{
  "id": "uuid",
  "first_name": "Emma",
  "last_name": "Doe",
  "dob": "2020-05-15",
  "age": 4,
  "classroom_id": "uuid",
  "classroom_name": "Preschool A",
  "guardian_references": [
    {
      "parent_id": "uuid",
      "relationship": "Mother",
      "name": "Jane Doe"
    }
  ]
}

Error Responses:
- 401: Unauthorized
- 403: Access denied (Admin/SuperAdmin only)
- 404: Child not found
```

**Database Query:**
```sql
SELECT c.id, c.first_name, c.last_name, c.dob,
       DATE_PART('year', AGE(c.dob)) as age,
       e.classroom_id, cl.name as classroom_name,
       json_agg(
         json_build_object(
           'parent_id', p.id,
           'relationship', pc.relationship,
           'name', CONCAT(p.first_name, ' ', p.last_name)
         )
       ) as guardian_references
FROM children c
LEFT JOIN enrollments e ON c.id = e.child_id
LEFT JOIN classrooms cl ON e.classroom_id = cl.id
LEFT JOIN parent_child pc ON c.id = pc.child_id
LEFT JOIN parents p ON pc.parent_id = p.id
WHERE c.id = $1
GROUP BY c.id, e.classroom_id, cl.name;
```

---
