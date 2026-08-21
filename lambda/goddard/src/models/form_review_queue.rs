use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FormReviewQueueQuery {
    pub school_id: Uuid,
    pub classroom_id: Option<Uuid>,
    pub form_template_id: Option<Uuid>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct StudentFormReviewQueueItem {
    pub assignment_id: Uuid,
    pub school_id: Uuid,
    pub enrollment_id: Uuid,
    pub child_id: Uuid,
    pub form_template_id: Uuid,
    pub form_name: String,
    pub fillout_form_id: Option<String>,
    pub status: String,
    pub submitted_at: NaiveDateTime,
    pub recent_edit_link: Option<String>,
    pub recent_pdf_link: Option<String>,
    pub student_first_name: String,
    pub student_last_name: String,
    pub parent_first_name: String,
    pub parent_last_name: String,
    pub parent_email: String,
    pub classroom_id: Option<Uuid>,
    pub classroom_name: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EmployeeFormReviewQueueItem {
    pub assignment_id: Uuid,
    pub school_id: Uuid,
    pub employee_id: Uuid,
    pub form_template_id: Uuid,
    pub form_name: String,
    pub fillout_form_id: Option<String>,
    pub status: String,
    pub submitted_at: NaiveDateTime,
    pub recent_edit_link: Option<String>,
    pub recent_pdf_link: Option<String>,
    pub employee_first_name: String,
    pub employee_last_name: String,
    pub employee_email: String,
}
