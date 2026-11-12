use std::sync::Arc;
use uuid::Uuid;

use crate::dao::drive_dao::DriveDao;
use crate::dao::school_dao::SchoolDao;
use crate::error::{ApiResult, AppError};
use crate::middleware::auth::AuthContext;
use crate::models::drive::*;
use crate::models::schema::UserRole;
use crate::models::school::SchoolFeature;
use crate::services::upload_service::UploadService;

#[derive(Clone)]
pub struct DriveService {
    dao: DriveDao,
    upload: Arc<UploadService>,
    schools: SchoolDao,
}

impl DriveService {
    pub fn new(
        dao: DriveDao,
        upload: Arc<UploadService>,
        schools: SchoolDao,
    ) -> Self {
        Self { dao, upload, schools }
    }

    async fn check_access(&self, auth: &AuthContext) -> ApiResult<()> {
        if !matches!(auth.role, UserRole::SuperAdmin | UserRole::Owner) {
            self.schools.ensure_feature_enabled(auth.school_id, SchoolFeature::Drive).await?;
        }
        Ok(())
    }

    /// Determine which shared drive to use based on role and optional caller preference.
    /// Admins are always forced to their group drive. Owner/SuperAdmin can choose.
    fn resolve_drive_type(&self, auth: &AuthContext, requested: Option<&str>) -> ApiResult<String> {
        match auth.role {
            UserRole::Admin => Ok("admin".to_string()),
            UserRole::SuperAdmin | UserRole::Owner => {
                Ok(requested.unwrap_or("owner").to_string())
            }
            _ => Err(AppError::Authorization("Drive access not permitted".into())),
        }
    }

    /// Validate that the requested drive_type is accessible by this role.
    fn check_drive_type_access(&self, auth: &AuthContext, drive_type: &str) -> ApiResult<()> {
        match auth.role {
            UserRole::Admin if drive_type != "admin" => {
                Err(AppError::Authorization("Admins can only access the admin drive".into()))
            }
            UserRole::SuperAdmin | UserRole::Owner | UserRole::Admin => Ok(()),
            _ => Err(AppError::Authorization("Drive access not permitted".into())),
        }
    }

    async fn modifier_name(&self, auth: &AuthContext) -> String {
        self.dao.get_user_display_name(auth.user_id).await
            .unwrap_or_else(|| auth.email.clone())
    }

    pub async fn get_upload_url(
        &self,
        req: DriveUploadUrlRequest,
        auth: &AuthContext,
    ) -> ApiResult<DriveUploadUrlResponse> {
        self.check_access(auth).await?;

        let drive_type = self.resolve_drive_type(auth, req.drive_type.as_deref())?;
        let ext = req.file_name.rsplit('.').next().unwrap_or("bin").to_lowercase();
        let s3_key = format!("drive/{}/{}/{}.{}", auth.school_id, drive_type, Uuid::new_v4(), ext);

        let upload_url = self.upload.create_drive_upload_url(&s3_key, &req.content_type, req.file_size).await?;
        Ok(DriveUploadUrlResponse { upload_url, s3_key })
    }

    pub async fn register_file(
        &self,
        req: DriveRegisterFileRequest,
        auth: &AuthContext,
    ) -> ApiResult<DriveItem> {
        self.check_access(auth).await?;

        let drive_type = self.resolve_drive_type(auth, req.drive_type.as_deref())?;
        let expected_prefix = format!("drive/{}/{}/", auth.school_id, drive_type);
        if !req.s3_key.starts_with(&expected_prefix) {
            return Err(AppError::Authorization("Invalid S3 key for this drive".into()));
        }

        if let Some(folder_id) = req.folder_id {
            let folder = self.dao.get_item(folder_id, auth.school_id).await?
                .ok_or_else(|| AppError::NotFound("Folder".into()))?;
            if folder.drive_type != drive_type {
                return Err(AppError::Authorization("Folder is in a different drive".into()));
            }
        }

        let modifier_name = self.modifier_name(auth).await;
        self.dao.register_file(
            auth.school_id, auth.user_id, &drive_type, req.folder_id,
            &req.name, &req.s3_key, &req.mime_type, req.file_size,
            auth.user_id, &modifier_name,
        ).await
    }

