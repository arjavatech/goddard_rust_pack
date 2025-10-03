use uuid::Uuid;

use crate::dao::admin_dao::AdminDao;
use crate::models::admin::{GetAdminDashboardMetricsRequest, AdminDashboardMetricsResponse};
use crate::error::AppError;

type ApiResult<T> = Result<T, AppError>;

pub struct AdminService {
    admin_dao: AdminDao,
}

impl AdminService {
    pub fn new(admin_dao: AdminDao) -> Self {
        Self { admin_dao }
    }

    pub async fn get_dashboard_metrics(&self, request: GetAdminDashboardMetricsRequest) -> ApiResult<AdminDashboardMetricsResponse> {
        println!("[AdminService] Fetching dashboard metrics for school_id: {}", request.school_id);

        let metrics = self.admin_dao.get_dashboard_metrics(request.school_id).await?;

        println!("[AdminService] Successfully retrieved dashboard metrics");
        Ok(metrics)
    }
}
