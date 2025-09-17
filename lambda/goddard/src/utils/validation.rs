use uuid::Uuid;
use crate::error::AppError;

pub struct ValidationUtils;

impl ValidationUtils {
    pub fn validate_uuid(uuid_str: &str) -> Result<Uuid, AppError> {
        Uuid::parse_str(uuid_str)
            .map_err(|_| AppError::Validation(format!("Invalid UUID format: {}", uuid_str)))
    }

    pub fn validate_email(email: &str) -> Result<(), AppError> {
        // Simple email validation
        if email.contains('@') && email.contains('.') && email.len() > 5 {
            Ok(())
        } else {
            Err(AppError::Validation("Invalid email format".to_string()))
        }
    }

    pub fn validate_role(role: &str) -> Result<(), AppError> {
        match role {
            "SuperAdmin" | "Admin" | "Teacher" | "Parent" | "primary-parent" | "secondary-parent" => Ok(()),
            _ => Err(AppError::Validation("Invalid role".to_string())),
        }
    }

    pub fn validate_school_access(
        jwt_school_id: Option<&str>,
        requested_school_id: &str,
        user_role: &str,
    ) -> Result<(), AppError> {
        if user_role == "SuperAdmin" {
            return Ok(());
        }

        match jwt_school_id {
            Some(school_id) if school_id == requested_school_id => Ok(()),
            _ => Err(AppError::Authorization("School access denied".to_string())),
        }
    }
}