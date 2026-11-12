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
            drive_type: row.get("drive_type"),
            name: row.get("name"),
            item_type: row.get("item_type"),
            mime_type: row.get("mime_type"),
            file_size: row.get("file_size"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            modified_by_name: row.get("modified_by_name"),
        }
    }

    pub async fn list_items(
        &self,
        school_id: Uuid,
        drive_type: &str,
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

        let query = format!(
            "SELECT id, folder_id, user_id, drive_type, name, item_type, mime_type, file_size, \
                    created_at, updated_at, modified_by_name \
             FROM drive_items \
             WHERE school_id = $1 AND drive_type = $2 AND folder_id {} \
             ORDER BY (item_type = 'folder') DESC, {} NULLS LAST, name ASC",
            if folder_id.is_some() { "= $3" } else { "IS NULL" },
            format!("{} {}", sort_col, sort_dir),
        );

        let rows = if let Some(fid) = folder_id {
            client.query(&query as &str, &[&school_id, &drive_type, &fid]).await
        } else {
            client.query(&query as &str, &[&school_id, &drive_type]).await
        }.map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.iter().map(Self::row_to_item).collect())
    }

    pub async fn create_folder(
        &self,
        school_id: Uuid,
        user_id: Uuid,
        drive_type: &str,
        folder_id: Option<Uuid>,
        name: &str,
        modified_by_user_id: Uuid,
        modified_by_name: &str,
    ) -> ApiResult<DriveItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            "INSERT INTO drive_items \
               (school_id, user_id, drive_type, folder_id, name, item_type, \
                modified_by_user_id, modified_by_name) \
             VALUES ($1, $2, $3, $4, $5, 'folder', $6, $7) \
             RETURNING id, folder_id, user_id, drive_type, name, item_type, mime_type, file_size, \
                       created_at, updated_at, modified_by_name",
            &[&school_id, &user_id, &drive_type, &folder_id, &name,
              &modified_by_user_id, &modified_by_name],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self::row_to_item(&row))
    }

    pub async fn register_file(
        &self,
        school_id: Uuid,
        user_id: Uuid,
        drive_type: &str,
        folder_id: Option<Uuid>,
        name: &str,
        s3_key: &str,
        mime_type: &str,
        file_size: i64,
        modified_by_user_id: Uuid,
        modified_by_name: &str,
    ) -> ApiResult<DriveItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_one(
            "INSERT INTO drive_items \
               (school_id, user_id, drive_type, folder_id, name, item_type, s3_key, mime_type, file_size, \
                modified_by_user_id, modified_by_name) \
             VALUES ($1, $2, $3, $4, $5, 'file', $6, $7, $8, $9, $10) \
             RETURNING id, folder_id, user_id, drive_type, name, item_type, mime_type, file_size, \
                       created_at, updated_at, modified_by_name",
            &[&school_id, &user_id, &drive_type, &folder_id, &name,
              &s3_key, &mime_type, &file_size, &modified_by_user_id, &modified_by_name],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self::row_to_item(&row))
    }

    pub async fn rename_item(
        &self,
        id: Uuid,
        name: &str,
        modified_by_user_id: Uuid,
        modified_by_name: &str,
    ) -> ApiResult<DriveItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt(
            "UPDATE drive_items \
             SET name = $1, updated_at = NOW(), modified_by_user_id = $2, modified_by_name = $3 \
             WHERE id = $4 \
             RETURNING id, folder_id, user_id, drive_type, name, item_type, mime_type, file_size, \
                       created_at, updated_at, modified_by_name",
            &[&name, &modified_by_user_id, &modified_by_name, &id],
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
            "SELECT id, folder_id, user_id, drive_type, name, item_type, mime_type, file_size, \
                    created_at, updated_at, modified_by_name \
             FROM drive_items WHERE id = $1 AND school_id = $2",
            &[&id, &school_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(row.map(|r| Self::row_to_item(&r)))
    }

    pub async fn move_item(
        &self,
        id: Uuid,
        new_folder_id: Option<Uuid>,
        modified_by_user_id: Uuid,
        modified_by_name: &str,
    ) -> ApiResult<DriveItem> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let row = client.query_opt(
            "UPDATE drive_items \
             SET folder_id = $1, updated_at = NOW(), modified_by_user_id = $2, modified_by_name = $3 \
             WHERE id = $4 \
             RETURNING id, folder_id, user_id, drive_type, name, item_type, mime_type, file_size, \
                       created_at, updated_at, modified_by_name",
            &[&new_folder_id, &modified_by_user_id, &modified_by_name, &id],
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

    pub async fn get_user_display_name(&self, user_id: Uuid) -> Option<String> {
        let client = self.pool.get().await.ok()?;
        let row = client.query_opt(
            "SELECT first_name || ' ' || last_name AS display_name FROM users WHERE id = $1",
            &[&user_id],
        ).await.ok()??;
        row.get("display_name")
    }

    /// Recursively collect S3 keys of all files nested under a folder (for cleanup before deletion).
    pub async fn get_all_s3_keys_under_folder(&self, folder_id: Uuid) -> ApiResult<Vec<String>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = client.query(
            "WITH RECURSIVE subtree AS ( \
                SELECT id, item_type, s3_key FROM drive_items WHERE id = $1 \
                UNION ALL \
                SELECT di.id, di.item_type, di.s3_key FROM drive_items di \
                JOIN subtree s ON di.folder_id = s.id \
             ) \
             SELECT s3_key FROM subtree WHERE item_type = 'file' AND s3_key IS NOT NULL",
            &[&folder_id],
        ).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(rows.iter().filter_map(|r| r.get::<_, Option<String>>("s3_key")).collect())
    }
}
