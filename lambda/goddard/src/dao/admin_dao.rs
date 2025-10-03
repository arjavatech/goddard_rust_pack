use deadpool_postgres::Pool;
use uuid::Uuid;
use std::time::Duration;

use crate::models::admin::{AdminDashboardMetricsResponse, ClasswiseMetric};
use crate::error::AppError;

type ApiResult<T> = Result<T, AppError>;

#[derive(Clone)]
pub struct AdminDao {
    pool: Pool,
}

impl AdminDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn get_dashboard_metrics(&self, school_id: Uuid) -> ApiResult<AdminDashboardMetricsResponse> {
        use tokio::time::timeout;

        println!("[AdminDao] Fetching dashboard metrics for school_id: {}", school_id);

        let client_result = timeout(Duration::from_secs(5), self.pool.get()).await;
        let client = match client_result {
            Ok(client) => client.map_err(|e| AppError::Database(format!("Failed to get database connection: {}", e)))?,
            Err(_) => return Err(AppError::Database("Timeout getting database connection for dashboard metrics".to_string()))
        };

        // Single optimized query using CTEs to fetch all metrics
        let query = "
            WITH classroom_count AS (
                SELECT COUNT(*) as total
                FROM classrooms
                WHERE school_id = $1 AND is_active = true
            ),
            parent_count AS (
                SELECT COUNT(DISTINCT c.parent_id) as total
                FROM children c
                INNER JOIN enrollments e ON c.id = e.child_id
                WHERE c.school_id = $1 AND e.is_active = true
            ),
            children_count AS (
                SELECT COUNT(DISTINCT e.child_id) as total
                FROM enrollments e
                WHERE e.school_id = $1 AND e.is_active = true
            ),
            forms_count AS (
                SELECT COUNT(*) as total
                FROM form_templates
                WHERE school_id = $1 AND (status = 'school_default' OR status = 'active') AND is_active = true
            ),
            classwise_data AS (
                SELECT
                    cl.id as classroom_id,
                    cl.name as classroom_name,
                    COUNT(e.id) as total_enrollments,
                    COUNT(CASE WHEN e.status = 'completed' THEN 1 END) as completed_enrollments
                FROM classrooms cl
                LEFT JOIN enrollments e ON cl.id = e.classroom_id AND e.school_id = cl.school_id AND e.is_active = true
                WHERE cl.school_id = $1 AND cl.is_active = true
                GROUP BY cl.id, cl.name
                ORDER BY cl.name
            )
            SELECT
                (SELECT total FROM classroom_count) as total_classrooms,
                (SELECT total FROM parent_count) as total_active_parents,
                (SELECT total FROM children_count) as total_active_children,
                (SELECT total FROM forms_count) as total_forms,
                COALESCE(
                    json_agg(
                        json_build_object(
                            'classroom_id', cd.classroom_id,
                            'classroom_name', cd.classroom_name,
                            'total_enrollments', cd.total_enrollments,
                            'completed_enrollments', cd.completed_enrollments
                        )
                        ORDER BY cd.classroom_name
                    ),
                    '[]'::json
                ) as classwise_metrics
            FROM classwise_data cd
        ";

        let query_result = timeout(Duration::from_secs(10),
            client.query_one(query, &[&school_id])
        ).await;

        let row = match query_result {
            Ok(result) => result.map_err(|e| AppError::Database(format!("Failed to fetch dashboard metrics: {}", e)))?,
            Err(_) => return Err(AppError::Database("Timeout during dashboard metrics query".to_string()))
        };

        // Parse classwise metrics from JSON
        let classwise_json: serde_json::Value = row.get("classwise_metrics");
        let classwise_metrics: Vec<ClasswiseMetric> = serde_json::from_value(classwise_json)
            .unwrap_or_else(|_| vec![]);

        let response = AdminDashboardMetricsResponse {
            school_id,
            total_classrooms: row.get::<_, Option<i64>>("total_classrooms").unwrap_or(0),
            total_active_parents: row.get::<_, Option<i64>>("total_active_parents").unwrap_or(0),
            total_active_children: row.get::<_, Option<i64>>("total_active_children").unwrap_or(0),
            total_forms: row.get::<_, Option<i64>>("total_forms").unwrap_or(0),
            classwise_metrics,
        };

        println!("[AdminDao] Successfully fetched dashboard metrics: {} classrooms, {} parents, {} children, {} forms",
            response.total_classrooms, response.total_active_parents, response.total_active_children, response.total_forms);

        Ok(response)
    }
}
