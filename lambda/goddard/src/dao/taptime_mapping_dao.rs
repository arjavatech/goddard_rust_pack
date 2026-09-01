use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};

/// Non-secret information used only while diagnosing the development Lambda.
/// Do not add DATABASE_URL, credentials, or configuration values here.
#[derive(Clone, Debug, serde::Serialize)]
pub struct TapTimeDatabaseDiagnostics {
    pub current_database: String,
    pub current_user: String,
    pub current_schema: String,
    pub server_address: String,
    pub search_path: String,
    pub resolved_relation: Option<String>,
    pub public_relation: Option<String>,
    pub has_goddard_user_id: bool,
}

#[derive(Clone, Debug)]
pub struct TapTimeUserMapping {
    pub goddard_user_id: Uuid,
    pub taptime_emp_id: Uuid,
    pub last_push_at: Option<String>,
    pub last_push_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TapTimeEligibleUser {
    pub user_id: Uuid,
    pub employee_id: Option<Uuid>,
    pub school_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub taptime_employee_id: Option<Uuid>,
    pub taptime_pin: Option<String>,
}

#[derive(Clone)]
pub struct TapTimeMappingDao {
    pool: Pool,
}

impl TapTimeMappingDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn eligible_users(&self, school_id: Uuid) -> ApiResult<Vec<TapTimeEligibleUser>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let rows = client
            .query(
                r#"SELECT u.id AS user_id, e.id AS employee_id, u.school_id,
                          u.first_name, u.last_name, u.email, u.role,
                          COALESCE(e.phone, u.metadata->>'phone_number') AS phone,
                          COALESCE(u.is_active, true) AS is_active,
                          u.taptime_employee_id, u.taptime_pin
                   FROM users u
                   LEFT JOIN employees e ON e.user_id = u.id AND e.school_id = u.school_id
                   WHERE u.school_id = $1
                     AND u.role IN ('Employee', 'Admin', 'SuperAdmin')
                     AND COALESCE(u.is_active, true) = true
                   ORDER BY u.first_name, u.last_name"#,
                &[&school_id],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to load TapTime mappings: {e}")))?;

        Ok(rows.into_iter().map(|row| TapTimeEligibleUser {
            user_id: row.get("user_id"), employee_id: row.get("employee_id"), school_id: row.get("school_id"),
            first_name: row.get("first_name"), last_name: row.get("last_name"), email: row.get("email"),
            phone: row.get("phone"), role: row.get("role"), is_active: row.get("is_active"),
            taptime_employee_id: row.get("taptime_employee_id"), taptime_pin: row.get("taptime_pin"),
        }).collect())
    }

    pub async fn database_diagnostics(&self) -> ApiResult<TapTimeDatabaseDiagnostics> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        let row = client
            .query_one(
                r#"SELECT current_database() AS current_database,
                          current_user AS current_user,
                          current_schema() AS current_schema,
                          COALESCE(inet_server_addr()::text, '') AS server_address,
                          current_setting('search_path') AS search_path,
                          to_regclass('taptime_user_mappings')::text AS resolved_relation,
                          to_regclass('public.taptime_user_mappings')::text AS public_relation,
                          EXISTS (
                              SELECT 1
                              FROM information_schema.columns
                              WHERE table_schema = 'public'
                                AND table_name = 'taptime_user_mappings'
                                AND column_name = 'goddard_user_id'
                          ) AS has_goddard_user_id"#,
                &[],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to inspect TapTime mapping database: {e}")))?;

        Ok(TapTimeDatabaseDiagnostics {
            current_database: row.get("current_database"),
            current_user: row.get("current_user"),
            current_schema: row.get("current_schema"),
            server_address: row.get("server_address"),
            search_path: row.get("search_path"),
            resolved_relation: row.get("resolved_relation"),
            public_relation: row.get("public_relation"),
            has_goddard_user_id: row.get("has_goddard_user_id"),
        })
    }

    pub async fn save_identity(&self, user_id: Uuid, emp_id: Uuid, pin: &str) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        client.execute(
            "UPDATE users SET taptime_employee_id = $2, taptime_pin = $3, updated_at = NOW() WHERE id = $1",
            &[&user_id, &emp_id, &pin],
        ).await.map_err(|e| AppError::Database(format!("Failed to save TapTime identity: {e}")))?;
        Ok(())
    }

    pub async fn create(
        &self,
        school_id: Uuid,
        goddard_user_id: Uuid,
        taptime_emp_id: Uuid,
        role: &str,
        mapped_by: Uuid,
    ) -> ApiResult<()> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {e}")))?;
        client.execute(
            "INSERT INTO taptime_user_mappings (school_id, goddard_user_id, taptime_emp_id, user_role, mapped_by)\
             VALUES ($1, $2, $3, $4, $5)",
            &[&school_id, &goddard_user_id, &taptime_emp_id, &role, &mapped_by],
        ).await.map_err(|e| {
            if e.code().map(|code| code.code()) == Some("23505") {
                AppError::Conflict("This Goddard user or TapTime employee is already mapped for this school".into())
            } else {
                AppError::Database(format!("Failed to save TapTime mapping: {e}"))
            }
        })?;
        Ok(())
    }
}
