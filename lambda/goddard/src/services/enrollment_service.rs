use uuid::Uuid;
use std::collections::HashMap;

use crate::dao::enrollment_dao::EnrollmentDao;
use crate::services::supabase_client::SupabaseClient;
use crate::models::enrollment::{
    ParentInviteRequest, ParentInviteResponse, ParentInviteDetails,
    ParentDetails, ChildDetails, EnrollmentDetails, AssignedFormDetails,
    AuthUserResult, FormTemplate, ClassFormOverride, CreatedFormAssignment
};
use crate::error::AppError;

type ApiResult<T> = Result<T, AppError>;

pub struct EnrollmentService {
    enrollment_dao: EnrollmentDao,
    supabase_client: SupabaseClient,
}

impl EnrollmentService {
    pub fn new(enrollment_dao: EnrollmentDao, supabase_client: SupabaseClient) -> Self {
        Self {
            enrollment_dao,
            supabase_client,
        }
    }

    pub async fn create_parent_invite(&self, request: ParentInviteRequest) -> ApiResult<ParentInviteResponse> {
        // Step 1: Validate classroom belongs to school
        if !self.enrollment_dao.verify_classroom_belongs_to_school(request.class_id, request.school_id).await? {
            return Err(AppError::Validation("Classroom does not belong to the specified school".to_string()));
        }

        // Step 2: Check if parent email already exists for this school
        if self.enrollment_dao.check_email_exists(&request.parent_email, request.school_id).await? {
            return Err(AppError::Conflict("Parent email already exists for this school".to_string()));
        }

        // Step 3: Create auth user via Supabase
        let auth_result = self.create_auth_user(&request.parent_email).await?;

        // Step 4: Create parent user in users table
        let created_user = self.enrollment_dao.create_user(
            auth_result.auth_user_id,
            request.school_id,
            &request.parent_first_name,
            &request.parent_last_name,
            &request.parent_email,
            "Parent",
        ).await?;

        // Step 5: Create child in children table
        let created_child = self.enrollment_dao.create_child(
            created_user.id,
            request.school_id,
            &request.child_first_name,
            &request.child_last_name,
            request.child_birth_date,
            &request.gender,
        ).await?;

        // Step 6: Create enrollment
        let created_enrollment = self.enrollment_dao.create_enrollment(
            created_child.id,
            request.school_id,
            request.class_id,
        ).await?;

        // Step 7: Get school default forms
        let school_forms = self.enrollment_dao.get_school_default_forms(request.school_id).await?;

        // Step 8: Get classroom form overrides
        let classroom_overrides = self.enrollment_dao.get_classroom_form_overrides(
            request.class_id,
            request.school_id,
        ).await?;

        // Step 9: Process forms and create assignments
        let assigned_forms = self.process_form_assignments(
            &school_forms,
            &classroom_overrides,
            created_enrollment.id,
            created_child.id,
            request.school_id,
        ).await?;

        // Step 10: Generate response
        let response = ParentInviteResponse {
            parent_id: created_user.id,
            child_id: created_child.id,
            enrollment_id: created_enrollment.id,
            assigned_forms_count: assigned_forms.len(),
            invite_id: auth_result.auth_user_id, // Using auth_user_id as invite_id for now
            signup_email_sent: true,
            message: "Parent invite created successfully and signup email sent".to_string(),
            details: ParentInviteDetails {
                parent: ParentDetails {
                    id: created_user.id,
                    school_id: created_user.school_id,
                    first_name: created_user.first_name,
                    last_name: created_user.last_name,
                    email: created_user.email,
                    role: created_user.role,
                    is_verified: created_user.is_verified,
                    created_at: created_user.created_at,
                },
                child: ChildDetails {
                    id: created_child.id,
                    parent_id: created_child.parent_id,
                    school_id: created_child.school_id,
                    first_name: created_child.first_name,
                    last_name: created_child.last_name,
                    birth_date: created_child.birth_date,
                    gender: created_child.gender,
                    status: created_child.status,
                    created_at: created_child.created_at,
                },
                enrollment: EnrollmentDetails {
                    id: created_enrollment.id,
                    child_id: created_enrollment.child_id,
                    school_id: created_enrollment.school_id,
                    classroom_id: created_enrollment.classroom_id,
                    status: created_enrollment.status,
                    application_status: created_enrollment.application_status,
                    created_at: created_enrollment.created_at,
                },
                assigned_forms: assigned_forms.into_iter().map(|form| AssignedFormDetails {
                    id: form.id,
                    form_template_id: form.form_template_id,
                    form_name: form.form_name,
                    assignment_source: form.assignment_source,
                    status: form.status,
                    is_required: form.is_required,
                }).collect(),
            },
        };

        Ok(response)
    }

    // Step 3: Create auth user via Supabase
    async fn create_auth_user(&self, email: &str) -> ApiResult<AuthUserResult> {
        // Call Supabase to create auth user
        let auth_user_id = self.supabase_client.create_auth_user(email).await?;

        Ok(AuthUserResult {
            auth_user_id,
            email: email.to_string(),
        })
    }

    // Step 9: Process form assignments logic
    async fn process_form_assignments(
        &self,
        school_forms: &[FormTemplate],
        classroom_overrides: &[ClassFormOverride],
        enrollment_id: Uuid,
        child_id: Uuid,
        school_id: Uuid,
    ) -> ApiResult<Vec<CreatedFormAssignment>> {
        let mut final_forms = HashMap::new();

        // Start with school default forms
        for form in school_forms {
            final_forms.insert(form.id, (form.clone(), "school_default".to_string()));
        }

        // Apply classroom overrides
        for override_form in classroom_overrides {
            match override_form.action.as_deref() {
                Some("add") => {
                    let form_template = FormTemplate {
                        id: override_form.form_template_id,
                        form_name: override_form.form_name.clone(),
                        is_required: override_form.is_required,
                    };
                    final_forms.insert(override_form.form_template_id, (form_template, "class_override".to_string()));
                }
                Some("remove") => {
                    final_forms.remove(&override_form.form_template_id);
                }
                _ => {
                    // Handle other actions (including None) if needed
                    continue;
                }
            }
        }

        // Create form assignments for all final forms
        let mut assigned_forms = Vec::new();
        for (form_id, (form_template, assignment_source)) in final_forms {
            let assignment = self.enrollment_dao.create_student_form_assignment(
                enrollment_id,
                child_id,
                school_id,
                form_id,
                &assignment_source,
                form_template.is_required,
            ).await?;

            assigned_forms.push(assignment);
        }

        Ok(assigned_forms)
    }
}