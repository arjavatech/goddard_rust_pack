use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{NaiveDate, NaiveDateTime};

// ─── DB row struct ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub id: Uuid,
    pub school_id: Uuid,
    pub requester_id: Option<Uuid>,
    pub requester_name: String,
    pub requester_role: Option<String>,
    pub item: String,
    pub quantity: i32,
    pub category: Option<String>,
    pub scope: Option<String>,
    pub classroom_id: Option<Uuid>,
    pub classroom_name: Option<String>,
    pub teacher_id: Option<Uuid>,
    pub teacher_name: Option<String>,
    pub product_link: Option<String>,
    pub product_image: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub source: String,
    pub amount_spent: Option<f64>,
    pub payment_method: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub payment_notes: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

// ─── API request bodies ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequestBody {
    pub school_id: Uuid,
    pub requester_id: Uuid,
    pub requester_name: String,
    pub requester_role: String,
    pub item: String,
    pub quantity: i32,
    pub category: Option<String>,
    pub scope: String,
    pub classroom_id: Option<Uuid>,
    pub classroom_name: Option<String>,
    pub teacher_id: Option<Uuid>,
    pub teacher_name: Option<String>,
    pub product_link: Option<String>,
    pub product_image: Option<String>,
    pub notes: Option<String>,
    // Image upload fields — frontend sends file as base64; backend uploads to S3
    pub image_base64: Option<String>,
    pub image_name: Option<String>,
    pub image_content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequestStatusBody {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayRequestBody {
    pub amount_spent: f64,
    pub payment_method: String,
    pub purchase_date: NaiveDate,
    pub payment_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExpenseBody {
    pub school_id: Uuid,
    pub requester_name: String,
    pub requester_role: Option<String>,
    pub item: String,
    pub quantity: Option<i32>,
    pub category: Option<String>,
    pub scope: Option<String>,
    pub classroom_name: Option<String>,
    pub teacher_name: Option<String>,
    pub amount_spent: f64,
    pub payment_method: String,
    pub purchase_date: NaiveDate,
    pub payment_notes: Option<String>,
}

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRequestsParams {
    pub school_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub status: Option<String>,
    pub requester_role: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExpensesParams {
    pub school_id: Option<Uuid>,
    pub include: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

// ─── Response shapes ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RequestStatusCounts {
    pub pending: i64,
    pub in_progress: i64,
    pub completed: i64,
}

#[derive(Debug, Serialize)]
pub struct RequestsListResponse {
    pub data: Vec<Request>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub counts: RequestStatusCounts,
}

#[derive(Debug, Serialize)]
pub struct CategoryBreakdown {
    pub name: String,
    pub total: f64,
}

#[derive(Debug, Serialize)]
pub struct ScopeBreakdown {
    pub classroom: f64,
    pub teacher: f64,
    pub school: f64,
}

#[derive(Debug, Serialize)]
pub struct ExpenseSummary {
    pub total_spent: f64,
    pub by_scope: ScopeBreakdown,
    pub by_category: Vec<CategoryBreakdown>,
    pub by_classroom: Vec<CategoryBreakdown>,
    pub by_teacher: Vec<CategoryBreakdown>,
}

#[derive(Debug, Serialize)]
pub struct ExpensesListResponse {
    pub data: Vec<Request>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ExpenseSummary>,
}
