| Endpoint | Purpose | Expected Payload | Notes |
| --- | --- | --- | --- |
| `GET /users/me` (enhancement) | Include guardian context needed to scope downstream calls | Add `parent_id` and `school_id` when the authenticated user is a parent | Presently returns only `role` and optionally `school_id`; both fields are required and should always be present in mock mode. |
| `GET /parents/{parent_id}/children` | Return the list of children linked to the logged-in guardian | Array of children with `child_id`, `first_name`, `last_name`, `dob`, `age`, `class_name`, `enrollment_id` | Avoid fetching every child for the school; data feeds the child selector and overview cards. |
| `GET /parents/{parent_id}/children/{child_id}/profile` | Provide detailed child metadata for dashboard cards | Object containing `dob`, `age`, `class_name`, `enrollment_id`, `enrollment_progress` | Needed to populate DOB/age rows and normalized progress metrics. |
| `GET /parents/{parent_id}/children/{child_id}/forms` | Surface assigned forms with richer metadata | Array with `assignment_id`, `form_template_id`, `title`, `status`, `due_date`, `last_updated`, `launch_url` | Supplies the per-child "Forms & Documents" grid and progress calculations. |

## Admin Portal (`/admin/*`)

| Endpoint | Purpose | Expected Payload | Notes |
| --- | --- | --- | --- |
| `GET /classrooms/{id}` | Fetch detailed classroom profile | Object with `name`, `capacity`, `age_group`, `teachers`, `notes` | Classroom detail page falls back to IDs without this endpoint. |
| `GET /classrooms/{id}/forms` | List forms assigned to a classroom | Array with `form_template_id`, `title`, `status`, `due_date`, `assigned_by` | Required for classroom form review and to replace placeholder notices. |
| `POST /classrooms/{id}/forms` & `DELETE /classrooms/{id}/forms/{form_id}` | Manage classroom-level form overrides | Accept assignments/removals with audit fields | Enables actual form assignment actions instead of disabled UI controls. |
| `GET /parents/{parent_id}` | Fetch a single parent profile | Object with names, phone numbers, mailing address, linked children | Parent detail page currently re-fetches the entire list and still lacks core info. |
| `GET /children/{child_id}` | Retrieve child demographics | Object with `first_name`, `last_name`, `dob`, `classroom_id`, guardian references | Student management relies on this to avoid inferring from enrollment snapshots. |