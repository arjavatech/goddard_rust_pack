use uuid::Uuid;
use std::collections::HashMap;

use crate::dao::enrollment_dao::EnrollmentDao;
use crate::services::supabase_client::SupabaseClient;
use crate::models::enrollment::{
    ParentInviteRequest, ParentInviteResponse, ParentInviteDetails,
    ParentDetails, ChildDetails, EnrollmentDetails, AssignedFormDetails,
    AuthUserResult, FormTemplate, ClassFormOverride, CreatedFormAssignment,
    ResendConfirmationRequest, ResendConfirmationResponse, ResendConfirmationParentDetails,
    AddChildRequest, AddChildResponse, AddChildDetails, AddChildParentDetails,
    GetParentDetailsBySchoolRequest, GetParentDetailsBySchoolResponse, ParentWithAuthDetails,
    GetEnrollmentChildrenRequest, GetEnrollmentChildrenResponse, EnrollmentChildWithForms,
    GetClassWiseCountRequest, GetClassWiseCountResponse,
    GetSchoolFormsRequest, GetSchoolFormsResponse
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

        // Step 2: Create auth user via Supabase
        let auth_result = self.create_auth_user(
            &request.parent_email,
            request.school_id,
            &request.parent_first_name,
            &request.parent_last_name,
            "Parent"
        ).await?;

        // Step 3: Get or create parent (may have been created by DB trigger)
        let created_parent = match self.enrollment_dao.get_parent_by_id(auth_result.auth_user_id, request.school_id).await {
            Ok(existing_parent) => existing_parent,
            Err(_) => {
                self.enrollment_dao.create_parent(
                    auth_result.auth_user_id,
                    request.school_id,
                    &request.parent_first_name,
                    &request.parent_last_name,
                    &request.parent_email,
                    "Parent"
                ).await?
            }
        };

        // Step 4: Create child
        let created_child = self.enrollment_dao.create_child(
            auth_result.auth_user_id,
            request.school_id,
            &request.child_first_name,
            &request.child_last_name,
            request.child_birth_date,
            &request.gender,
        ).await?;

        // Step 5: Create enrollment
        let created_enrollment = self.enrollment_dao.create_enrollment(
            created_child.id,
            request.school_id,
            request.class_id,
        ).await?;

        // Step 6: Get school default forms
        let school_forms = self.enrollment_dao.get_school_default_forms(request.school_id).await?;

        // Step 7: Get classroom form overrides
        let classroom_overrides = self.enrollment_dao.get_classroom_form_overrides(
            request.class_id,
            request.school_id,
        ).await?;

        // Step 8: Process forms and create assignments
        let assigned_forms = self.process_form_assignments(
            &school_forms,
            &classroom_overrides,
            created_enrollment.id,
            created_child.id,
            request.school_id,
        ).await?;

        let assigned_forms_count = assigned_forms.len();

        let parent = ParentDetails {
            id: created_parent.id,
            school_id: created_parent.school_id,
            first_name: created_parent.first_name.clone(),
            last_name: created_parent.last_name.clone(),
            email: created_parent.email.clone(),
            role: created_parent.role.clone(),
            is_verified: created_parent.is_verified,
            created_at: created_parent.created_at,
        };

        let child = ChildDetails {
            id: created_child.id,
            parent_id: created_child.parent_id,
            school_id: created_child.school_id,
            first_name: created_child.first_name.clone(),
            last_name: created_child.last_name.clone(),
            birth_date: created_child.birth_date,
            gender: created_child.gender.clone(),
            status: created_child.status.clone(),
            created_at: created_child.created_at,
        };

        let enrollment = EnrollmentDetails {
            id: created_enrollment.id,
            child_id: created_enrollment.child_id,
            school_id: created_enrollment.school_id,
            classroom_id: created_enrollment.classroom_id,
            status: created_enrollment.status.clone(),
            application_status: created_enrollment.application_status.clone(),
            created_at: created_enrollment.created_at,
        };

        // Convert form assignments to response format
        let assigned_form_details: Vec<AssignedFormDetails> = assigned_forms.into_iter().map(|form| AssignedFormDetails {
            id: form.id,
            form_template_id: form.form_template_id,
            form_name: form.form_name,
            assignment_source: form.assignment_source,
            status: form.status,
            is_required: form.is_required,
        }).collect();

        // Step 9: Generate response
        let response = ParentInviteResponse {
            parent_id: auth_result.auth_user_id,
            child_id: created_child.id,
            enrollment_id: created_enrollment.id,
            assigned_forms_count,
            invite_id: auth_result.auth_user_id,
            signup_email_sent: true,
            message: "Parent invite created successfully and signup email sent".to_string(),
            details: ParentInviteDetails {
                parent,
                child,
                enrollment,
                assigned_forms: assigned_form_details,
            },
        };

        Ok(response)
    }

    // Create auth user via Supabase
    async fn create_auth_user(
        &self,
        email: &str,
        school_id: Uuid,
        first_name: &str,
        last_name: &str,
        role: &str
    ) -> ApiResult<AuthUserResult> {
        // Create user metadata with school_id and other details
        let metadata = crate::services::supabase_client::UserMetadata::new(
            Some(school_id),
            Some(first_name.to_string()),
            Some(last_name.to_string()),
            Some(role.to_string())
        );

        let auth_user_id_string = self.supabase_client.create_user_invitation_enhanced(email, metadata).await?;

        let auth_user_id = Uuid::parse_str(&auth_user_id_string)
            .map_err(|_| crate::error::AppError::Validation("Invalid UUID format from auth service".to_string()))?;

        Ok(AuthUserResult {
            auth_user_id,
            email: email.to_string(),
        })
    }

    // Process form assignments logic
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
                    continue;
                }
            }
        }

        // Create form assignments using batch operation
        let assigned_forms = self.enrollment_dao.create_student_form_assignments_batch(
            enrollment_id,
            child_id,
            school_id,
            final_forms,
        ).await?;

        Ok(assigned_forms)
    }

    pub async fn resend_parent_confirmation(&self, request: ResendConfirmationRequest) -> ApiResult<ResendConfirmationResponse> {
        // Step 1: Get user email from Supabase auth using parent_id as auth user ID
        let parent_email = self.supabase_client.get_user_email_by_id(request.parent_id).await?;

        // Step 2: Resend confirmation email through Supabase
        self.supabase_client.resend_invitation(&parent_email).await?;

        // Step 3: Generate response
        let response = ResendConfirmationResponse {
            parent_id: request.parent_id,
            email_sent: true,
            message: "Confirmation email resent successfully".to_string(),
            parent_details: ResendConfirmationParentDetails {
                email: parent_email,
            },
        };

        Ok(response)
    }

    pub async fn add_child(&self, request: AddChildRequest) -> ApiResult<AddChildResponse> {
        // Step 1: Verify parent exists and get parent details
        let parent_user = self.enrollment_dao.get_parent_by_id(request.parent_id, request.school_id).await?;

        // Step 2: Validate classroom belongs to school
        if !self.enrollment_dao.verify_classroom_belongs_to_school(request.class_id, request.school_id).await? {
            return Err(AppError::Validation("Classroom does not belong to the specified school".to_string()));
        }

        // Step 3: Create child in children table
        let created_child = self.enrollment_dao.create_child(
            request.parent_id,
            request.school_id,
            &request.child_first_name,
            &request.child_last_name,
            request.child_birth_date,
            &request.gender,
        ).await?;

        // Step 4: Create enrollment
        let created_enrollment = self.enrollment_dao.create_enrollment(
            created_child.id,
            request.school_id,
            request.class_id,
        ).await?;

        // Step 5: Get school default forms
        let school_forms = self.enrollment_dao.get_school_default_forms(request.school_id).await?;

        // Step 6: Get classroom form overrides
        let classroom_overrides = self.enrollment_dao.get_classroom_form_overrides(
            request.class_id,
            request.school_id,
        ).await?;

        // Step 7: Process forms and create assignments
        let assigned_forms = self.process_form_assignments(
            &school_forms,
            &classroom_overrides,
            created_enrollment.id,
            created_child.id,
            request.school_id,
        ).await?;

        // Step 8: Generate response
        let response = AddChildResponse {
            child_id: created_child.id,
            enrollment_id: created_enrollment.id,
            assigned_forms_count: assigned_forms.len(),
            message: "Additional child added successfully".to_string(),
            details: AddChildDetails {
                parent: AddChildParentDetails {
                    id: parent_user.id,
                    first_name: parent_user.first_name,
                    last_name: parent_user.last_name,
                    email: parent_user.email,
                    is_verified: parent_user.is_verified,
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

    pub async fn get_parent_details_by_school(&self, request: GetParentDetailsBySchoolRequest) -> ApiResult<GetParentDetailsBySchoolResponse> {
        // Step 1: Get all parents from the school
        let parents = self.enrollment_dao.get_parents_by_school(request.school_id).await?;

        if parents.is_empty() {
            return Ok(GetParentDetailsBySchoolResponse {
                school_id: request.school_id,
                total_parents: 0,
                message: "No parents found for this school".to_string(),
                parents: vec![],
            });
        }

        // Step 2: Get auth details for all parents in parallel (batch processing)
        let mut tasks = Vec::new();
        for parent in parents {
            let supabase_client = self.supabase_client.clone();
            let task = tokio::spawn(async move {
                match supabase_client.get_user_auth_details(parent.id).await {
                    Ok((auth_email, auth_created_at, id_signed)) => {
                        Some(ParentWithAuthDetails {
                            id: parent.id,
                            school_id: parent.school_id,
                            first_name: parent.first_name,
                            last_name: parent.last_name,
                            email: parent.email,
                            role: parent.role,
                            created_at: parent.created_at.map(|dt| {
                                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                            }).unwrap_or_else(chrono::Utc::now),
                            id_signed,
                        })
                    }
                    Err(e) => {
                        // Log warning and return None if auth details not found
                        eprintln!("Warning: Could not get auth details for parent {}: {}", parent.id, e);
                        None
                    }
                }
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete and collect results
        let mut parents_with_auth = Vec::new();
        for task in tasks {
            if let Ok(Some(parent_with_auth)) = task.await {
                parents_with_auth.push(parent_with_auth);
            }
        }

        // Step 3: Generate response
        let response = GetParentDetailsBySchoolResponse {
            school_id: request.school_id,
            total_parents: parents_with_auth.len(),
            message: format!("Retrieved {} parent details successfully", parents_with_auth.len()),
            parents: parents_with_auth,
        };

        Ok(response)
    }

    // Get Enrollment Children with Form Assignments
    pub async fn get_enrollment_children_with_forms(&self, request: GetEnrollmentChildrenRequest) -> ApiResult<GetEnrollmentChildrenResponse> {
        // Get enrollment children with their form assignments
        let children = self.enrollment_dao.get_enrollment_children_with_forms(request.school_id).await?;

        let response = GetEnrollmentChildrenResponse {
            children,
        };

        Ok(response)
    }

    // Get All Enrollment Form Details by School
    pub async fn get_school_forms(&self, request: GetSchoolFormsRequest) -> ApiResult<GetSchoolFormsResponse> {
        // Get all enrollment form details for the school
        let enrollments = self.enrollment_dao.get_school_forms(request.school_id).await?;

        let response = GetSchoolFormsResponse {
            enrollments,
        };

        Ok(response)
    }

    // Get Class-wise Child Count Details
    pub async fn get_class_wise_count(&self, request: GetClassWiseCountRequest) -> ApiResult<GetClassWiseCountResponse> {
        // Get class-wise count details
        let classes = self.enrollment_dao.get_class_wise_count(request.school_id).await?;

        let response = GetClassWiseCountResponse {
            classes,
        };

        Ok(response)
    }
}