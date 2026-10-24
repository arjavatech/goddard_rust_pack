use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AdminNavCounts {
    pub student_forms_due: i64,
    pub student_forms_review: i64,
    pub employee_forms_due: i64,
    pub employee_forms_review: i64,
    pub student_docs_due: i64,
    pub student_docs_review: i64,
    pub employee_docs_due: i64,
    pub employee_docs_review: i64,
}

#[derive(Debug, Serialize)]
pub struct MyNavCounts {
    pub forms_pending: i64,
    pub docs_pending: i64,
}
