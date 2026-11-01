use std::sync::Arc;
use uuid::Uuid;

use crate::dao::admin_permission_dao::AdminPermissionDao;
use crate::dao::drive_dao::DriveDao;
use crate::dao::school_dao::SchoolDao;
use crate::error::{ApiResult, AppError};
use crate::middleware::auth::AuthContext;
use crate::models::admin_permission::MANAGE_DRIVE;
use crate::models::drive::*;
use crate::models::schema::UserRole;
use crate::models::school::SchoolFeature;
use crate::services::upload_service::UploadService;

#[derive(Clone)]
pub struct DriveService {
    dao: DriveDao,
    upload: Arc<UploadService>,
    schools: SchoolDao,
    admin_permissions: AdminPermissionDao,
}

impl DriveService {
    pub fn new(
        dao: DriveDao,
        upload: Arc<UploadService>,
        schools: SchoolDao,
        admin_permissions: AdminPermissionDao,
    ) -> Self {
        Self { dao, upload, schools, admin_permissions }
    }

    async fn check_access(&self, auth: &AuthContext) -> ApiResult<()> {
        if !matches!(auth.role, UserRole::SuperAdmin) {
            self.schools.ensure_feature_enabled(auth.school_id, SchoolFeature::Drive).await?;
            self.admin_permissions.ensure_permission(auth, MANAGE_DRIVE).await?;
        }
        Ok(())
    }

    fn resolve_target_user(&self, auth: &AuthContext, requested_user_id: Option<Uuid>) -> Uuid {
        match requested_user_id {
            Some(uid) if matches!(auth.role, UserRole::SuperAdmin) => uid,
            _ => auth.user_id,
        }
    }

    pub async fn get_upload_url(
        &self,
        req: DriveUploadUrlRequest,
        auth: &AuthContext,
    ) -> ApiResult<DriveUploadUrlResponse> {
        self.check_access(auth).await?;

        let target_user_id = self.resolve_target_user(auth, req.user_id);
        let ext = req.file_name.rsplit('.').next().unwrap_or("bin").to_lowercase();
        let s3_key = format!("drive/{}/{}/{}.{}", auth.school_id, target_user_id, Uuid::new_v4(), ext);

        let upload_url = self.upload.create_drive_upload_url(&s3_key, &req.content_type, req.file_size).await?;
        Ok(DriveUploadUrlResponse { upload_url, s3_key })
    }

    pub async fn register_file(
        &self,
        req: DriveRegisterFileRequest,
        auth: &AuthContext,
    ) -> ApiResult<DriveItem> {
        self.check_access(auth).await?;

        let target_user_id = self.resolve_target_user(auth, req.user_id);
        let expected_prefix = format!("drive/{}/{}/", auth.school_id, target_user_id);
        if !req.s3_key.starts_with(&expected_prefix) {
            return Err(AppError::Authorization("Invalid S3 key for this user".into()));
        }

        if let Some(folder_id) = req.folder_id {
            let folder = self.dao.get_item(folder_id, auth.school_id).await?
                .ok_or_else(|| AppError::NotFound("Folder".into()))?;
            if folder.user_id != target_user_id {
                return Err(AppError::Authorization("Folder access denied".into()));
            }
        }

        self.dao.register_file(
            auth.school_id, target_user_id, req.folder_id,
            &req.name, &req.s3_key, &req.mime_type, req.file_size,
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

        let target_user_id = self.resolve_target_user(auth, req.user_id);

        if let Some(folder_id) = req.folder_id {
            let folder = self.dao.get_item(folder_id, auth.school_id).await?
                .ok_or_else(|| AppError::NotFound("Parent folder".into()))?;
            if !matches!(auth.role, UserRole::SuperAdmin) && folder.user_id != target_user_id {
                return Err(AppError::Authorization("Parent folder access denied".into()));
            }
        }

        self.dao.create_folder(auth.school_id, target_user_id, req.folder_id, req.name.trim()).await
    }

    pub async fn list_items(
        &self,
        query: DriveListQuery,
        auth: &AuthContext,
    ) -> ApiResult<Vec<DriveItem>> {
        self.check_access(auth).await?;

        let target_user_id = self.resolve_target_user(auth, query.user_id);
        let sort_by = query.sort_by.as_deref().unwrap_or("name");
        let sort_order = query.sort_order.as_deref().unwrap_or("asc");

        self.dao.list_items(auth.school_id, target_user_id, query.folder_id, sort_by, sort_order).await
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

        if !matches!(auth.role, UserRole::SuperAdmin) && item.user_id != auth.user_id {
            return Err(AppError::Authorization("Access denied".into()));
        }

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

        if !matches!(auth.role, UserRole::SuperAdmin) && item.user_id != auth.user_id {
            return Err(AppError::Authorization("Access denied".into()));
        }

        self.dao.rename_item(id, name.trim()).await
    }

    pub async fn delete_item(&self, id: Uuid, auth: &AuthContext) -> ApiResult<()> {
        self.check_access(auth).await?;

        let item = self.dao.get_item(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("Drive item".into()))?;

        if !matches!(auth.role, UserRole::SuperAdmin) && item.user_id != auth.user_id {
            return Err(AppError::Authorization("Access denied".into()));
        }

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

        if !matches!(auth.role, UserRole::SuperAdmin) && item.user_id != auth.user_id {
            return Err(AppError::Authorization("Access denied".into()));
        }

        if let Some(target_folder_id) = req.folder_id {
            if target_folder_id == id {
                return Err(AppError::Validation("Cannot move item into itself".into()));
            }
            let target = self.dao.get_item(target_folder_id, auth.school_id).await?
                .ok_or_else(|| AppError::NotFound("Target folder".into()))?;
            if target.item_type != "folder" {
                return Err(AppError::Validation("Target must be a folder".into()));
            }
            if !matches!(auth.role, UserRole::SuperAdmin) && target.user_id != auth.user_id {
                return Err(AppError::Authorization("Target folder access denied".into()));
            }
        }

        self.dao.move_item(id, req.folder_id).await
    }

    pub async fn copy_item(&self, id: Uuid, auth: &AuthContext) -> ApiResult<DriveItem> {
        self.check_access(auth).await?;

        let item = self.dao.get_item(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("Drive item".into()))?;

        if !matches!(auth.role, UserRole::SuperAdmin) && item.user_id != auth.user_id {
            return Err(AppError::Authorization("Access denied".into()));
        }

        if item.item_type != "file" {
            return Err(AppError::Validation("Can only copy files, not folders".into()));
        }

        let source_key = self.dao.get_s3_key(id, auth.school_id).await?
            .ok_or_else(|| AppError::NotFound("Source file".into()))?;

        let ext = source_key.rsplit('.').next().unwrap_or("bin");
        let dest_key = format!("drive/{}/{}/{}.{}", auth.school_id, item.user_id, Uuid::new_v4(), ext);

        self.upload.copy_s3_object(&source_key, &dest_key).await?;

        let copy_name = format!("Copy of {}", item.name);
        self.dao.register_file(
            auth.school_id,
            item.user_id,
            item.folder_id,
            &copy_name,
            &dest_key,
            item.mime_type.as_deref().unwrap_or("application/octet-stream"),
            item.file_size.unwrap_or(0),
        ).await
    }

    pub async fn list_admins(&self, auth: &AuthContext) -> ApiResult<Vec<DriveAdminSummary>> {
        if !matches!(auth.role, UserRole::SuperAdmin) {
            return Err(AppError::Authorization("SuperAdmin only".into()));
        }
        self.dao.list_admins_with_drive(auth.school_id).await
    }
}
