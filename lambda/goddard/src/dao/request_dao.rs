use deadpool_postgres::Pool;
use uuid::Uuid;
use chrono::NaiveDate;
use crate::models::requests::{
    Request, CreateRequestBody, CreateExpenseBody,
    ExpenseSummary, CategoryBreakdown, ScopeBreakdown,
};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct RequestDao {
    pool: Pool,
}

impl RequestDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_request(&self, row: &tokio_postgres::Row) -> Request {
        Request {
            id: row.get("id"),
            school_id: row.get("school_id"),
            requester_id: row.get("requester_id"),
            requester_name: row.get("requester_name"),
            requester_role: row.get("requester_role"),
            item: row.get("item"),
            quantity: row.get("quantity"),
            category: row.get("category"),
            scope: row.get("scope"),
            classroom_id: row.get("classroom_id"),
            classroom_name: row.get("classroom_name"),
            teacher_id: row.get("teacher_id"),
            teacher_name: row.get("teacher_name"),
            product_link: row.get("product_link"),
            product_image: row.get("product_image"),
            notes: row.get("notes"),
            status: row.get("status"),
            source: row.get("source"),
            amount_spent: row.get("amount_spent"),
            payment_method: row.get("payment_method"),
            purchase_date: row.get("purchase_date"),
            payment_notes: row.get("payment_notes"),
            created_at: row.get("created_at"),
        }
    }

    // ── Requests ──────────────────────────────────────────────────────────────

    pub async fn list_requests(
        &self,
        school_id: Option<Uuid>,
        requester_id: Option<Uuid>,
        status: Option<&str>,
        requester_role: Option<&str>,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<Request>, i64, i64, i64, i64), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let offset = (page - 1) * limit;

        let rows = client.query(
            "SELECT id, school_id, requester_id, requester_name, requester_role,
                    item, quantity, category, scope, classroom_id, classroom_name,
                    teacher_id, teacher_name, product_link, product_image, notes,
                    status, source, amount_spent, payment_method, purchase_date, payment_notes, created_at,
                    COUNT(*) OVER() AS total_count,
                    COUNT(*) FILTER (WHERE status = 'Pending') OVER() AS pending_count,
                    COUNT(*) FILTER (WHERE status = 'In Progress') OVER() AS in_progress_count,
                    COUNT(*) FILTER (WHERE status = 'Completed') OVER() AS completed_count
             FROM requests
             WHERE source = 'request'
               AND ($1::uuid IS NULL OR school_id = $1)
               AND ($2::uuid IS NULL OR requester_id = $2)
               AND ($3::text IS NULL OR status = $3)
               AND ($4::text IS NULL OR requester_role = $4)
             ORDER BY created_at DESC
             LIMIT $5 OFFSET $6",
            &[&school_id, &requester_id, &status, &requester_role, &limit, &offset],
        ).await.map_err(|e| AppError::Database(format!("Failed to list requests: {}", e)))?;

        let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
        let pending: i64 = rows.first().map(|r| r.get("pending_count")).unwrap_or(0);
        let in_progress: i64 = rows.first().map(|r| r.get("in_progress_count")).unwrap_or(0);
        let completed: i64 = rows.first().map(|r| r.get("completed_count")).unwrap_or(0);

        let data = rows.iter().map(|r| self.row_to_request(r)).collect();
        Ok((data, total, pending, in_progress, completed))
    }

    pub async fn create_request(&self, body: &CreateRequestBody) -> Result<Request, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "INSERT INTO requests (
                id, school_id, requester_id, requester_name, requester_role,
                item, quantity, category, scope, classroom_id, classroom_name,
                teacher_id, teacher_name, product_link, product_image, notes,
                status, source, created_at
             ) VALUES (
                gen_random_uuid(), $1, $2, $3, $4,
                $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15,
                'Pending', 'request', NOW()
             ) RETURNING id, school_id, requester_id, requester_name, requester_role,
                         item, quantity, category, scope, classroom_id, classroom_name,
                         teacher_id, teacher_name, product_link, product_image, notes,
                         status, source, amount_spent, payment_method, purchase_date, payment_notes, created_at",
            &[
                &body.school_id, &body.requester_id, &body.requester_name, &body.requester_role,
                &body.item, &body.quantity, &body.category, &body.scope,
                &body.classroom_id, &body.classroom_name,
                &body.teacher_id, &body.teacher_name,
                &body.product_link, &body.product_image, &body.notes,
            ],
        ).await.map_err(|e| AppError::Database(format!("Failed to create request: {}", e)))?;

        Ok(self.row_to_request(&row))
    }

    pub async fn get_request_by_id(&self, id: Uuid) -> Result<Option<Request>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT id, school_id, requester_id, requester_name, requester_role,
                    item, quantity, category, scope, classroom_id, classroom_name,
                    teacher_id, teacher_name, product_link, product_image, notes,
                    status, source, amount_spent, payment_method, purchase_date, payment_notes, created_at
             FROM requests WHERE id = $1",
            &[&id],
        ).await.map_err(|e| AppError::Database(format!("Failed to get request: {}", e)))?;

        Ok(row.map(|r| self.row_to_request(&r)))
    }

    pub async fn update_request_status(&self, id: Uuid, status: &str) -> Result<Request, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "UPDATE requests SET status = $2
             WHERE id = $1
             RETURNING id, school_id, requester_id, requester_name, requester_role,
                       item, quantity, category, scope, classroom_id, classroom_name,
                       teacher_id, teacher_name, product_link, product_image, notes,
                       status, source, amount_spent, payment_method, purchase_date, payment_notes, created_at",
            &[&id, &status],
        ).await.map_err(|e| AppError::Database(format!("Failed to update request status: {}", e)))?;

        Ok(self.row_to_request(&row))
    }

    pub async fn pay_request(
        &self,
        id: Uuid,
        amount_spent: f64,
        payment_method: &str,
        purchase_date: NaiveDate,
        payment_notes: Option<&str>,
    ) -> Result<Request, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "UPDATE requests
             SET status = 'Completed', amount_spent = $2, payment_method = $3,
                 purchase_date = $4, payment_notes = $5
             WHERE id = $1
             RETURNING id, school_id, requester_id, requester_name, requester_role,
                       item, quantity, category, scope, classroom_id, classroom_name,
                       teacher_id, teacher_name, product_link, product_image, notes,
                       status, source, amount_spent, payment_method, purchase_date, payment_notes, created_at",
            &[&id, &amount_spent, &payment_method, &purchase_date, &payment_notes],
        ).await.map_err(|e| AppError::Database(format!("Failed to pay request: {}", e)))?;

        Ok(self.row_to_request(&row))
    }

    pub async fn delete_request(&self, id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let n = client.execute(
            "DELETE FROM requests WHERE id = $1",
            &[&id],
        ).await.map_err(|e| AppError::Database(format!("Failed to delete request: {}", e)))?;

        if n == 0 {
            return Err(AppError::NotFound("Request not found".to_string()));
        }
        Ok(())
    }

    // ── Expenses (completed entries from both request workflow and manual) ─────

    pub async fn list_expenses(
        &self,
        school_id: Option<Uuid>,
        search: Option<&str>,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<Request>, i64), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let offset = (page - 1) * limit;
        let search_pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows = client.query(
            "SELECT id, school_id, requester_id, requester_name, requester_role,
                    item, quantity, category, scope, classroom_id, classroom_name,
                    teacher_id, teacher_name, product_link, product_image, notes,
                    status, source, amount_spent, payment_method, purchase_date, payment_notes, created_at,
                    COUNT(*) OVER() AS total_count
             FROM requests
             WHERE status = 'Completed'
               AND ($1::uuid IS NULL OR school_id = $1)
               AND ($2::text IS NULL OR (
                   LOWER(item) LIKE $2
                   OR LOWER(requester_name) LIKE $2
                   OR LOWER(COALESCE(category, '')) LIKE $2
               ))
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4",
            &[&school_id, &search_pattern, &limit, &offset],
        ).await.map_err(|e| AppError::Database(format!("Failed to list expenses: {}", e)))?;

        let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
        let data = rows.iter().map(|r| self.row_to_request(r)).collect();
        Ok((data, total))
    }

    pub async fn get_expense_summary(&self, school_id: Option<Uuid>) -> Result<ExpenseSummary, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let total_row = client.query_one(
            "SELECT
                COALESCE(SUM(amount_spent), 0.0)::float8 AS total_spent,
                COALESCE(SUM(CASE WHEN scope = 'classroom' THEN amount_spent ELSE 0 END), 0.0)::float8 AS classroom_total,
                COALESCE(SUM(CASE WHEN scope = 'teacher'   THEN amount_spent ELSE 0 END), 0.0)::float8 AS teacher_total,
                COALESCE(SUM(CASE WHEN scope = 'school'    THEN amount_spent ELSE 0 END), 0.0)::float8 AS school_total
             FROM requests
             WHERE status = 'Completed'
               AND ($1::uuid IS NULL OR school_id = $1)",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to get expense totals: {}", e)))?;

        let by_scope = ScopeBreakdown {
            classroom: total_row.get("classroom_total"),
            teacher: total_row.get("teacher_total"),
            school: total_row.get("school_total"),
        };

        let category_rows = client.query(
            "SELECT COALESCE(category, 'Uncategorized') AS name, SUM(amount_spent)::float8 AS total
             FROM requests
             WHERE status = 'Completed' AND ($1::uuid IS NULL OR school_id = $1)
             GROUP BY category ORDER BY total DESC LIMIT 10",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to get category breakdown: {}", e)))?;

        let classroom_rows = client.query(
            "SELECT COALESCE(classroom_name, 'Unknown') AS name, SUM(amount_spent)::float8 AS total
             FROM requests
             WHERE status = 'Completed' AND scope = 'classroom' AND ($1::uuid IS NULL OR school_id = $1)
             GROUP BY classroom_name ORDER BY total DESC LIMIT 10",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to get classroom breakdown: {}", e)))?;

        let teacher_rows = client.query(
            "SELECT COALESCE(teacher_name, 'Unknown') AS name, SUM(amount_spent)::float8 AS total
             FROM requests
             WHERE status = 'Completed' AND scope = 'teacher' AND ($1::uuid IS NULL OR school_id = $1)
             GROUP BY teacher_name ORDER BY total DESC LIMIT 10",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to get teacher breakdown: {}", e)))?;

        Ok(ExpenseSummary {
            total_spent: total_row.get("total_spent"),
            by_scope,
            by_category: category_rows.iter().map(|r| CategoryBreakdown { name: r.get("name"), total: r.get("total") }).collect(),
            by_classroom: classroom_rows.iter().map(|r| CategoryBreakdown { name: r.get("name"), total: r.get("total") }).collect(),
            by_teacher: teacher_rows.iter().map(|r| CategoryBreakdown { name: r.get("name"), total: r.get("total") }).collect(),
        })
    }

    pub async fn create_manual_expense(&self, body: &CreateExpenseBody) -> Result<Request, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "INSERT INTO requests (
                id, school_id, requester_name, requester_role,
                item, quantity, category, scope, classroom_name, teacher_name,
                status, source, amount_spent, payment_method, purchase_date, payment_notes, created_at
             ) VALUES (
                gen_random_uuid(), $1, $2, $3,
                $4, $5, $6, $7, $8, $9,
                'Completed', 'manual', $10, $11, $12, $13, NOW()
             ) RETURNING id, school_id, requester_id, requester_name, requester_role,
                         item, quantity, category, scope, classroom_id, classroom_name,
                         teacher_id, teacher_name, product_link, product_image, notes,
                         status, source, amount_spent, payment_method, purchase_date, payment_notes, created_at",
            &[
                &body.school_id, &body.requester_name, &body.requester_role,
                &body.item, &body.quantity, &body.category, &body.scope,
                &body.classroom_name, &body.teacher_name,
                &body.amount_spent, &body.payment_method, &body.purchase_date, &body.payment_notes,
            ],
        ).await.map_err(|e| AppError::Database(format!("Failed to create manual expense: {}", e)))?;

        Ok(self.row_to_request(&row))
    }
}