    pub async fn create_folder(
        &self,
        req: DriveCreateFolderRequest,
        auth: &AuthContext,
    ) -> ApiResult<DriveItem> {
        self.check_access(auth).await?;

        if req.name.trim().is_empty() {
            return Err(AppError::Validation("Folder name cannot be empty".into()));
        }

        let drive_type = self.resolve_drive_type(auth, req.drive_type.as_deref())?;

        if let Some(folder_id) = req.folder_id {
            let folder = self.dao.get_item(folder_id, auth.school_id).await?
                .ok_or_else(|| AppError::NotFound("Parent folder".into()))?;
            if folder.drive_type != drive_type {
                return Err(AppError::Authorization("Parent folder is in a different drive".into()));
            }
        }

        let modifier_name = self.modifier_name(auth).await;
        self.dao.create_folder(
            auth.school_id, auth.user_id, &drive_type,
            req.folder_id, req.name.trim(),
            auth.user_id, &modifier_name,
        ).await
    }

    pub async fn list_items(
        &self,
        query: DriveListQuery,
        auth: &AuthContext,
    ) -> ApiResult<Vec<DriveItem>> {
        self.check_access(auth).await?;

        let drive_type = self.resolve_drive_type(auth, query.drive_type.as_deref())?;
        self.check_drive_type_access(auth, &drive_type)?;

        let sort_by = query.sort_by.as_deref().unwrap_or("name");
        let sort_order = query.sort_order.as_deref().unwrap_or("asc");

        self.dao.list_items(auth.school_id, &drive_type, query.folder_id, sort_by, sort_order).await
    }

    pub async fn get_download_url(
        &self,
        id: Uuid,
        download: bool,
        auth: &AuthContext,
    ) -> ApiResult<DriveDownloadUrlResponse> {
        self.check_access(auth).await?;

        let item = self.dao.get_item(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("Drive item".into()))?;

        self.check_drive_type_access(auth, &item.drive_type)?;

        let s3_key = self.dao.get_s3_key(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("File".into()))?;

        let url = self.upload.create_document_access_url(&s3_key, download, Some(&item.name)).await?;
        Ok(DriveDownloadUrlResponse { url })
    }

    pub async fn rename_item(
        &self,
        id: Uuid,
        name: String,
        auth: &AuthContext,
    ) -> ApiResult<DriveItem> {
        self.check_access(auth).await?;

        if name.trim().is_empty() {
            return Err(AppError::Validation("Name cannot be empty".into()));
        }

        let item = self.dao.get_item(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("Drive item".into()))?;

        self.check_drive_type_access(auth, &item.drive_type)?;

        let modifier_name = self.modifier_name(auth).await;
        self.dao.rename_item(id, name.trim(), auth.user_id, &modifier_name).await
    }

    pub async fn delete_item(&self, id: Uuid, auth: &AuthContext) -> ApiResult<()> {
        self.check_access(auth).await?;

        let item = self.dao.get_item(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("Drive item".into()))?;

        self.check_drive_type_access(auth, &item.drive_type)?;

        // Collect S3 keys while DB records still exist
        let s3_keys: Vec<String> = if item.item_type == "file" {
            self.dao.get_s3_key(id, auth.school_id).await.ok().flatten().into_iter().collect()
        } else {
            self.dao.get_all_s3_keys_under_folder(id).await.unwrap_or_default()
        };

        // Delete DB record immediately so the API returns fast
        self.dao.delete_item(id).await?;

        // S3 cleanup runs in background — doesn't block the response
        if !s3_keys.is_empty() {
            let upload = Arc::clone(&self.upload);
            tokio::spawn(async move {
                for key in s3_keys {
                    let _ = upload.delete_document_object(&key).await;
                }
            });
        }

        Ok(())
    }

