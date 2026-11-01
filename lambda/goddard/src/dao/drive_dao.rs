use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};
use crate::models::drive::*;

#[derive(Clone)]
pub struct DriveDao {
    pool: Pool,
}

impl DriveDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_item(row: &tokio_postgres::Row) -> DriveItem {
        DriveItem {
            id: row.get("id"),
            folder_id: row.get("folder_id"),
            user_id: row.get("user_id"),
            name: row.get("name"),
            item_type: row.get("item_type"),
            mime_type: row.get("mime_type"),
            file_size: row.get("file_size"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    pub async fn list_items(
        &self,
        school_id: Uuid,
        user_id: Uuid,
        folder_id: Option<Uuid>,
        sort_by: &str,
        sort_order: &str,
    ) -> ApiResult<Vec<DriveItem>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sort_col = match sort_by {
            "file_size" => "file_size",
            "updated_at" => "updated_at",
            "created_at" => "created_at",
            _ => "name",
        };
        let sort_dir = if sort_order == "desc" { "DESC" } else { "ASC" };

        // Folders sort first, then the chosen column
        let query = format!(
            "SELECT id, folder_id, user_id, name, item_type, mime_type, file_size, created_at, updated_at \
             FROM drive_items \
             WHERE school_id = $1 AND user_id = $2 AND folder_id {} \
             ORDER BY (item_type = 'folder') DESC, {} NULLS LAST, name ASC",
            if folder_id.is_some() { "= $3" } else { "IS NULL" },
            format!("{} {}", sort_col, sort_dir),
        );

        let rows = if let Some(fid) = folder_id {
            client.query(&query as &str, &[&school_id, &user_id, &fid]).await
        } else {
            client.query(&query as &str, &[&school_id, &user_id]).await
        }.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(Self::row_to_item).collect())
    }

    pub async fn create_folder(
        &self,
        school_id: Uuid,
        user_id: Uuid,
        folder_id: Option<Uuid>,
        name: &str,
    ) -> ApiResult<DriveItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            "INSERT INTO drive_items (school_id, user_id, folder_id, name, item_type) \
             VALUES ($1, $2, $3, $4, 'folder') \
             RETURNING id, folder_id, user_id, name, item_type, mime_type, file_size, created_at, updated_at",
            &[&school_id, &user_id, &folder_id, &name],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self::row_to_item(&row))
    }

    pub async fn register_file(
        &self,
        school_id: Uuid,
        user_id: Uuid,
        folder_id: Option<Uuid>,
        name: &str,
        s3_key: &str,
        mime_type: &str,
        file_size: i64,
    ) -> ApiResult<DriveItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            "INSERT INTO drive_items (school_id, user_id, folder_id, name, item_type, s3_key, mime_type, file_size) \
             VALUES ($1, $2, $3, $4, 'file', $5, $6, $7) \
             RETURNING id, folder_id, user_id, name, item_type, mime_type, file_size, created_at, updated_at",
            &[&school_id, &user_id, &folder_id, &name, &s3_key, &mime_type, &file_size],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self::row_to_item(&row))
    }

    pub async fn rename_item(&self, id: Uuid, name: &str) -> ApiResult<DriveItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt(
            "UPDATE drive_items SET name = $1, updated_at = NOW() WHERE id = $2 \
             RETURNING id, folder_id, user_id, name, item_type, mime_type, file_size, created_at, updated_at",
            &[&name, &id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        row.map(|r| Self::row_to_item(&r))
           .ok_or_else(|| AppError::NotFound("Drive item".into()))
    }

    pub async fn delete_item(&self, id: Uuid) -> ApiResult<()> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        client.execute("DELETE FROM drive_items WHERE id = $1", &[&id])
            .await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_item(&self, id: Uuid, school_id: Uuid) -> ApiResult<Option<DriveItem>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt(
            "SELECT id, folder_id, user_id, name, item_type, mime_type, file_size, created_at, updated_at \
             FROM drive_items WHERE id = $1 AND school_id = $2",
            &[&id, &school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(row.map(|r| Self::row_to_item(&r)))
    }

    pub async fn list_admins_with_drive(&self, school_id: Uuid) -> ApiResult<Vec<DriveAdminSummary>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = client.query(
            "SELECT u.id, u.first_name, u.last_name, u.email, \
                    COUNT(d.id) AS item_count \
             FROM users u \
             LEFT JOIN drive_items d ON d.user_id = u.id AND d.school_id = $1 \
             WHERE u.school_id = $1 AND u.role = 'Admin' \
               AND (u.is_active = true OR u.is_active IS NULL) \
             GROUP BY u.id, u.first_name, u.last_name, u.email \
             ORDER BY u.first_name, u.last_name",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(|row| DriveAdminSummary {
            user_id: row.get("id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            email: row.get("email"),
            item_count: row.get("item_count"),
        }).collect())
    }

    pub async fn move_item(&self, id: Uuid, new_folder_id: Option<Uuid>) -> ApiResult<DriveItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt(
            "UPDATE drive_items SET folder_id = $1, updated_at = NOW() WHERE id = $2 \
             RETURNING id, folder_id, user_id, name, item_type, mime_type, file_size, created_at, updated_at",
            &[&new_folder_id, &id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        row.map(|r| Self::row_to_item(&r))
           .ok_or_else(|| AppError::NotFound("Drive item".into()))
    }

    pub async fn get_s3_key(&self, id: Uuid, school_id: Uuid) -> ApiResult<Option<String>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt(
            "SELECT s3_key FROM drive_items WHERE id = $1 AND school_id = $2 AND item_type = 'file'",
            &[&id, &school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(row.and_then(|r| r.get::<_, Option<String>>("s3_key")))
    }
}
