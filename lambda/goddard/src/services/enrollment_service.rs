use uuid::Uuid;
use std::collections::HashMap;

use crate::dao::enrollment_dao::EnrollmentDao;
use crate::services::supabase_client::SupabaseClient;
use crate::models::parent_details::{
    ParentDetailsResponse, ParentChild, ParentChildForm
};
use crate::models::enrollment::{
    ParentInviteRequest, ParentInviteResponse, ParentInviteDetails,
    ParentDetails, ChildDetails, EnrollmentDetails, AssignedFormDetails,
    AuthUserResult, FormTemplate, ClassFormOverride, CreatedFormAssignment,
    ResendConfirmationRequest, ResendConfirmationResponse, ResendConfirmationParentDetails,
    AddChildRequest, AddChildResponse, AddChildDetails, AddChildParentDetails,
    GetParentDetailsBySchoolRequest, GetParentDetailsBySchoolResponse, ParentWithAuthDetails,
    ParentWithChildren, ChildWithForms, FormStatus,
    GetEnrollmentChildrenRequest, GetEnrollmentChildrenResponse, EnrollmentChildWithForms,
    GetClassWiseCountRequest, GetClassWiseCountResponse,
    GetSchoolFormsRequest, GetSchoolFormsResponse,
    DeactivateParentResponse, ActivateParentResponse
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

        // Step 1.1: Validate secondary parent fields if email is provided
        if request.secondary_parent_email.is_some() {
            if request.secondary_parent_first_name.is_none() || request.secondary_parent_last_name.is_none() {
                return Err(AppError::Validation("Secondary parent first name and last name are required when secondary parent email is provided".to_string()));
            }
        }

        // Step 2: Create auth user via Supabase (primary parent)
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

        // Step 3.1: Create secondary parent if provided
        let (secondary_parent_id, secondary_signup_email_sent, created_secondary_parent) =
            if let (Some(sec_email), Some(sec_first_name), Some(sec_last_name)) = (
                &request.secondary_parent_email,
                &request.secondary_parent_first_name,
                &request.secondary_parent_last_name,
            ) {
                // Create auth user for secondary parent
                let sec_auth_result = self.create_auth_user(
                    sec_email,
                    request.school_id,
                    sec_first_name,
                    sec_last_name,
                    "secondary-parent"
                ).await?;

                // Get or create secondary parent user record
                let sec_parent = match self.enrollment_dao.get_parent_by_id(sec_auth_result.auth_user_id, request.school_id).await {
                    Ok(existing) => existing,
                    Err(_) => {
                        self.enrollment_dao.create_parent(
                            sec_auth_result.auth_user_id,
                            request.school_id,
                            sec_first_name,
                            sec_last_name,
                            sec_email,
                            "secondary-parent"
                        ).await?
                    }
                };

                (Some(sec_auth_result.auth_user_id), Some(true), Some(sec_parent))
            } else {
                (None, None, None)
            };

        // Step 4: Create child with optional secondary_parent_id
        let created_child = self.enrollment_dao.create_child(
            auth_result.auth_user_id,
            request.school_id,
            &request.child_first_name,
            &request.child_last_name,
            request.child_birth_date,
            &request.gender,
            secondary_parent_id,
        ).await?;

        // Step 5: Create enrollment
        let created_enrollment = self.enrollment_dao.create_enrollment(
            created_child.id,
            request.school_id,
            request.class_id,
        ).await?;

        // Step 6 & 7: Get forms data in parallel for better performance
        let (school_forms, classroom_overrides) = tokio::try_join!(
            self.enrollment_dao.get_school_default_forms(request.school_id),
            self.enrollment_dao.get_classroom_form_overrides(request.class_id, request.school_id)
        )?;

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

        // Build secondary parent details if created
        let secondary_parent = created_secondary_parent.map(|sp| ParentDetails {
            id: sp.id,
            school_id: sp.school_id,
            first_name: sp.first_name,
            last_name: sp.last_name,
            email: sp.email,
            role: sp.role,
            is_verified: sp.is_verified,
            created_at: sp.created_at,
        });

        let child = ChildDetails {
            id: created_child.id,
            parent_id: created_child.parent_id,
            secondary_parent_id: created_child.secondary_parent_id,
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
            secondary_parent_id,
            secondary_signup_email_sent,
            message: if secondary_parent_id.is_some() {
                "Parent invite created successfully. Signup emails sent to both primary and secondary parents".to_string()
            } else {
                "Parent invite created successfully and signup email sent".to_string()
            },
            details: ParentInviteDetails {
                parent,
                secondary_parent,
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
            Some(role.to_string()),
            None,  // phone_number - not provided in enrollment flow
            None,  // is_verified - will be set after email confirmation
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

        // Step 3: Create child in children table (no secondary parent for add_child flow)
        let created_child = self.enrollment_dao.create_child(
            request.parent_id,
            request.school_id,
            &request.child_first_name,
            &request.child_last_name,
            request.child_birth_date,
            &request.gender,
            None, // secondary_parent_id - not supported in add_child flow
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
                    secondary_parent_id: created_child.secondary_parent_id,
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
        // Get parent details with children and forms - separated by active/inactive status
        let (active_parents, inactive_parents) = self.enrollment_dao.get_parent_details_with_children_and_forms(request.school_id).await?;

        // Generate response with both active and inactive parent lists
        let response = GetParentDetailsBySchoolResponse {
            active_parents,
            inactive_parents,
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

    // Get Class-Based Enrollment Form Details
    pub async fn get_class_based_enrollments(&self, request: crate::models::enrollment::GetClassBasedEnrollmentsRequest) -> ApiResult<crate::models::enrollment::GetClassBasedEnrollmentsResponse> {
        // Get class-based enrollment form details
        let enrollments = self.enrollment_dao.get_class_based_enrollments(request.school_id, request.class_id).await?;

        let response = crate::models::enrollment::GetClassBasedEnrollmentsResponse {
            enrollments,
        };

        Ok(response)
    }

    // Get parent details by parent ID
    pub async fn get_parent_details_by_id(&self, parent_id: Uuid) -> ApiResult<ParentDetailsResponse> {
        println!("[DEBUG] EnrollmentService: Getting parent details for ID: {}", parent_id);

        // Get parent details from DAO
        let rows = self.enrollment_dao.get_parent_details_by_id(parent_id).await?;

        if rows.is_empty() {
            return Err(AppError::NotFound(format!("Parent with ID {} not found", parent_id)));
        }

        // Group data by child
        let mut children_map: HashMap<Uuid, ParentChild> = HashMap::new();

        // Get parent info from first row
        let first_row = &rows[0];
        let parent_id = first_row.parent_id;
        let parent_email = first_row.parent_email.clone();
        let parent_first_name = first_row.parent_first_name.clone();
        let parent_last_name = first_row.parent_last_name.clone();
        let signed_status = first_row.signed_status.clone();

        for row in rows {
            // Skip rows without child data (parent exists but has no children)
            if let (Some(child_id), Some(child_first_name), Some(child_last_name)) =
                (&row.child_id, &row.child_first_name, &row.child_last_name) {

                // Determine parent_type based on whether requesting parent is primary or secondary
                let parent_type = if row.child_parent_id == Some(parent_id) {
                    "primary_parent".to_string()
                } else {
                    "secondary_parent".to_string()
                };

                let child = children_map.entry(*child_id).or_insert_with(|| {
                    ParentChild {
                        child_id: *child_id,
                        child_full_name: format!("{} {}", child_first_name, child_last_name),
                        child_dob: row.child_dob,
                        child_status: row.child_status.clone(),
                        enrollment_id: row.enrollment_id.unwrap_or_default(),
                        classroom_id: row.classroom_id.unwrap_or_default(),
                        classroom_name: row.classroom_name.clone().unwrap_or_default(),
                        parent_type,
                        forms: Vec::new(),
                    }
                });

                // Add form if present
                if let (Some(assignment_id), Some(form_template_id), Some(form_name)) =
                    (&row.student_form_assignment_id, &row.form_template_id, &row.form_name) {

                    // Build fillout_form_id by replacing the placeholder 'xxxxx' with actual assignment_id
                    let fillout_form_id = if let Some(fillout_id) = &row.fillout_form_id {
                        Some(fillout_id.replace("xxxxx", &assignment_id.to_string()))
                    } else {
                        None
                    };

                    child.forms.push(ParentChildForm {
                        form_id: format!("form_{}", form_template_id),
                        student_form_assignment_id: *assignment_id,
                        fillout_form_id,
                        form_name: form_name.clone(),
                        status: row.status.clone().unwrap_or_else(|| "incomplete".to_string()),
                        is_required: row.is_required.unwrap_or(false),
                        recent_edit_link: row.recent_edit_link.clone(),
                        recent_pdf_link: row.recent_pdf_link.clone(),
                        approved_by: row.approved_by,
                        approved_on: row.approved_on,
                        assigned_at: row.assigned_at.map(|dt| dt.format("%d-%m-%Y").to_string()),
                    });
                }
            }
        }

        let response = ParentDetailsResponse {
            parent_id,
            parent_email,
            parent_first_name,
            parent_last_name,
            signed_status,
            children: children_map.into_values().collect(),
        };

        println!("[DEBUG] EnrollmentService: Successfully retrieved parent details with {} children", response.children.len());
        Ok(response)
    }

    pub async fn validate_api_key(&self, api_key: &str) -> ApiResult<()> {
        println!("[DEBUG] EnrollmentService: Validating API key");

        // Use the same API key validation as other endpoints
        let expected_api_key = match std::env::var("OWNER_API_KEY") {
            Ok(key) => {
                println!("[DEBUG] EnrollmentService: OWNER_API_KEY found");
                key
            }
            Err(e) => {
                println!("[ERROR] EnrollmentService: OWNER_API_KEY not configured: {:?}", e);
                return Err(AppError::Internal("OWNER_API_KEY not configured".to_string()));
            }
        };

        if api_key != expected_api_key {
            println!("[ERROR] EnrollmentService: API key mismatch");
            return Err(AppError::Authentication("Invalid API key".to_string()));
        }

        println!("[DEBUG] EnrollmentService: API key validation successful");
        Ok(())
    }

    // Deactivate parent and all related children and enrollments
    pub async fn deactivate_parent(&self, parent_id: Uuid) -> ApiResult<DeactivateParentResponse> {
        println!("[DEBUG] EnrollmentService: Deactivating parent {}", parent_id);
        self.enrollment_dao.deactivate_parent(parent_id).await
    }

    // Activate parent and all related children and enrollments
    pub async fn activate_parent(&self, parent_id: Uuid) -> ApiResult<ActivateParentResponse> {
        println!("[DEBUG] EnrollmentService: Activating parent {}", parent_id);
        self.enrollment_dao.activate_parent(parent_id).await
    }

    // Update child status (admin only - no validation, accepts any status value)
    pub async fn update_child_status(&self, child_id: Uuid, request: crate::models::enrollment::UpdateChildStatusRequest) -> ApiResult<crate::models::enrollment::UpdateChildStatusResponse> {
        println!("[DEBUG] EnrollmentService: Updating child {} status to: {}", child_id, request.status);
        self.enrollment_dao.update_child_status(child_id, &request.status).await
    }
}