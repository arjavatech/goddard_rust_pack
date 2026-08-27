use std::sync::Arc;
use uuid::Uuid;
use crate::{dao::DocumentRequestDao, error::{ApiResult, AppError}, models::{document_request::*, email::BulkEmailResponse, notification::CreateNotification}, services::{EmailService, NotificationService, UploadService}};

#[derive(Clone)]
pub struct DocumentRequestService { dao: DocumentRequestDao, uploads: Arc<UploadService>, notifications: Arc<NotificationService>, email: Arc<EmailService> }
impl DocumentRequestService {
    pub fn new(dao: DocumentRequestDao, uploads: Arc<UploadService>, notifications: Arc<NotificationService>, email: Arc<EmailService>) -> Self { Self { dao, uploads, notifications, email } }
    pub async fn create(&self, request: CreateDocumentRequest, actor: Uuid) -> ApiResult<DocumentRequestSummary> {
        if !matches!(request.audience.as_str(), "student" | "employee") || !matches!(request.target.as_str(), "all" | "selected") { return Err(AppError::Validation("audience must be student/employee and target must be all/selected".into())); }
        if request.document_name.trim().is_empty() { return Err(AppError::Validation("Document name is required".into())); }
        if request.target == "selected" {
            let ids = if request.audience == "student" { request.child_ids.as_deref().unwrap_or(&[]) } else { request.employee_ids.as_deref().unwrap_or(&[]) };
            if ids.is_empty() { return Err(AppError::Validation("Select at least one recipient".into())); }
        }
        let summary = self.dao.create_request(&request, actor).await?;
        if request.target == "selected" {
            let ids = if request.audience == "student" { request.child_ids.as_deref().unwrap_or(&[]) } else { request.employee_ids.as_deref().unwrap_or(&[]) };
            self.dao.create_selected_assignments(summary.id, &request.audience, ids).await?;
        }
        self.dao.get_request_summary(summary.id).await
    }
    pub async fn publish(&self, request_id: Uuid, actor: Uuid) -> ApiResult<DocumentRequestSummary> {
        let summary=self.dao.publish_request(request_id, actor).await?;
        for user_id in self.dao.recipient_users(request_id,None).await? {
            self.notifications.notify_user(user_id,CreateNotification{school_id:summary.school_id,notification_type:"document_requested".into(),title:"New document requested".into(),body:format!("{} is required.",summary.document_name),related_entity_id:Some(request_id),related_entity_type:Some("document_request".into()),action_url:Some(if summary.audience=="student" {"/dashboard/documents".into()} else {"/employee/documents".into()})}).await;
        }
        Ok(summary)
    }
    pub async fn list(&self, q:&DocumentRequestQuery)->ApiResult<Vec<DocumentRequestSummary>>{self.dao.list_requests(q).await}
    pub async fn recipients(&self, school_id:Uuid, audience:&str)->ApiResult<Vec<DocumentRecipient>>{self.dao.recipients(school_id,audience).await}
    pub async fn request_summary(&self, id:Uuid)->ApiResult<DocumentRequestSummary>{self.dao.get_request_summary(id).await}
    pub async fn assignment_school(&self, assignment_id:Uuid, user:Uuid, admin:bool)->ApiResult<Uuid>{Ok(self.dao.assignment_access(assignment_id,user,admin).await?.0)}
    pub async fn submission_assignment(&self, submission_id:Uuid)->ApiResult<Uuid>{self.dao.submission_assignment(submission_id).await}
    pub async fn assignments(&self,q:&DocumentAssignmentQuery,user:Option<Uuid>,review:bool)->ApiResult<PagedDocumentAssignments>{self.dao.list_assignments(q,user,review).await}
    pub async fn send_reminders(&self, request: DocumentReminderRequest, actor: Uuid) -> ApiResult<BulkEmailResponse> {
        if request.assignment_ids.is_empty() { return Err(AppError::Validation("Select at least one document assignment".into())); }
        if request.assignment_ids.len() > 300 { return Err(AppError::Validation("A maximum of 300 document assignments can be reminded at once".into())); }
        let reminders = self.dao.reminders_for_assignments(request.school_id, &request.assignment_ids).await?;
        if reminders.is_empty() { return Err(AppError::Validation("No selected documents are eligible for a reminder".into())); }
        let eligible_ids = reminders.iter().map(|reminder| reminder.assignment_id).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>();
        let response = self.email.send_bulk_document_reminders(reminders).await?;
        self.dao.record_reminders(&eligible_ids, actor).await?;
        Ok(response)
    }
    pub async fn upload_intent(&self, assignment_id:Uuid, user:Uuid, admin:bool, data:&UploadIntentRequest)->ApiResult<UploadIntentResponse>{
        let (school_id,_,request_status)=self.dao.assignment_access(assignment_id,user,admin).await?;
        if request_status != "active" { return Err(AppError::Validation("This document request is closed".into())); }
        let assignment_status = self.dao.assignment_status(assignment_id).await?;
        if !matches!(assignment_status.as_str(), "pending" | "submitted" | "rejected") {
            return Err(AppError::Validation(if assignment_status == "approved" { "Approved documents are locked and cannot be replaced".into() } else { "This document is not currently available for upload".into() }));
        }
        if !crate::services::upload_service::DOCUMENT_ALLOWED_CONTENT_TYPES.contains(&data.content_type.as_str()) || data.file_size_bytes <= 0 || data.file_size_bytes > crate::services::upload_service::DOCUMENT_MAX_SIZE_BYTES { return Err(AppError::Validation("Upload a PDF, JPG/JPEG, or PNG no larger than 10 MB".into())); }
        let extension=match data.content_type.as_str(){"application/pdf"=>"pdf","image/jpeg"=>"jpg","image/png"=>"png",_=>"bin"};
        let key=format!("private/schools/{}/document-assignments/{}/{}.{}",school_id,assignment_id,Uuid::new_v4(),extension);
        let upload_url=self.uploads.create_document_upload_url(&key,&data.content_type,data.file_size_bytes).await?;
        Ok(UploadIntentResponse{storage_key:key,upload_url,expires_in_seconds:300})
    }
    pub async fn complete_upload(&self, assignment_id:Uuid,user:Uuid,admin:bool,data:&CompleteUploadRequest)->ApiResult<DocumentAssignmentItem>{
        let (school_id,_,request_status)=self.dao.assignment_access(assignment_id,user,admin).await?;
        if request_status != "active" || !data.storage_key.starts_with(&format!("private/schools/{}/document-assignments/{}/",school_id,assignment_id)) { return Err(AppError::Validation("Invalid document upload".into())); }
        self.uploads.verify_document_object(&data.storage_key,&data.content_type,data.file_size_bytes).await?;
        let item=self.dao.complete_upload(assignment_id,user,data).await?;
        self.notifications.notify_school_admins(CreateNotification{school_id,notification_type:"document_submitted".into(),title:"Document ready for review".into(),body:format!("{} was submitted for {}.",item.subject_name,item.document_name),related_entity_id:Some(assignment_id),related_entity_type:Some("document_assignment".into()),action_url:Some("/admin/documents/review".into())}, None).await;
        Ok(item)
    }
    pub async fn review(&self,assignment_id:Uuid,actor:Uuid,data:&ReviewDocumentAssignmentRequest)->ApiResult<DocumentAssignmentItem>{
        if !matches!(data.status.as_str(),"approved"|"rejected") || (data.status=="rejected" && data.reason.as_deref().unwrap_or("").trim().is_empty()) { return Err(AppError::Validation("Use approved or rejected; rejection requires a reason".into())); }
        let item=self.dao.review(assignment_id,actor,data).await?;
        for user_id in self.dao.recipient_users(item.request_id,Some(assignment_id)).await? {
            let rejected=data.status=="rejected";
            self.notifications.notify_user(user_id,CreateNotification{school_id:item.school_id,notification_type:if rejected {"document_rejected".into()} else {"document_approved".into()},title:if rejected {"Document re-upload required".into()} else {"Document approved".into()},body:if rejected {format!("{} needs changes: {}",item.document_name,data.reason.as_deref().unwrap_or("Please upload a corrected document."))} else {format!("{} has been approved.",item.document_name)},related_entity_id:Some(assignment_id),related_entity_type:Some("document_assignment".into()),action_url:Some(if item.audience=="student" {"/dashboard/documents".into()} else {"/employee/documents".into()})}).await;
        }
        Ok(item)
    }
    pub async fn file_url(&self,submission_id:Uuid,user:Uuid,admin:bool,download:bool)->ApiResult<FileAccessResponse>{let assignment=self.dao.submission_assignment(submission_id).await?;self.dao.assignment_access(assignment,user,admin).await?;let(_,key)=self.dao.file_key(submission_id).await?;Ok(FileAccessResponse{url:self.uploads.create_document_access_url(&key,download).await?,expires_in_seconds:300})}
    pub async fn history(&self,assignment_id:Uuid,user:Uuid,admin:bool)->ApiResult<Vec<DocumentAuditEvent>>{self.dao.assignment_access(assignment_id,user,admin).await?;self.dao.history(assignment_id).await}
}
