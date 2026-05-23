use uuid::Uuid;
use crate::dao::classroom_dao::ClassroomDao;
use crate::models::classroom::{Classroom, CreateClassroomRequest, UpdateClassroomRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct ClassroomService {
    dao: ClassroomDao,
}

impl ClassroomService {
    pub fn new(dao: ClassroomDao) -> Self {
        Self { dao }
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

        self.dao.create_classroom(&request).await
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

        self.dao.delete_classroom(&classroom_id, &school_id).await
    }
}