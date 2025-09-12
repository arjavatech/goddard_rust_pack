use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user: Option<User>,
    pub users: Option<Vec<User>>,
    pub message: String,
    pub success: bool,
}

impl User {
    pub fn new(email: String, name: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            name,
            created_at: now.clone(),
            updated_at: now,
            is_active: true,
        }
    }

    pub fn update(&mut self, req: UpdateUserRequest) {
        if let Some(name) = req.name {
            self.name = name;
        }
        if let Some(is_active) = req.is_active {
            self.is_active = is_active;
        }
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}