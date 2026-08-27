use std::sync::Arc;
use uuid::Uuid;

use crate::dao::classroom_dao::ClassroomDao;
use crate::dao::school_dao::SchoolDao;
use crate::error::error_types::AppError;
use crate::models::classroom::{Classroom, CreateClassroomRequest, UpdateClassroomRequest};
use crate::models::notification::{notification_type, CreateNotification};
use crate::services::NotificationService;

#[derive(Clone)]
pub struct ClassroomService {
    dao: ClassroomDao,
    school_dao: SchoolDao,
    notification_service: Arc<NotificationService>,
}

impl ClassroomService {
    pub fn new(
        dao: ClassroomDao,
        school_dao: SchoolDao,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            dao,
            school_dao,
            notification_service,
        }
    }

    pub async fn create_classroom(&self, request: CreateClassroomRequest) -> Result<Classroom, AppError> {
        if request.class_name.trim().is_empty() {
            return Err(AppError::Validation("Class name cannot be empty".to_string()));
        }

        if self.dao.name_exists_for_school(&request.class_name, &request.school_id).await? {
            return Err(AppError::Conflict(format!(
                "A class named '{}' already exists for this school",
                request.class_name
            )));
        }

        let classroom = self.dao.create_classroom(&request).await?;

        let school_name = self
            .school_dao
            .get_school_name(&classroom.school_id)
            .await
            .unwrap_or_default();
        let body = if school_name.is_empty() {
            format!("Classroom \"{}\" has been added.", classroom.name)
        } else {
            format!("Classroom \"{}\" has been added to {}.", classroom.name, school_name)
        };

        self.notification_service.notify_school_admins(
            CreateNotification {
                school_id: classroom.school_id,
                notification_type: notification_type::CLASSROOM_ADDED.to_string(),
                title: "New Classroom Added".to_string(),
                body,
                related_entity_id: Some(classroom.id),
                related_entity_type: Some("classroom".to_string()),
                action_url: Some("/admin/classrooms".to_string()),
            },
            None,
        ).await;

        Ok(classroom)
    }

    pub async fn get_classrooms_by_school(&self, school_id: Uuid) -> Result<Vec<Classroom>, AppError> {
        self.dao.get_classrooms_by_school(&school_id).await
    }

    pub async fn update_classroom(&self, request: UpdateClassroomRequest) -> Result<Classroom, AppError> {
        if request.class_name.trim().is_empty() {
            return Err(AppError::Validation("Class name cannot be empty".to_string()));
        }

        self.dao.update_classroom(&request).await
    }

    pub async fn delete_classroom(&self, classroom_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        if self.dao.has_enrollments(&classroom_id).await? {
            return Err(AppError::Conflict(
                "Cannot delete classroom: it has existing enrollments".to_string(),
            ));
        }

        // Fetch the name BEFORE the delete so the notification body can show it.
        let classroom_name = self
            .dao
            .get_classroom_name(&classroom_id)
            .await
            .ok()
            .flatten();

        self.dao.delete_classroom(&classroom_id, &school_id).await?;

        let body = match classroom_name {
            Some(name) if !name.is_empty() => format!("Classroom \"{}\" has been deleted.", name),
            _ => "A classroom has been deleted.".to_string(),
        };

        self.notification_service.notify_school_admins(
            CreateNotification {
                school_id,
                notification_type: notification_type::CLASSROOM_DELETED.to_string(),
                title: "Classroom Deleted".to_string(),
                body,
                related_entity_id: Some(classroom_id),
                related_entity_type: Some("classroom".to_string()),
                action_url: Some("/admin/classrooms".to_string()),
            },
            None,
        ).await;

        Ok(())
    }
}