    pub async fn move_item(
        &self,
        id: Uuid,
        req: DriveMoveRequest,
        auth: &AuthContext,
    ) -> ApiResult<DriveItem> {
        self.check_access(auth).await?;

        let item = self.dao.get_item(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("Drive item".into()))?;

        self.check_drive_type_access(auth, &item.drive_type)?;

        if let Some(target_folder_id) = req.folder_id {
            if target_folder_id == id {
                return Err(AppError::Validation("Cannot move item into itself".into()));
            }
            let target = self.dao.get_item(target_folder_id, auth.school_id).await?
                .ok_or_else(|| AppError::NotFound("Target folder".into()))?;
            if target.item_type != "folder" {
                return Err(AppError::Validation("Target must be a folder".into()));
            }
            if target.drive_type != item.drive_type {
                return Err(AppError::Validation("Cannot move items between different drives".into()));
            }
        }

        let modifier_name = self.modifier_name(auth).await;
        self.dao.move_item(id, req.folder_id, auth.user_id, &modifier_name).await
    }

    pub async fn copy_item(&self, id: Uuid, auth: &AuthContext) -> ApiResult<DriveItem> {
        self.check_access(auth).await?;

        let item = self.dao.get_item(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("Drive item".into()))?;

        self.check_drive_type_access(auth, &item.drive_type)?;

        let copy_name = format!("Copy of {}", item.name);
        let modifier_name = self.modifier_name(auth).await;

        if item.item_type == "file" {
            let source_key = self.dao.get_s3_key(id, auth.school_id).await?
                .ok_or_else(|| AppError::NotFound("Source file".into()))?;

            let ext = source_key.rsplit('.').next().unwrap_or("bin");
            let dest_key = format!("drive/{}/{}/{}.{}", auth.school_id, item.drive_type, Uuid::new_v4(), ext);

            self.upload.copy_s3_object(&source_key, &dest_key).await?;

            self.dao.register_file(
                auth.school_id,
                auth.user_id,
                &item.drive_type,
                item.folder_id,
                &copy_name,
                &dest_key,
                item.mime_type.as_deref().unwrap_or("application/octet-stream"),
                item.file_size.unwrap_or(0),
                auth.user_id,
                &modifier_name,
            ).await
        } else {
            self.copy_folder_recursive(
                id, &item.drive_type, &copy_name, item.folder_id, auth.school_id, auth.user_id, &modifier_name,
            ).await
        }
    }

    fn copy_folder_recursive<'a>(
        &'a self,
        source_id: Uuid,
        drive_type: &'a str,
        new_name: &'a str,
        parent_folder_id: Option<Uuid>,
        school_id: Uuid,
        actor_user_id: Uuid,
        modifier_name: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ApiResult<DriveItem>> + Send + 'a>> {
        Box::pin(async move {
            let new_folder = self.dao.create_folder(
                school_id, actor_user_id, drive_type, parent_folder_id, new_name,
                actor_user_id, modifier_name,
            ).await?;
            let children = self.dao.list_items(school_id, drive_type, Some(source_id), "name", "asc").await.unwrap_or_default();

            for child in children {
                if child.item_type == "file" {
                    if let Ok(Some(s3_key)) = self.dao.get_s3_key(child.id, school_id).await {
                        let ext = s3_key.rsplit('.').next().unwrap_or("bin");
                        let dest_key = format!("drive/{}/{}/{}.{}", school_id, drive_type, Uuid::new_v4(), ext);
                        if self.upload.copy_s3_object(&s3_key, &dest_key).await.is_ok() {
                            let _ = self.dao.register_file(
                                school_id, actor_user_id, drive_type, Some(new_folder.id), &child.name, &dest_key,
                                child.mime_type.as_deref().unwrap_or("application/octet-stream"),
                                child.file_size.unwrap_or(0),
                                actor_user_id, modifier_name,
                            ).await;
                        }
                    }
                } else {
                    let _ = self.copy_folder_recursive(
                        child.id, drive_type, &child.name, Some(new_folder.id),
                        school_id, actor_user_id, modifier_name,
                    ).await;
                }
            }

            Ok(new_folder)
        })
    }
}
