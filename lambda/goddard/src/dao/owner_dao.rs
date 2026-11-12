use deadpool_postgres::Pool;
use uuid::Uuid;
use crate::dao::auth_dao::UserDetails;
use crate::error::{AppError, ApiResult};

#[derive(Clone)]
pub struct OwnerDao {
    pool: Pool,
}

impl OwnerDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn get_owners_by_school(&self, school_id: Uuid) -> ApiResult<Vec<UserDetails>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT u.id, u.school_id, u.first_name, u.last_name, u.email, u.role,
                   CASE WHEN au.raw_user_meta_data->>'password_set' = 'true' THEN true ELSE false END as is_verified,
                   u.metadata->>'phone_number' as phone_number,
                   u.taptime_employee_id, u.taptime_pin,
                   u.created_at, COALESCE(u.permissions, '[]'::jsonb) as permissions
            FROM users u
            LEFT JOIN auth.users au ON au.email = u.email
            WHERE u.school_id = $1 AND u.role = 'Owner' AND (u.is_active = true OR u.is_active IS NULL)
            ORDER BY u.first_name, u.last_name
        "#;

        let rows = client.query(query, &[&school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to query owners: {}", e)))?;

        let mut owners = Vec::new();
        for row in rows {
            let created_at_naive: chrono::NaiveDateTime = row.get("created_at");
            let created_at = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(created_at_naive, chrono::Utc);

            owners.push(UserDetails {
                id: row.get("id"),
                school_id: row.get("school_id"),
                first_name: row.get("first_name"),
                last_name: row.get("last_name"),
                email: row.get("email"),
                role: row.get("role"),
                is_verified: row.get("is_verified"),
                phone_number: row.get("phone_number"),
                taptime_employee_id: row.get("taptime_employee_id"),
                taptime_pin: row.get("taptime_pin"),
                created_at,
                permissions: {
                    let v: serde_json::Value = row.get("permissions");
                    serde_json::from_value(v).unwrap_or_default()
                },
            });
        }

        Ok(owners)
    }

    pub async fn get_owner_by_id(&self, id: Uuid, school_id: Uuid) -> ApiResult<Option<UserDetails>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT u.id, u.school_id, u.first_name, u.last_name, u.email, u.role,
                   COALESCE(u.is_verified, false) as is_verified,
                   u.metadata->>'phone_number' as phone_number,
                   u.taptime_employee_id, u.taptime_pin,
                   u.created_at, COALESCE(u.permissions, '[]'::jsonb) as permissions
            FROM users u
            WHERE u.id = $1 AND u.school_id = $2 AND u.role = 'Owner' AND (u.is_active = true OR u.is_active IS NULL)
        "#;

        let row_opt = client.query_opt(query, &[&id, &school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get owner: {}", e)))?;

        Ok(row_opt.map(|row| {
            let created_at_naive: chrono::NaiveDateTime = row.get("created_at");
            let created_at = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(created_at_naive, chrono::Utc);
            UserDetails {
                id: row.get("id"),
                school_id: row.get("school_id"),
                first_name: row.get("first_name"),
                last_name: row.get("last_name"),
                email: row.get("email"),
                role: row.get("role"),
                is_verified: row.get("is_verified"),
                phone_number: row.get("phone_number"),
                taptime_employee_id: row.get("taptime_employee_id"),
                taptime_pin: row.get("taptime_pin"),
                created_at,
                permissions: {
                    let v: serde_json::Value = row.get("permissions");
                    serde_json::from_value(v).unwrap_or_default()
                },
            }
        }))
    }

    pub async fn update_owner(
        &self,
        id: Uuid,
        school_id: Uuid,
        first_name: Option<String>,
        last_name: Option<String>,
        phone_number: Option<String>,
    ) -> ApiResult<UserDetails> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE users
            SET first_name = COALESCE($3, first_name),
                last_name = COALESCE($4, last_name),
                metadata = CASE
                    WHEN $5::text IS NOT NULL THEN
                        COALESCE(metadata, '{}'::jsonb) || jsonb_build_object('phone_number', $5::text)
                    ELSE metadata
                END,
                updated_at = NOW()
            WHERE id = $1 AND school_id = $2 AND role = 'Owner' AND (is_active = true OR is_active IS NULL)
            RETURNING id, school_id, first_name, last_name, email, role,
                      COALESCE(is_verified, false) as is_verified, metadata->>'phone_number' as phone_number,
                      taptime_employee_id, taptime_pin, created_at, COALESCE(permissions, '[]'::jsonb) as permissions
        "#;

        match client.query_opt(query, &[&id, &school_id, &first_name, &last_name, &phone_number]).await {
            Ok(Some(row)) => {
                let created_at_naive: chrono::NaiveDateTime = row.get("created_at");
                let created_at = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(created_at_naive, chrono::Utc);
                Ok(UserDetails {
                    id: row.get("id"),
                    school_id: row.get("school_id"),
                    first_name: row.get("first_name"),
                    last_name: row.get("last_name"),
                    email: row.get("email"),
                    role: row.get("role"),
                    is_verified: row.get("is_verified"),
                    phone_number: row.get("phone_number"),
                    taptime_employee_id: row.get("taptime_employee_id"),
                    taptime_pin: row.get("taptime_pin"),
                    created_at,
                    permissions: {
                        let v: serde_json::Value = row.get("permissions");
                        serde_json::from_value(v).unwrap_or_default()
                    },
                })
            }
            Ok(None) => Err(AppError::NotFound("Owner not found".to_string())),
            Err(e) => Err(AppError::Database(format!("Failed to update owner: {}", e))),
        }
    }

    pub async fn soft_delete_owner(&self, id: Uuid, school_id: Uuid) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE users
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND school_id = $2 AND role = 'Owner' AND (is_active = true OR is_active IS NULL)
        "#;

        let result = client.execute(query, &[&id, &school_id]).await
            .map_err(|e| AppError::Database(format!("Failed to delete owner: {}", e)))?;

        if result == 0 {
            return Err(AppError::NotFound("Owner not found or already deactivated".to_string()));
        }

        Ok(())
    }
}
