use tracing::{info, error, warn};
use crate::models::{User, CreateUserRequest, UpdateUserRequest, UserResponse};
use crate::services::{DynamoDbService, DatabaseError};

#[derive(thiserror::Error, Debug)]
pub enum UserServiceError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("User not found with id: {0}")]
    NotFound(String),
    #[error("User already exists with email: {0}")]
    AlreadyExists(String),
    #[error("Validation error: {0}")]
    Validation(String),
}

pub struct UserService {
    db: DynamoDbService,
}

impl UserService {
    pub async fn new() -> Result<Self, UserServiceError> {
        let db = DynamoDbService::new().await?;
        Ok(Self { db })
    }

    pub async fn create_user(&self, req: CreateUserRequest) -> Result<UserResponse, UserServiceError> {
        info!("Creating user with email: {}", req.email);

        // Basic validation
        if req.email.is_empty() || !req.email.contains('@') {
            warn!("Invalid email provided: {}", req.email);
            return Err(UserServiceError::Validation("Invalid email format".to_string()));
        }

        if req.name.is_empty() {
            warn!("Empty name provided");
            return Err(UserServiceError::Validation("Name cannot be empty".to_string()));
        }

        let user = User::new(req.email, req.name);
        
        match self.db.create_user(&user).await {
            Ok(_) => {
                info!("Successfully created user with id: {}", user.id);
                Ok(UserResponse {
                    user: Some(user),
                    users: None,
                    message: "User created successfully".to_string(),
                    success: true,
                })
            },
            Err(e) => {
                error!("Failed to create user: {}", e);
                Err(UserServiceError::Database(e))
            }
        }
    }

    pub async fn get_user(&self, id: &str) -> Result<UserResponse, UserServiceError> {
        info!("Getting user with id: {}", id);

        match self.db.get_user(id).await? {
            Some(user) => {
                info!("Found user with id: {}", id);
                Ok(UserResponse {
                    user: Some(user),
                    users: None,
                    message: "User retrieved successfully".to_string(),
                    success: true,
                })
            },
            None => {
                warn!("User not found with id: {}", id);
                Err(UserServiceError::NotFound(id.to_string()))
            }
        }
    }

    pub async fn update_user(&self, id: &str, req: UpdateUserRequest) -> Result<UserResponse, UserServiceError> {
        info!("Updating user with id: {}", id);

        // Get existing user
        let mut user = match self.db.get_user(id).await? {
            Some(user) => user,
            None => {
                warn!("User not found for update: {}", id);
                return Err(UserServiceError::NotFound(id.to_string()));
            }
        };

        // Validate update request
        if let Some(ref name) = req.name {
            if name.is_empty() {
                warn!("Empty name provided in update");
                return Err(UserServiceError::Validation("Name cannot be empty".to_string()));
            }
        }

        // Update user
        user.update(req);

        match self.db.update_user(&user).await {
            Ok(_) => {
                info!("Successfully updated user with id: {}", id);
                Ok(UserResponse {
                    user: Some(user),
                    users: None,
                    message: "User updated successfully".to_string(),
                    success: true,
                })
            },
            Err(e) => {
                error!("Failed to update user: {}", e);
                Err(UserServiceError::Database(e))
            }
        }
    }

    pub async fn delete_user(&self, id: &str) -> Result<UserResponse, UserServiceError> {
        info!("Deleting user with id: {}", id);

        // Check if user exists
        if self.db.get_user(id).await?.is_none() {
            warn!("User not found for deletion: {}", id);
            return Err(UserServiceError::NotFound(id.to_string()));
        }

        match self.db.delete_user(id).await {
            Ok(_) => {
                info!("Successfully deleted user with id: {}", id);
                Ok(UserResponse {
                    user: None,
                    users: None,
                    message: "User deleted successfully".to_string(),
                    success: true,
                })
            },
            Err(e) => {
                error!("Failed to delete user: {}", e);
                Err(UserServiceError::Database(e))
            }
        }
    }

    pub async fn list_users(&self, limit: Option<i32>) -> Result<UserResponse, UserServiceError> {
        info!("Listing users with limit: {:?}", limit);

        match self.db.list_users(limit).await {
            Ok(users) => {
                info!("Successfully retrieved {} users", users.len());
                Ok(UserResponse {
                    user: None,
                    users: Some(users),
                    message: "Users retrieved successfully".to_string(),
                    success: true,
                })
            },
            Err(e) => {
                error!("Failed to list users: {}", e);
                Err(UserServiceError::Database(e))
            }
        }
    }
}