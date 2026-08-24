use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::error::error_types::AppError;
use crate::models::tap_time::{EmployeeSyncData, TapTimeConnection, TapTimeLinkedPerson, TapTimeRoleLinkSummary, TapTimeSyncJob};

#[derive(Clone)]
pub struct TapTimeDao {
    pool: Pool,
}

impl TapTimeDao {
    pub fn new(pool: Pool) -> Self { Self { pool } }

    /// Atomically acquire a short-lived per-school retry lock. This prevents
    /// duplicate browser requests from concurrently sending the same people.
    pub async fn acquire_sync_lock(&self, school_id: Uuid) -> Result<bool, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let changed = client.execute(
            "UPDATE tap_time_connections SET sync_started_at = NOW(), updated_at = NOW() \
             WHERE school_id = $1 AND status = 'active' \
               AND (sync_started_at IS NULL OR sync_started_at < NOW() - INTERVAL '5 minutes')",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to acquire Tap-Time sync lock: {e}")))?;
        Ok(changed == 1)
    }

    pub async fn release_sync_lock(&self, school_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        client.execute(
            "UPDATE tap_time_connections SET sync_started_at = NULL, updated_at = NOW() WHERE school_id = $1",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to release Tap-Time sync lock: {e}")))?;
        Ok(())
    }

    pub async fn get_connection(&self, school_id: Uuid) -> Result<Option<TapTimeConnection>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let row = client.query_opt(
            "SELECT school_id, tap_company_id, tap_company_name, tap_timezone, status, connected_by, connected_at, \
                    disconnected_at, last_health_check_at, last_error \
             FROM tap_time_connections WHERE school_id = $1",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to load Tap-Time connection: {e}")))?;
        Ok(row.map(|row| TapTimeConnection {
            school_id: row.get("school_id"),
            tap_company_id: row.get("tap_company_id"),
            tap_company_name: row.get("tap_company_name"),
            tap_timezone: row.get("tap_timezone"),
            status: row.get("status"),
            connected_by: row.get("connected_by"),
            connected_at: row.get("connected_at"),
            disconnected_at: row.get("disconnected_at"),
            last_health_check_at: row.get("last_health_check_at"),
            last_error: row.get("last_error"),
        }))
    }

    pub async fn connection_access_token_material(&self, school_id: Uuid) -> Result<Option<(Vec<u8>, Vec<u8>)>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let row = client.query_opt(
            "SELECT access_token_ciphertext, access_token_nonce FROM tap_time_connections WHERE school_id = $1 AND status = 'active'",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to load Tap-Time credential: {e}")))?;
        Ok(row.and_then(|row| {
            let ciphertext: Option<Vec<u8>> = row.get("access_token_ciphertext");
            let nonce: Option<Vec<u8>> = row.get("access_token_nonce");
            ciphertext.zip(nonce)
        }))
    }

    /// Queue only a Goddard employee identifier. The dispatcher re-reads profile data;
    /// this makes the outbox safe to retain and guarantees PINs can never enter it.
    pub async fn enqueue_employee_sync_if_connected(&self, school_id: Uuid, employee_id: Uuid, operation: &str) -> Result<bool, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let active = client.query_opt(
            "SELECT 1 FROM tap_time_connections WHERE school_id = $1 AND status = 'active'",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to check Tap-Time connection: {e}")))?.is_some();
        if !active { return Ok(false); }
        client.execute(
            "INSERT INTO tap_time_sync_outbox (school_id, employee_id, operation, payload) VALUES ($1, $2, $3, jsonb_build_object('employee_id', $2::text))",
            &[&school_id, &employee_id, &operation],
        ).await.map_err(|e| AppError::Database(format!("Failed to queue Tap-Time employee sync: {e}")))?;
        Ok(true)
    }

    pub async fn claim_sync_jobs(&self, school_id: Uuid, limit: i64) -> Result<Vec<TapTimeSyncJob>, AppError> {
        let mut client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let transaction = client.transaction().await.map_err(|e| AppError::Database(format!("Failed to begin sync transaction: {e}")))?;
        let rows = transaction.query(
            "WITH ready AS (SELECT id FROM tap_time_sync_outbox WHERE school_id = $1 AND status IN ('pending', 'failed') AND next_attempt_at <= NOW() ORDER BY created_at LIMIT $2 FOR UPDATE SKIP LOCKED) \
             UPDATE tap_time_sync_outbox o SET status = 'processing', attempt_count = attempt_count + 1 \
             FROM ready WHERE o.id = ready.id RETURNING o.id, o.school_id, o.employee_id, o.operation",
            &[&school_id, &limit],
        ).await.map_err(|e| AppError::Database(format!("Failed to claim Tap-Time sync jobs: {e}")))?;
        transaction.commit().await.map_err(|e| AppError::Database(format!("Failed to claim Tap-Time sync jobs: {e}")))?;
        Ok(rows.into_iter().filter_map(|row| {
            let employee_id: Option<Uuid> = row.get("employee_id");
            employee_id.map(|employee_id| TapTimeSyncJob {
                id: row.get("id"), school_id: row.get("school_id"), employee_id, operation: row.get("operation"),
            })
        }).collect())
    }

    pub async fn employee_sync_data(&self, employee_id: Uuid, school_id: Uuid) -> Result<EmployeeSyncData, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let row = client.query_opt(
            "SELECT e.id, e.school_id, u.first_name, u.last_name, u.email, e.phone, COALESCE(e.is_active, TRUE) AS is_active \
             FROM employees e JOIN users u ON u.id = e.user_id WHERE e.id = $1 AND e.school_id = $2",
            &[&employee_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to load employee for Tap-Time sync: {e}")))?
            .ok_or_else(|| AppError::NotFound("Employee".to_string()))?;
        Ok(EmployeeSyncData { id: row.get("id"), school_id: row.get("school_id"), first_name: row.get("first_name"), last_name: row.get("last_name"), email: row.get("email"), phone: row.get("phone"), is_active: row.get("is_active"), is_admin: 0 })
    }

    pub async fn school_employee_sync_data(&self, school_id: Uuid) -> Result<Vec<EmployeeSyncData>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let rows = client.query(
            "SELECT e.id, e.school_id, u.first_name, u.last_name, u.email, e.phone, COALESCE(e.is_active, TRUE) AS is_active \
             FROM employees e JOIN users u ON u.id = e.user_id WHERE e.school_id = $1",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to list school employees: {e}")))?;
        Ok(rows.into_iter().map(|row| EmployeeSyncData { id: row.get("id"), school_id: row.get("school_id"), first_name: row.get("first_name"), last_name: row.get("last_name"), email: row.get("email"), phone: row.get("phone"), is_active: row.get("is_active"), is_admin: 0 }).collect())
    }

    /// Admins live in `users`, not `employees`; their Goddard user id is the
    /// permanent external id sent to Tap-Time.
    pub async fn school_admin_sync_data(&self, school_id: Uuid) -> Result<Vec<EmployeeSyncData>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let rows = client.query(
            "SELECT id, school_id, first_name, last_name, email, COALESCE(NULLIF(BTRIM(phone_number), ''), metadata->>'phone_number') AS phone, COALESCE(is_active, TRUE) AS is_active, role \
             FROM users WHERE school_id = $1 AND role IN ('Admin', 'SuperAdmin')",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to list school admins for Tap-Time sync: {e}")))?;
        Ok(rows.into_iter().map(|row| {
            let role: String = row.get("role");
            EmployeeSyncData { id: row.get("id"), school_id: row.get("school_id"), first_name: row.get("first_name"), last_name: row.get("last_name"), email: row.get("email"), phone: row.get("phone"), is_active: row.get("is_active"), is_admin: if role == "SuperAdmin" { 2 } else { 1 } }
        }).collect())
    }

    pub async fn integration_role_summaries(&self, school_id: Uuid) -> Result<(TapTimeRoleLinkSummary, TapTimeRoleLinkSummary, TapTimeRoleLinkSummary, i64), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let row = client.query_one(
            "SELECT \
                (SELECT COUNT(*) FROM employees WHERE school_id = $1) AS employee_total, \
                (SELECT COUNT(*) FROM employees e JOIN tap_time_employee_links l ON l.employee_id = e.id WHERE e.school_id = $1 AND l.tap_employee_id IS NOT NULL) AS employee_linked, \
                (SELECT COUNT(*) FROM users WHERE school_id = $1 AND role = 'Admin') AS admin_total, \
                (SELECT COUNT(*) FROM users u JOIN tap_time_user_links l ON l.user_id = u.id WHERE u.school_id = $1 AND u.role = 'Admin' AND l.tap_employee_id IS NOT NULL) AS admin_linked, \
                (SELECT COUNT(*) FROM users WHERE school_id = $1 AND role = 'SuperAdmin') AS super_admin_total, \
                (SELECT COUNT(*) FROM users u JOIN tap_time_user_links l ON l.user_id = u.id WHERE u.school_id = $1 AND u.role = 'SuperAdmin' AND l.tap_employee_id IS NOT NULL) AS super_admin_linked, \
                (SELECT COUNT(*) FROM tap_time_employee_links WHERE school_id = $1 AND sync_status = 'failed') + \
                (SELECT COUNT(*) FROM tap_time_user_links WHERE school_id = $1 AND sync_status = 'failed') AS failed_syncs",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to load Tap-Time integration summary: {e}")))?;
        Ok((
            TapTimeRoleLinkSummary { total: row.get("employee_total"), linked: row.get("employee_linked") },
            TapTimeRoleLinkSummary { total: row.get("admin_total"), linked: row.get("admin_linked") },
            TapTimeRoleLinkSummary { total: row.get("super_admin_total"), linked: row.get("super_admin_linked") },
            row.get("failed_syncs"),
        ))
    }

    pub async fn linked_people(&self, school_id: Uuid) -> Result<Vec<TapTimeLinkedPerson>, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let rows = client.query(
            "SELECT e.id AS entity_id, 'employee'::TEXT AS entity_type, 'Employee'::TEXT AS role, \
                    CONCAT_WS(' ', u.first_name, u.last_name) AS person_name, u.email, e.phone AS phone_number, \
                    l.tap_employee_id, l.sync_status, l.last_synced_at AS linked_at \
             FROM employees e JOIN users u ON u.id = e.user_id JOIN tap_time_employee_links l ON l.employee_id = e.id \
             WHERE e.school_id = $1 AND l.tap_employee_id IS NOT NULL \
             UNION ALL \
             SELECT u.id AS entity_id, 'user'::TEXT AS entity_type, \
                    CASE WHEN u.role = 'SuperAdmin' THEN 'Super Admin' ELSE 'Admin' END AS role, \
                    CONCAT_WS(' ', u.first_name, u.last_name) AS person_name, u.email, \
                    COALESCE(NULLIF(BTRIM(u.phone_number), ''), u.metadata->>'phone_number') AS phone_number, \
                    l.tap_employee_id, l.sync_status, l.last_synced_at AS linked_at \
             FROM users u JOIN tap_time_user_links l ON l.user_id = u.id \
             WHERE u.school_id = $1 AND u.role IN ('Admin', 'SuperAdmin') AND l.tap_employee_id IS NOT NULL \
             ORDER BY role, person_name",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to load linked Tap-Time people: {e}")))?;
        Ok(rows.into_iter().map(|row| TapTimeLinkedPerson {
            entity_id: row.get("entity_id"), entity_type: row.get("entity_type"), role: row.get("role"),
            person_name: row.get("person_name"), email: row.get("email"), phone_number: row.get("phone_number"),
            tap_employee_id: row.get("tap_employee_id"), tap_employee_name: None,
            sync_status: row.get("sync_status"), linked_at: row.get("linked_at"),
        }).collect())
    }

    pub async fn save_reconciliation(&self, school_id: Uuid, employee_id: Uuid, tap_employee_id: Uuid, actor: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        client.execute(
            "INSERT INTO tap_time_employee_links (school_id, employee_id, tap_company_id, tap_employee_id, sync_status, last_synced_at, last_error) \
             SELECT $1, $2, c.tap_company_id, $3, 'synced', NOW(), NULL FROM tap_time_connections c WHERE c.school_id = $1 \
             ON CONFLICT (employee_id) DO UPDATE SET tap_company_id = EXCLUDED.tap_company_id, tap_employee_id = EXCLUDED.tap_employee_id, sync_status = 'synced', last_synced_at = NOW(), last_error = NULL, updated_at = NOW()",
            &[&school_id, &employee_id, &tap_employee_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to store employee reconciliation: {e}")))?;
        self.audit(&client, school_id, Some(actor), "employee_reconciled", "employee", Some(employee_id)).await
    }

    pub async fn linked_employee_for_user(&self, user_id: Uuid, school_id: Uuid) -> Result<Uuid, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let row = client.query_opt(
            "SELECT e.id FROM employees e JOIN tap_time_employee_links l ON l.employee_id = e.id \
             JOIN tap_time_connections c ON c.school_id = e.school_id AND c.status = 'active' \
             WHERE e.user_id = $1 AND e.school_id = $2 AND l.tap_employee_id IS NOT NULL",
            &[&user_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to load linked employee: {e}")))?
            .ok_or_else(|| AppError::NotFound("Linked employee".to_string()))?;
        Ok(row.get("id"))
    }

    pub async fn ensure_linked_user(&self, user_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let exists = client.query_opt(
            "SELECT 1 FROM tap_time_user_links l JOIN tap_time_connections c ON c.school_id = l.school_id AND c.status = 'active' \
             WHERE l.user_id = $1 AND l.school_id = $2 AND l.tap_employee_id IS NOT NULL",
            &[&user_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to verify linked admin: {e}")))?.is_some();
        if exists { Ok(()) } else { Err(AppError::NotFound("Linked Tap-Time admin".to_string())) }
    }

    pub async fn complete_admin_sync(&self, school_id: Uuid, user_id: Uuid, tap_employee_id: Uuid, sync_status: &str) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        client.execute(
            "INSERT INTO tap_time_user_links (school_id, user_id, tap_company_id, tap_employee_id, sync_status, last_synced_at, last_error) \
             SELECT $1, $2, c.tap_company_id, $3, $4, NOW(), NULL FROM tap_time_connections c WHERE c.school_id = $1 \
             ON CONFLICT (user_id) DO UPDATE SET tap_company_id = EXCLUDED.tap_company_id, tap_employee_id = EXCLUDED.tap_employee_id, sync_status = EXCLUDED.sync_status, last_synced_at = NOW(), last_error = NULL, updated_at = NOW()",
            &[&school_id, &user_id, &tap_employee_id, &sync_status],
        ).await.map_err(|e| AppError::Database(format!("Failed to update Tap-Time admin link: {e}")))?;
        self.record_user_audit(school_id, user_id, "admin_synced", user_id).await
    }

    pub async fn fail_admin_sync(&self, school_id: Uuid, user_id: Uuid, error: &str) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        client.execute(
            "INSERT INTO tap_time_user_links (school_id, user_id, tap_company_id, tap_employee_id, sync_status, last_error) \
             SELECT $1, $2, c.tap_company_id, NULL, 'failed', LEFT($3, 1000) FROM tap_time_connections c WHERE c.school_id = $1 \
             ON CONFLICT (user_id) DO UPDATE SET sync_status = 'failed', last_error = EXCLUDED.last_error, updated_at = NOW()",
            &[&school_id, &user_id, &error],
        ).await.map_err(|e| AppError::Database(format!("Failed to store Tap-Time admin sync failure: {e}")))?;
        Ok(())
    }

    pub async fn save_user_reconciliation(&self, school_id: Uuid, user_id: Uuid, tap_employee_id: Uuid, actor: Uuid) -> Result<(), AppError> {
        self.complete_admin_sync(school_id, user_id, tap_employee_id, "synced").await?;
        self.record_user_audit(school_id, actor, "admin_reconciled", user_id).await
    }

    pub async fn ensure_linked_employee(&self, employee_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let exists = client.query_opt(
            "SELECT 1 FROM tap_time_employee_links l JOIN tap_time_connections c ON c.school_id = l.school_id AND c.status = 'active' \
             WHERE l.employee_id = $1 AND l.school_id = $2 AND l.tap_employee_id IS NOT NULL",
            &[&employee_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to verify linked employee: {e}")))?.is_some();
        if exists { Ok(()) } else { Err(AppError::NotFound("Linked employee".to_string())) }
    }

    pub async fn record_audit(&self, school_id: Uuid, actor: Uuid, action: &str, employee_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        self.audit(&client, school_id, Some(actor), action, "employee", Some(employee_id)).await
    }

    pub async fn record_user_audit(&self, school_id: Uuid, actor: Uuid, action: &str, user_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        self.audit(&client, school_id, Some(actor), action, "user", Some(user_id)).await
    }

    pub async fn complete_sync_job(&self, job: &TapTimeSyncJob, tap_employee_id: Uuid, sync_status: &str) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        client.execute(
            "INSERT INTO tap_time_employee_links (school_id, employee_id, tap_company_id, tap_employee_id, sync_status, last_synced_at, last_error) \
             SELECT $1, $2, c.tap_company_id, $3, $4, NOW(), NULL FROM tap_time_connections c WHERE c.school_id = $1 \
             ON CONFLICT (employee_id) DO UPDATE SET tap_company_id = EXCLUDED.tap_company_id, tap_employee_id = EXCLUDED.tap_employee_id, sync_status = EXCLUDED.sync_status, last_synced_at = NOW(), last_error = NULL, updated_at = NOW()",
            &[&job.school_id, &job.employee_id, &tap_employee_id, &sync_status],
        ).await.map_err(|e| AppError::Database(format!("Failed to update Tap-Time employee link: {e}")))?;
        client.execute("UPDATE tap_time_sync_outbox SET status = 'completed', completed_at = NOW(), last_error = NULL WHERE id = $1", &[&job.id])
            .await.map_err(|e| AppError::Database(format!("Failed to complete Tap-Time sync job: {e}")))?;
        Ok(())
    }

    pub async fn fail_sync_job(&self, job_id: Uuid, error: &str) -> Result<(), AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        client.execute(
            "UPDATE tap_time_sync_outbox SET status = 'failed', last_error = LEFT($2, 1000), next_attempt_at = NOW() + INTERVAL '5 minutes' WHERE id = $1",
            &[&job_id, &error],
        ).await.map_err(|e| AppError::Database(format!("Failed to mark Tap-Time sync failure: {e}")))?;
        Ok(())
    }

    pub async fn save_connection(
        &self,
        school_id: Uuid,
        tap_company_id: Uuid,
        tap_company_name: &str,
        tap_timezone: Option<&str>,
        actor_user_id: Uuid,
        access_token_ciphertext: &[u8],
        access_token_nonce: &[u8],
    ) -> Result<TapTimeConnection, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let row = client.query_one(
            "INSERT INTO tap_time_connections \
                (school_id, tap_company_id, tap_company_name, tap_timezone, status, connected_by, connected_at, disconnected_by, disconnected_at, last_error, access_token_ciphertext, access_token_nonce, updated_at) \
             VALUES ($1, $2, $3, $4, 'active', $5, NOW(), NULL, NULL, NULL, $6, $7, NOW()) \
             ON CONFLICT (school_id) DO UPDATE SET \
                tap_company_id = EXCLUDED.tap_company_id, tap_company_name = EXCLUDED.tap_company_name, \
                tap_timezone = EXCLUDED.tap_timezone, status = 'active', connected_by = EXCLUDED.connected_by, \
                connected_at = NOW(), disconnected_by = NULL, disconnected_at = NULL, last_error = NULL, access_token_ciphertext = EXCLUDED.access_token_ciphertext, access_token_nonce = EXCLUDED.access_token_nonce, updated_at = NOW() \
             RETURNING school_id, tap_company_id, tap_company_name, tap_timezone, status, connected_by, connected_at, \
                       disconnected_at, last_health_check_at, last_error",
            &[&school_id, &tap_company_id, &tap_company_name, &tap_timezone, &actor_user_id, &access_token_ciphertext, &access_token_nonce],
        ).await.map_err(|e| AppError::Database(format!("Failed to save Tap-Time connection: {e}")))?;
        self.audit(&client, school_id, Some(actor_user_id), "connection_linked", "connection", None).await?;
        Ok(TapTimeConnection {
            school_id: row.get("school_id"), tap_company_id: row.get("tap_company_id"),
            tap_company_name: row.get("tap_company_name"), tap_timezone: row.get("tap_timezone"),
            status: row.get("status"), connected_by: row.get("connected_by"), connected_at: row.get("connected_at"),
            disconnected_at: row.get("disconnected_at"), last_health_check_at: row.get("last_health_check_at"), last_error: row.get("last_error"),
        })
    }

    pub async fn mark_disconnected(&self, school_id: Uuid, actor_user_id: Uuid) -> Result<TapTimeConnection, AppError> {
        let client = self.pool.get().await.map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let row = client.query_opt(
            "UPDATE tap_time_connections SET status = 'disconnected', disconnected_by = $2, disconnected_at = NOW(), updated_at = NOW() \
             WHERE school_id = $1 \
             RETURNING school_id, tap_company_id, tap_company_name, tap_timezone, status, connected_by, connected_at, \
                       disconnected_at, last_health_check_at, last_error",
            &[&school_id, &actor_user_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to disconnect Tap-Time: {e}")))?;
        let row = row.ok_or_else(|| AppError::NotFound("Tap-Time connection".to_string()))?;
        self.audit(&client, school_id, Some(actor_user_id), "connection_disconnected", "connection", None).await?;
        Ok(TapTimeConnection {
            school_id: row.get("school_id"), tap_company_id: row.get("tap_company_id"),
            tap_company_name: row.get("tap_company_name"), tap_timezone: row.get("tap_timezone"),
            status: row.get("status"), connected_by: row.get("connected_by"), connected_at: row.get("connected_at"),
            disconnected_at: row.get("disconnected_at"), last_health_check_at: row.get("last_health_check_at"), last_error: row.get("last_error"),
        })
    }

    async fn audit(&self, client: &deadpool_postgres::Client, school_id: Uuid, actor: Option<Uuid>, action: &str, entity_type: &str, entity_id: Option<Uuid>) -> Result<(), AppError> {
        client.execute(
            "INSERT INTO tap_time_audit_events (school_id, actor_user_id, action, entity_type, entity_id) VALUES ($1, $2, $3, $4, $5)",
            &[&school_id, &actor, &action, &entity_type, &entity_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to record Tap-Time audit event: {e}")))?;
        Ok(())
    }
}
