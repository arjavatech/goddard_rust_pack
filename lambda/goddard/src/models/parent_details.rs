use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ParentDetailsResponse {
    pub parent_id: Uuid,
    pub parent_email: String,
    pub parent_first_name: String,
    pub parent_last_name: String,
    pub signed_status: String,
    pub children: Vec<ParentChild>,
}

#[derive(Debug, Serialize)]
pub struct ParentChild {
    pub child_id: Uuid,
    pub child_full_name: String,
    pub child_dob: Option<NaiveDate>,
    pub child_status: Option<String>,
    pub enrollment_id: Uuid,
    pub classroom_id: Uuid,
    pub classroom_name: String,
    pub forms: Vec<ParentChildForm>,
}

#[derive(Debug, Serialize)]
pub struct ParentChildForm {
    pub form_id: String, // Format: "form_{uuid}"
    pub student_form_assignment_id: Uuid,
    pub fillout_form_id: Option<String>,
    pub form_name: String,
    pub status: String,
    pub is_required: bool,
    pub recent_edit_link: Option<String>,
    pub recent_pdf_link: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_on: Option<chrono::NaiveDateTime>,
}

// Database row structure for query results
#[derive(Debug)]
pub struct ParentDetailsRow {
    pub parent_id: Uuid,
    pub parent_email: String,
    pub parent_first_name: String,
    pub parent_last_name: String,
    pub signed_status: String,
    pub child_id: Option<Uuid>,
    pub child_first_name: Option<String>,
    pub child_last_name: Option<String>,
    pub child_dob: Option<NaiveDate>,
    pub child_status: Option<String>,
    pub enrollment_id: Option<Uuid>,
    pub classroom_id: Option<Uuid>,
    pub classroom_name: Option<String>,
    pub student_form_assignment_id: Option<Uuid>,
    pub form_template_id: Option<Uuid>,
    pub form_name: Option<String>,
    pub fillout_form_id: Option<String>,
    pub status: Option<String>,
    pub is_required: Option<bool>,
    pub recent_edit_link: Option<String>,
    pub recent_pdf_link: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_on: Option<chrono::NaiveDateTime>,
}