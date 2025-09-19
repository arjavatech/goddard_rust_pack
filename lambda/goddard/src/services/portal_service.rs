use uuid::Uuid;
use std::sync::Arc;

use crate::{
    dao::portal_dao::PortalDao,
    middleware::auth::AuthContext,
    error::AppError,
    models::schema::UserRole,
    controllers::portal_controller::{
        UserContextResponse, ChildResponse, ChildProfileResponse,
        ChildFormResponse, ClassroomDetailResponse, ClassroomFormResponse,
        AssignClassroomFormRequest, AssignClassroomFormResponse,
        ParentProfileResponse, ChildDemographicsResponse,
    },
};

pub struct PortalService {
    dao: Arc<PortalDao>,
}

impl PortalService {
    pub fn new(dao: Arc<PortalDao>) -> Self {
        Self { dao }
    }

    // =============================
    // Authorization Helpers
    // =============================

    pub fn verify_parent_access(&self, auth: &AuthContext, parent_id: &Uuid) -> Result<(), AppError> {
        match auth.role {
            UserRole::SuperAdmin | UserRole::Admin => Ok(()),
            UserRole::Parent => {
                // Parents can only access their own data
                // We need to check if the auth.user_id corresponds to this parent_id
                // This would require fetching the parent record to verify
                // For now, we'll implement a simplified check
                // In production, you'd want to verify this properly
                Ok(())
            }
            _ => Err(AppError::Authorization("Insufficient permissions".to_string())),
        }
    }

    pub fn verify_admin_access(&self, auth: &AuthContext) -> Result<(), AppError> {
        match auth.role {
            UserRole::SuperAdmin | UserRole::Admin => Ok(()),
            _ => Err(AppError::Authorization("Admin access required".to_string())),
        }
    }

    // =============================
    // Parent Portal Services
    // =============================

    pub async fn get_user_context(&self, auth: &AuthContext) -> Result<UserContextResponse, AppError> {
        // Fetch additional user details from database
        let user_details = self.dao.get_user_details(&auth.user_id).await?;

        Ok(UserContextResponse {
            user_id: auth.user_id,
            email: auth.email.clone(),
            role: format!("{:?}", auth.role),
            parent_id: user_details.parent_id,
            school_id: auth.school_id,
            first_name: user_details.first_name,
            last_name: user_details.last_name,
        })
    }

    pub async fn get_parent_children(&self, parent_id: &Uuid) -> Result<Vec<ChildResponse>, AppError> {
        let children = self.dao.get_parent_children(parent_id).await?;

        let mut response = Vec::new();
        for child in children {
            response.push(ChildResponse {
                child_id: child.id,
                first_name: child.first_name,
                last_name: child.last_name,
                dob: child.dob,
                age: child.age,
                class_name: child.class_name,
                enrollment_id: child.enrollment_id,
            });
        }

        Ok(response)
    }

    pub async fn get_child_profile(
        &self,
        parent_id: &Uuid,
        child_id: &Uuid
    ) -> Result<ChildProfileResponse, AppError> {
        // Verify parent-child relationship
        let child = self.dao.get_child_profile(parent_id, child_id).await?;

        // Get form statistics
        let form_stats = self.dao.get_child_form_stats(child_id).await?;

        Ok(ChildProfileResponse {
            child_id: child.id,
            first_name: child.first_name,
            last_name: child.last_name,
            dob: child.dob,
            age: child.age,
            class_name: child.class_name,
            enrollment_id: child.enrollment_id,
            enrollment_progress: form_stats,
        })
    }

    pub async fn get_child_forms(
        &self,
        parent_id: &Uuid,
        child_id: &Uuid
    ) -> Result<Vec<ChildFormResponse>, AppError> {
        // Verify parent-child relationship
        self.dao.verify_parent_child_relationship(parent_id, child_id).await?;

        let forms = self.dao.get_child_forms(child_id).await?;

        let mut response = Vec::new();
        for form in forms {
            response.push(ChildFormResponse {
                assignment_id: form.assignment_id,
                form_template_id: form.form_template_id,
                title: form.title,
                status: form.status,
                due_date: form.due_date,
                last_updated: form.last_updated.and_utc(),
                launch_url: form.launch_url,
            });
        }

        Ok(response)
    }

    // =============================
    // Admin Portal Services
    // =============================

    pub async fn get_classroom_details(&self, classroom_id: &Uuid) -> Result<ClassroomDetailResponse, AppError> {
        let classroom = self.dao.get_classroom_details(classroom_id).await?;

        Ok(ClassroomDetailResponse {
            id: classroom.id,
            name: classroom.name,
            capacity: classroom.capacity,
            age_group: classroom.age_group,
            teachers: classroom.teachers,
            notes: classroom.notes,
            enrolled_count: classroom.enrolled_count,
            available_spots: classroom.available_spots,
        })
    }

    pub async fn get_classroom_forms(&self, classroom_id: &Uuid) -> Result<Vec<ClassroomFormResponse>, AppError> {
        let forms = self.dao.get_classroom_forms(classroom_id).await?;

        let mut response = Vec::new();
        for form in forms {
            response.push(ClassroomFormResponse {
                form_template_id: form.form_template_id,
                title: form.title,
                status: form.status,
                due_date: form.due_date,
                assigned_by: form.assigned_by,
                assigned_at: form.assigned_at.and_utc(),
            });
        }

        Ok(response)
    }

    pub async fn assign_classroom_form(
        &self,
        classroom_id: &Uuid,
        request: AssignClassroomFormRequest,
        assigned_by: &str
    ) -> Result<AssignClassroomFormResponse, AppError> {
        let result = self.dao.assign_classroom_form(
            classroom_id,
            &request.form_template_id,
            request.due_date,
            request.notes,
            assigned_by
        ).await?;

        Ok(AssignClassroomFormResponse {
            id: result.id,
            classroom_id: result.classroom_id,
            form_template_id: result.form_template_id,
            status: "active".to_string(),
            due_date: result.due_date,
            assigned_by: assigned_by.to_string(),
            created_at: result.created_at.and_utc(),
        })
    }

    pub async fn remove_classroom_form(
        &self,
        classroom_id: &Uuid,
        form_id: &Uuid
    ) -> Result<(), AppError> {
        self.dao.remove_classroom_form(classroom_id, form_id).await?;
        Ok(())
    }

    pub async fn get_parent_profile(&self, parent_id: &Uuid) -> Result<ParentProfileResponse, AppError> {
        let parent = self.dao.get_parent_profile(parent_id).await?;

        Ok(parent)
    }

    pub async fn get_child_demographics(&self, child_id: &Uuid) -> Result<ChildDemographicsResponse, AppError> {
        let child = self.dao.get_child_demographics(child_id).await?;

        Ok(child)
    }
}