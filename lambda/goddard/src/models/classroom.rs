use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Classroom {
    pub id: Uuid,
    pub school_id: Uuid,
    pub name: String,
    pub age_group: Option<String>,
    pub capacity: Option<i32>,
    pub enrolled_count: Option<i32>,
    pub is_active: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateClassroomRequest {
    pub school_id: Uuid,
    pub class_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClassroomRequest {
    pub school_id: Uuid,
    pub class_id: Uuid,
    pub class_name: String,
}

#[derive(Debug, Serialize)]
pub struct ClassroomResponse {
    pub id: Uuid,
    pub school_id: Uuid,
    pub name: String,
    pub age_group: Option<String>,
    pub capacity: Option<i32>,
    pub enrolled_count: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct ClassroomListResponse {
    pub id: Uuid,
    pub class_name: String,
}

impl From<Classroom> for ClassroomResponse {
    fn from(classroom: Classroom) -> Self {
        Self {
            id: classroom.id,
            school_id: classroom.school_id,
            name: classroom.name,
            age_group: classroom.age_group,
            capacity: classroom.capacity,
            enrolled_count: classroom.enrolled_count,
            created_at: classroom.created_at,
            updated_at: classroom.updated_at,
        }
    }
}

impl From<Classroom> for ClassroomListResponse {
    fn from(classroom: Classroom) -> Self {
        Self {
            id: classroom.id,
            class_name: classroom.name,
        }
    }
}