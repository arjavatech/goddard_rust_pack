use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::{Client, Error as DynamoError};
use aws_sdk_dynamodb::types::AttributeValue;
use std::collections::HashMap;
use std::env;
use tracing::{info, error};

use crate::models::User;

#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error("DynamoDB error: {0}")]
    DynamoDb(#[from] DynamoError),
    #[error("Item not found")]
    NotFound,
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Configuration error: {0}")]
    Config(String),
}

pub struct DynamoDbService {
    client: Client,
    table_name: String,
}

impl DynamoDbService {
    pub async fn new() -> Result<Self, DatabaseError> {
        let table_name = env::var("TABLE_NAME")
            .map_err(|_| DatabaseError::Config("TABLE_NAME environment variable not set".to_string()))?;

        info!("Initializing DynamoDB client for table: {}", table_name);
        
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = Client::new(&config);

        Ok(Self {
            client,
            table_name,
        })
    }

    pub async fn create_user(&self, user: &User) -> Result<(), DatabaseError> {
        let mut item = HashMap::new();
        item.insert("id".to_string(), AttributeValue::S(user.id.clone()));
        item.insert("email".to_string(), AttributeValue::S(user.email.clone()));
        item.insert("name".to_string(), AttributeValue::S(user.name.clone()));
        item.insert("created_at".to_string(), AttributeValue::S(user.created_at.clone()));
        item.insert("updated_at".to_string(), AttributeValue::S(user.updated_at.clone()));
        item.insert("is_active".to_string(), AttributeValue::Bool(user.is_active));

        let result = self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(id)")
            .send()
            .await;

        match result {
            Ok(_) => {
                info!("Successfully created user with id: {}", user.id);
                Ok(())
            },
            Err(e) => {
                error!("Failed to create user: {}", e);
                Err(DatabaseError::DynamoDb(e.into()))
            }
        }
    }

    pub async fn get_user(&self, id: &str) -> Result<Option<User>, DatabaseError> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .send()
            .await
            .map_err(DatabaseError::DynamoDb)?;

        match result.item {
            Some(item) => {
                let user = self.item_to_user(item)?;
                info!("Retrieved user: {}", id);
                Ok(Some(user))
            },
            None => {
                info!("User not found: {}", id);
                Ok(None)
            }
        }
    }

    pub async fn update_user(&self, user: &User) -> Result<(), DatabaseError> {
        let mut item = HashMap::new();
        item.insert("id".to_string(), AttributeValue::S(user.id.clone()));
        item.insert("email".to_string(), AttributeValue::S(user.email.clone()));
        item.insert("name".to_string(), AttributeValue::S(user.name.clone()));
        item.insert("created_at".to_string(), AttributeValue::S(user.created_at.clone()));
        item.insert("updated_at".to_string(), AttributeValue::S(user.updated_at.clone()));
        item.insert("is_active".to_string(), AttributeValue::Bool(user.is_active));

        let result = self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_exists(id)")
            .send()
            .await;

        match result {
            Ok(_) => {
                info!("Successfully updated user with id: {}", user.id);
                Ok(())
            },
            Err(e) => {
                error!("Failed to update user: {}", e);
                Err(DatabaseError::DynamoDb(e.into()))
            }
        }
    }

    pub async fn delete_user(&self, id: &str) -> Result<(), DatabaseError> {
        let result = self
            .client
            .delete_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .condition_expression("attribute_exists(id)")
            .send()
            .await;

        match result {
            Ok(_) => {
                info!("Successfully deleted user with id: {}", id);
                Ok(())
            },
            Err(e) => {
                error!("Failed to delete user: {}", e);
                Err(DatabaseError::DynamoDb(e.into()))
            }
        }
    }

    pub async fn list_users(&self, limit: Option<i32>) -> Result<Vec<User>, DatabaseError> {
        let mut scan = self
            .client
            .scan()
            .table_name(&self.table_name);

        if let Some(limit) = limit {
            scan = scan.limit(limit);
        }

        let result = scan.send().await.map_err(DatabaseError::DynamoDb)?;

        let users = result
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|item| self.item_to_user(item))
            .collect::<Result<Vec<_>, _>>()?;

        info!("Retrieved {} users", users.len());
        Ok(users)
    }

    fn item_to_user(&self, item: HashMap<String, AttributeValue>) -> Result<User, DatabaseError> {
        let get_string = |key: &str| -> Result<String, DatabaseError> {
            item.get(key)
                .and_then(|v| v.as_s().ok())
                .map(|s| s.clone())
                .ok_or_else(|| DatabaseError::Serialization(format!("Missing or invalid {}", key)))
        };

        let get_bool = |key: &str| -> Result<bool, DatabaseError> {
            item.get(key)
                .and_then(|v| v.as_bool().ok())
                .cloned()
                .ok_or_else(|| DatabaseError::Serialization(format!("Missing or invalid {}", key)))
        };

        Ok(User {
            id: get_string("id")?,
            email: get_string("email")?,
            name: get_string("name")?,
            created_at: get_string("created_at")?,
            updated_at: get_string("updated_at")?,
            is_active: get_bool("is_active")?,
        })
    }
}