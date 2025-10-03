use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct GetAdminDashboardMetricsRequest {
    pub school_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct AdminDashboardMetricsResponse {
    pub school_id: Uuid,
    pub total_classrooms: i64,
    pub total_active_parents: i64,
    pub total_active_children: i64,
    pub total_forms: i64,
    pub classwise_metrics: Vec<ClasswiseMetric>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClasswiseMetric {
    pub classroom_id: Uuid,
    pub classroom_name: String,
    pub total_enrollments: i64,
    pub completed_enrollments: i64,
}
