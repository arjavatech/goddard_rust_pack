use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::dao::enrollment_dao::EnrollmentDao;
use crate::services::supabase_client::SupabaseClient;
use crate::services::email_service::{parent_dashboard_url, EmailService};
use crate::services::NotificationService;
use crate::models::notification::{notification_type, CreateNotification};
use crate::models::parent_details::{
    ParentDetailsResponse, ParentChild, ParentChildForm
};
use crate::models::email::{
    ChildAddedNotification, ChildArchivedNotification, ParentDeactivatedNotification,
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
    school_dao: crate::dao::school_dao::SchoolDao,
    supabase_client: SupabaseClient,
    email_service: Arc<EmailService>,
    notification_service: Arc<NotificationService>,
}

impl EnrollmentService {
    pub fn new(
        enrollment_dao: EnrollmentDao,
        school_dao: crate::dao::school_dao::SchoolDao,
        supabase_client: SupabaseClient,
        email_service: Arc<EmailService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            enrollment_dao,
            school_dao,
            supabase_client,
            email_service,
            notification_service,
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

        // Step 1.2: Primary and secondary parent emails must differ
        if let Some(ref secondary_email) = request.secondary_parent_email {
            if secondary_email.to_lowercase() == request.parent_email.to_lowercase() {
                return Err(AppError::Validation(
                    "Primary and secondary parent cannot have the same email address.".to_string()
                ));
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

                (Some(sec_auth_result.auth_user_id), Some(sec_auth_result.email_sent), Some(sec_parent))
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

        // Step 9: Check primary parent email delivery status via Resend
        let primary_email_status = self.supabase_client.get_recent_email_status(&request.parent_email).await;
        if matches!(primary_email_status.as_str(), "suppressed" | "bounced") {
            return Err(crate::error::AppError::ExternalService(match primary_email_status.as_str() {
                "suppressed" => "Email was suppressed by the mail provider. The address may have previously bounced — please ask the recipient to check with their IT or try a different address.".to_string(),
                _ => "Email bounced. Please verify the email address is correct and able to receive mail.".to_string(),
            }));
        }

        // Step 10: Generate response
        let response = ParentInviteResponse {
            parent_id: auth_result.auth_user_id,
            child_id: created_child.id,
            enrollment_id: created_enrollment.id,
            assigned_forms_count,
            invite_id: auth_result.auth_user_id,
            signup_email_sent: auth_result.email_sent,
            secondary_parent_id,
            secondary_signup_email_sent,
            message: if secondary_parent_id.is_some() {
                if auth_result.email_sent {
                    "Parent invite created successfully. Signup emails sent to both primary and secondary parents".to_string()
                } else {
                    "Parent invite created successfully but signup email failed. Use resend-confirmation to retry.".to_string()
                }
            } else if auth_result.email_sent {
                "Parent invite created successfully and signup email sent".to_string()
            } else {
                "Parent invite created successfully but signup email failed. Use resend-confirmation to retry.".to_string()
            },
            details: ParentInviteDetails {
                parent,
                secondary_parent,
                child,
                enrollment,
                assigned_forms: assigned_form_details,
            },
        };

        // Notify school admins that a new parent was invited.
        let parent_full_name = format!("{} {}", request.parent_first_name, request.parent_last_name);
        let child_full_name = format!("{} {}", request.child_first_name, request.child_last_name);
        let invite_classroom = self
            .enrollment_dao
            .get_classroom_name(request.class_id)
            .await
            .unwrap_or_default();
        let invite_classroom_suffix = if invite_classroom.is_empty() {
            String::new()
        } else {
            format!(" Classroom: {}.", invite_classroom)
        };
        self.notification_service.notify_school_admins(
            CreateNotification {
                school_id: request.school_id,
                notification_type: notification_type::PARENT_INVITED.to_string(),
                title: "New Parent Added".to_string(),
                body: format!(
                    "{} ({}) has been invited as parent for {}.{}",
                    parent_full_name, request.parent_email, child_full_name, invite_classroom_suffix
                ),
                related_entity_id: Some(response.parent_id),
                related_entity_type: Some("parent".to_string()),
                action_url: None,
            },
            None,
        ).await;

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
        // STEP 1: Fetch school name FIRST - PREREQUISITE VALIDATION
        tracing::info!("🔍 Fetching school name for school_id: {}", school_id);

        let school_name = match self.school_dao.get_school_name(&school_id).await {
            Ok(name) => {
                tracing::info!("✅ School name fetched: '{}' for school {}", name, school_id);
                name  // String, not Option<String>
            },
            Err(e) => {
                tracing::error!("❌ Failed to fetch school name for {}: {}", school_id, e);
                return Err(crate::error::AppError::Database(format!(
                    "Cannot create parent invitation: School name not found for school_id {}: {}",
                    school_id, e
                )));
            }
        };

        // STEP 2: Create user metadata with VALIDATED school_name
        let metadata = crate::services::supabase_client::UserMetadata::new(
            Some(school_id),
            Some(first_name.to_string()),
            Some(last_name.to_string()),
            Some(role.to_string()),
            None,  // phone_number - not provided in enrollment flow
            None,  // is_verified - will be set after email confirmation
        )
        .with_school_name_option(Some(school_name.clone()));  // school_name is guaranteed to exist

        // STEP 3: Create user in Supabase (no email sent here — we send our own branded email)
        let auth_user_id_string = self.supabase_client.create_user_only_in_supabase(email, metadata).await?;

        let auth_user_id = Uuid::parse_str(&auth_user_id_string)
            .map_err(|_| crate::error::AppError::Validation("Invalid UUID format from auth service".to_string()))?;

        // STEP 4: Store a 7-day invite token in DB
        let invite_token = self.enrollment_dao
            .create_invite_token(email, role, school_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", email, e);
                Uuid::nil()
            });

        // STEP 5: Send branded Resend email with 7-day activation link
        let email_sent = if invite_token != Uuid::nil() {
            self.supabase_client
                .send_parent_invite_email(email, invite_token, first_name, last_name)
                .await
                .unwrap_or(false)
        } else {
            // Fallback: use Supabase's built-in email if token creation failed
            self.supabase_client.resend_invitation(email).await.is_ok()
        };

        Ok(AuthUserResult {
            auth_user_id,
            email: email.to_string(),
            email_sent,
        })
    }

    /// Validate a 7-day invite token and return the URL to redirect the parent to.
    /// - If token is valid and user not yet confirmed → fresh Supabase signup URL
    /// - If token is valid and user already registered → login page URL
    /// - If token is expired / not found → returns Err
    pub async fn activate_invite(&self, token: Uuid) -> ApiResult<String> {
        let result = self.enrollment_dao.get_invite_by_token(token).await?;

        let frontend_url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "https://dev.goddard-web.pages.dev".to_string());

        match result {
            None => Err(AppError::NotFound("Invalid invite link".to_string())),

            // used_at is set by a DB trigger when the user calls updateUser({ password })
            // This is the only reliable signal — encrypted_password is non-empty for all users
            Some((_, _, true)) => {
                Ok(format!("{}/", frontend_url))
            }

            Some((_, false, false)) => Err(AppError::Validation(
                "Invite link has expired (7-day limit). Please contact your school admin to resend the invitation.".to_string(),
            )),

            Some((email, true, false)) => {
                // Valid, not yet used → generate a fresh Supabase set-password link
                let action_link = self.supabase_client.generate_signup_link(&email).await?;
                Ok(action_link)
            }
        }
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
        // A record in class_form_overrides means "this form belongs to this class"
        // Only an explicit "remove" action should exclude a form
        for override_form in classroom_overrides {
            match override_form.action.as_deref() {
                Some("remove") => {
                    final_forms.remove(&override_form.form_template_id);
                }
                _ => {
                    // NULL, "add", "include", or any other value → include the form
                    let form_template = FormTemplate {
                        id: override_form.form_template_id,
                        form_name: override_form.form_name.clone(),
                        is_required: override_form.is_required,
                    };
                    final_forms.insert(override_form.form_template_id, (form_template, "class_override".to_string()));
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
        // Step 1: Look up user from users table to get first_name, last_name, school_id, email
        let user = self.enrollment_dao.get_user_by_id(request.parent_id).await?;

        // Step 2: Create a fresh 7-day invite token
        let invite_token = self.enrollment_dao
            .create_invite_token(&user.email, &user.role, user.school_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to store invite token for {}: {}", user.email, e);
                uuid::Uuid::nil()
            });

        // Step 3: Send branded invite email via Resend
        let email_sent = if invite_token != uuid::Uuid::nil() {
            self.supabase_client
                .send_parent_invite_email(&user.email, invite_token, &user.first_name, &user.last_name)
                .await
                .unwrap_or(false)
        } else {
            false
        };

        // Step 4: Check delivery status
        let email_status = self.supabase_client.get_recent_email_status(&user.email).await;
        if matches!(email_status.as_str(), "suppressed" | "bounced") {
            return Err(AppError::ExternalService(match email_status.as_str() {
                "suppressed" => "Email was suppressed by the mail provider. The address may have previously bounced — please ask the recipient to check with their IT or try a different address.".to_string(),
                _ => "Email bounced. Please verify the email address is correct and able to receive mail.".to_string(),
            }));
        }

        Ok(ResendConfirmationResponse {
            parent_id: request.parent_id,
            email_sent,
            message: "Confirmation email resent successfully".to_string(),
            parent_details: ResendConfirmationParentDetails {
                email: user.email,
            },
        })
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

        // Fire child-added notification (non-blocking).
        let email_svc = self.email_service.clone();
        let classroom_name = self
            .enrollment_dao
            .get_classroom_name(request.class_id)
            .await
            .unwrap_or_default();
        let school_name = self
            .enrollment_dao
            .get_school_name(request.school_id)
            .await
            .unwrap_or_default();
        let classroom_for_inapp = classroom_name.clone();
        let notification = ChildAddedNotification {
            parent_email: response.details.parent.email.clone(),
            parent_first_name: response.details.parent.first_name.clone(),
            child_name: format!(
                "{} {}",
                response.details.child.first_name, response.details.child.last_name
            ),
            child_dob: response.details.child.birth_date,
            classroom_name,
            school_name,
            added_on: Utc::now(),
            form_count: response.assigned_forms_count,
            dashboard_url: parent_dashboard_url(),
        };
        tokio::spawn(async move {
            if let Err(e) = email_svc.send_child_added_email(notification).await {
                eprintln!("[EmailService] child_added notification failed (non-fatal): {:?}", e);
            }
        });

        // In-app notifications (parent + all school admins).
        let child_full_name = format!(
            "{} {}",
            response.details.child.first_name, response.details.child.last_name
        );
        let parent_full_name = format!(
            "{} {}",
            response.details.parent.first_name, response.details.parent.last_name
        );
        let dob_suffix = match response.details.child.birth_date {
            Some(d) => format!(" (DOB {})", d.format("%b %d, %Y")),
            None => String::new(),
        };
        let classroom_suffix = if classroom_for_inapp.is_empty() {
            String::new()
        } else {
            format!(" Classroom: {}.", classroom_for_inapp)
        };

        self.notification_service.notify_user(
            response.details.parent.id,
            CreateNotification {
                school_id: request.school_id,
                notification_type: notification_type::CHILD_ADDED.to_string(),
                title: "New Child Added".to_string(),
                body: format!(
                    "{}{} has been added to your account.{}",
                    child_full_name, dob_suffix, classroom_suffix
                ),
                related_entity_id: Some(response.details.child.id),
                related_entity_type: Some("child".to_string()),
                action_url: Some("/dashboard".to_string()),
            },
        ).await;
        self.notification_service.notify_school_admins(
            CreateNotification {
                school_id: request.school_id,
                notification_type: notification_type::CHILD_ADDED.to_string(),
                title: "New Student Added".to_string(),
                body: format!(
                    "{}{} was added to {} ({})'s account.{}",
                    child_full_name,
                    dob_suffix,
                    parent_full_name,
                    response.details.parent.email,
                    classroom_suffix
                ),
                related_entity_id: Some(response.details.child.id),
                related_entity_type: Some("child".to_string()),
                action_url: None,
            },
            None,
        ).await;

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
                        gender: row.child_gender.clone(),
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
                        due_date: row.due_date.map(|d| d.format("%d-%m-%Y").to_string()),
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

        // Capture parent email + school name BEFORE the DAO update so the
        // notification has everything it needs even if the user record changes.
        let parent_user = self.enrollment_dao.get_user_by_id(parent_id).await.ok();

        let response = self.enrollment_dao.deactivate_parent(parent_id).await?;

        if let Some(user) = parent_user {
            let email_svc = self.email_service.clone();
            let school_name = self
                .enrollment_dao
                .get_school_name(user.school_id)
                .await
                .unwrap_or_default();
            let school_name_for_inapp = school_name.clone();
            let notification = ParentDeactivatedNotification {
                parent_email: user.email.clone(),
                parent_first_name: user.first_name.clone(),
                parent_full_name: format!("{} {}", user.first_name, user.last_name),
                school_name,
                deactivated_on: Utc::now(),
                children_count: response.deactivated_children_count,
                enrollments_count: response.deactivated_enrollments_count,
            };
            tokio::spawn(async move {
                if let Err(e) = email_svc.send_parent_deactivated_email(notification).await {
                    eprintln!("[EmailService] parent_deactivated notification failed (non-fatal): {:?}", e);
                }
            });

            // In-app: notify the parent themselves AND all admins of the school.
            let parent_full = format!("{} {}", user.first_name, user.last_name);
            self.notification_service.notify_user(
                user.id,
                CreateNotification {
                    school_id: user.school_id,
                    notification_type: notification_type::PARENT_DEACTIVATED.to_string(),
                    title: "Account Deactivated".to_string(),
                    body: format!(
                        "Your account at {} has been deactivated. {} child(ren) and {} enrollment(s) affected. Contact your school administrator with questions.",
                        school_name_for_inapp,
                        response.deactivated_children_count,
                        response.deactivated_enrollments_count
                    ),
                    related_entity_id: Some(user.id),
                    related_entity_type: Some("parent".to_string()),
                    action_url: None,
                },
            ).await;
            self.notification_service.notify_school_admins(
                CreateNotification {
                    school_id: user.school_id,
                    notification_type: notification_type::PARENT_DEACTIVATED.to_string(),
                    title: "Parent Deactivated".to_string(),
                    body: format!(
                        "{} ({}) has been deactivated. {} child(ren), {} enrollment(s) paused.",
                        parent_full,
                        user.email,
                        response.deactivated_children_count,
                        response.deactivated_enrollments_count
                    ),
                    related_entity_id: Some(user.id),
                    related_entity_type: Some("parent".to_string()),
                    action_url: None,
                },
                None,
            ).await;
        } else {
            println!("[EnrollmentService] Skipping deactivation email — could not load parent user");
        }

        Ok(response)
    }

    // Activate parent and all related children and enrollments
    pub async fn activate_parent(&self, parent_id: Uuid) -> ApiResult<ActivateParentResponse> {
        println!("[DEBUG] EnrollmentService: Activating parent {}", parent_id);
        self.enrollment_dao.activate_parent(parent_id).await
    }

    // Update child status (admin only - no validation, accepts any status value)
    pub async fn update_child_status(&self, child_id: Uuid, request: crate::models::enrollment::UpdateChildStatusRequest) -> ApiResult<crate::models::enrollment::UpdateChildStatusResponse> {
        println!("[DEBUG] EnrollmentService: Updating child {} status to: {}", child_id, request.status);
        let response = self.enrollment_dao.update_child_status(child_id, &request.status).await?;

        // Fire child-archived notification (non-blocking) when the new status is
        // "archive" or "archived" — the frontend sends "archive" today, but accept
        // both spellings so any other caller works too.
        let normalized_status = request.status.trim().to_ascii_lowercase();
        if normalized_status == "archive" || normalized_status == "archived" {
            match self.enrollment_dao.get_child_notification_context(child_id).await {
                Ok(ctx) => {
                    let school_name = self
                        .enrollment_dao
                        .get_school_name(ctx.school_id)
                        .await
                        .unwrap_or_default();
                    let recipients = match ctx.secondary_parent_email.as_ref() {
                        Some(sp) if !sp.trim().is_empty() => {
                            format!("{},{}", ctx.parent_email, sp)
                        }
                        _ => ctx.parent_email.clone(),
                    };
                    let school_name_for_inapp = school_name.clone();
                    let notification = ChildArchivedNotification {
                        parent_email: recipients,
                        parent_first_name: ctx.parent_first_name.clone(),
                        child_name: format!("{} {}", ctx.child_first_name, ctx.child_last_name),
                        school_name,
                        archived_on: Utc::now(),
                    };
                    let email_svc = self.email_service.clone();
                    tokio::spawn(async move {
                        if let Err(e) = email_svc.send_child_archived_email(notification).await {
                            eprintln!("[EmailService] child_archived notification failed (non-fatal): {:?}", e);
                        }
                    });

                    // In-app: parent + secondary parent + all school admins.
                    let child_full = format!("{} {}", ctx.child_first_name, ctx.child_last_name);
                    let parent_full = format!("{} {}", ctx.parent_first_name, ctx.parent_last_name);
                    let classroom_text = ctx
                        .classroom_name
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("n/a");
                    let parent_body = format!(
                        "{}'s record at {} has been archived. Their enrollment is no longer active.",
                        child_full, school_name_for_inapp
                    );
                    self.notification_service.notify_user(
                        ctx.parent_id,
                        CreateNotification {
                            school_id: ctx.school_id,
                            notification_type: notification_type::CHILD_ARCHIVED.to_string(),
                            title: "Child Archived".to_string(),
                            body: parent_body.clone(),
                            related_entity_id: Some(child_id),
                            related_entity_type: Some("child".to_string()),
                            action_url: None,
                        },
                    ).await;
                    if let Some(secondary_id) = ctx.secondary_parent_id {
                        self.notification_service.notify_user(
                            secondary_id,
                            CreateNotification {
                                school_id: ctx.school_id,
                                notification_type: notification_type::CHILD_ARCHIVED.to_string(),
                                title: "Child Archived".to_string(),
                                body: parent_body,
                                related_entity_id: Some(child_id),
                                related_entity_type: Some("child".to_string()),
                                action_url: None,
                            },
                        ).await;
                    }
                    self.notification_service.notify_school_admins(
                        CreateNotification {
                            school_id: ctx.school_id,
                            notification_type: notification_type::CHILD_ARCHIVED.to_string(),
                            title: "Student Archived".to_string(),
                            body: format!(
                                "{} (parent: {}) has been archived. Classroom: {}.",
                                child_full, parent_full, classroom_text
                            ),
                            related_entity_id: Some(child_id),
                            related_entity_type: Some("child".to_string()),
                            action_url: None,
                        },
                        None,
                    ).await;
                }
                Err(e) => {
                    println!("[EnrollmentService] Skipping archive email — could not load child context: {:?}", e);
                }
            }
        }

        Ok(response)
    }

    // ==========================================
    // CLASS TRANSITIONS SERVICE METHODS
    // ==========================================

    /// Promote student to next class (creates new transition record via trigger)
    pub async fn promote_enrollment(
        &self,
        enrollment_id: Uuid,
        request: crate::models::enrollment::PromoteEnrollmentRequest,
        changed_by_user_id: Uuid,
        school_id: Uuid,
    ) -> ApiResult<crate::models::enrollment::PromoteEnrollmentResponse> {
        println!("[DEBUG] EnrollmentService: Promoting enrollment {} to classroom {}", enrollment_id, request.to_classroom_id);

        // Step 1: Get current enrollment details
        let enrollment = self.enrollment_dao.get_enrollment_with_classroom(enrollment_id, school_id).await?;

        // Step 2: Verify target classroom belongs to same school
        if !self.enrollment_dao.verify_classroom_belongs_to_school(request.to_classroom_id, school_id).await? {
            return Err(AppError::Validation("Target classroom does not belong to the school".to_string()));
        }

        // Step 3: Verify different classroom (prevent no-op)
        if enrollment.classroom_id == request.to_classroom_id {
            // Allow if this is the first transition (backfilling scenario)
            let has_transitions = self.enrollment_dao
                .has_any_transitions_for_enrollment(enrollment_id)
                .await?;

            if has_transitions {
                return Err(AppError::Validation(
                    "Student is already in the target classroom".to_string()
                ));
            }

            // Allow creating first transition record even to same classroom
            println!("[DEBUG] Allowing promotion to same classroom - creating first transition record for enrollment {}", enrollment_id);
        }

        // Step 4: Update enrollment with user context (single transaction)
        // This sets the session variable and updates the classroom in one transaction
        // so the database trigger can capture the changed_by user
        self.enrollment_dao.update_enrollment_classroom_with_user_context(
            enrollment_id,
            request.to_classroom_id,
            request.effective_date.map(|dt| dt.naive_utc()),
            changed_by_user_id
        ).await?;

        // Step 6: Get the newly created transition record
        let transition = self.enrollment_dao.get_latest_transition_for_enrollment(enrollment_id).await?;

        // Step 7: Update transition reason if provided
        if let Some(reason) = &request.reason {
            self.enrollment_dao.update_transition_reason(transition.id, reason).await?;
        }

        // Step 8: Get classroom details for response
        let from_classroom = self.enrollment_dao.get_classroom_by_id(enrollment.classroom_id).await?;
        let to_classroom = self.enrollment_dao.get_classroom_by_id(request.to_classroom_id).await?;

        // Build response
        let message = format!("Student successfully promoted from {} to {}", from_classroom.name, to_classroom.name);

        let response = crate::models::enrollment::PromoteEnrollmentResponse {
            enrollment_id,
            child_id: enrollment.child_id,
            child_name: format!("{} {}", enrollment.child_first_name, enrollment.child_last_name),
            from_classroom: crate::models::enrollment::ClassroomInfo {
                id: from_classroom.id,
                name: from_classroom.name,
            },
            to_classroom: crate::models::enrollment::ClassroomInfo {
                id: to_classroom.id,
                name: to_classroom.name,
            },
            transition: crate::models::enrollment::TransitionInfo {
                id: transition.id,
                transitioned_at: transition.transitioned_at,
                changed_by: Some(changed_by_user_id),
                reason: request.reason,
            },
            message,
        };

        println!("[DEBUG] EnrollmentService: Successfully promoted enrollment {}", enrollment_id);
        Ok(response)
    }

    /// Edit existing class transition record (no new entry created)
    pub async fn edit_class_transition(
        &self,
        enrollment_id: Uuid,
        request: crate::models::enrollment::EditClassTransitionRequest,
        school_id: Uuid,
    ) -> ApiResult<crate::models::enrollment::EditClassTransitionResponse> {
        println!("[DEBUG] EnrollmentService: Editing latest transition for enrollment {}", enrollment_id);

        // Step 1A: Verify enrollment belongs to school
        let enrollment_exists = self.enrollment_dao
            .verify_enrollment_belongs_to_school(enrollment_id, school_id)
            .await?;

        if !enrollment_exists {
            return Err(AppError::NotFound(
                format!("Enrollment {} not found or does not belong to school", enrollment_id)
            ));
        }

        // Step 1B: Get latest transition for enrollment
        let transition = self.enrollment_dao
            .get_latest_transition_for_enrollment(enrollment_id)
            .await
            .map_err(|e| match e {
                AppError::Database(ref msg) if msg.contains("query returned no rows") => {
                    AppError::NotFound(
                        format!("No class transitions found for enrollment {}", enrollment_id)
                    )
                },
                _ => e
            })?;

        // Step 1C: Security check - verify transition belongs to school
        if transition.school_id != school_id {
            return Err(AppError::Authorization(
                "This transition does not belong to your school".to_string()
            ));
        }

        let transition_id = transition.id;  // Extract for subsequent operations

        // Step 2: Validate new classroom if provided
        if let Some(new_classroom_id) = request.to_classroom_id {
            if !self.enrollment_dao.verify_classroom_belongs_to_school(new_classroom_id, school_id).await? {
                return Err(AppError::Validation("Target classroom does not belong to the school".to_string()));
            }

            // Prevent setting to same classroom as 'from'
            if new_classroom_id == transition.from_classroom_id {
                return Err(AppError::Validation("Cannot set to_classroom same as from_classroom".to_string()));
            }
        }

        // Step 3: Update transition record (NO NEW ENTRY)
        // Convert DateTime<Utc> to NaiveDateTime for database storage
        let updated_transition = self.enrollment_dao.update_transition_record(
            transition_id,
            request.to_classroom_id,
            request.reason.clone(),
            request.transitioned_at.map(|dt| dt.naive_utc()),
        ).await?;

        // Step 4: Optionally sync enrollment if requested
        let mut enrollment_synced = false;
        if request.sync_enrollment.unwrap_or(false) && request.to_classroom_id.is_some() {
            self.enrollment_dao.update_enrollment_classroom_direct(
                transition.enrollment_id,
                request.to_classroom_id.unwrap(),
            ).await?;
            enrollment_synced = true;
        }

        // Step 5: Get classroom details
        let from_classroom = self.enrollment_dao.get_classroom_by_id(transition.from_classroom_id).await?;
        let to_classroom = self.enrollment_dao.get_classroom_by_id(updated_transition.to_classroom_id).await?;

        // Step 6: Get child name
        let child = self.enrollment_dao.get_child_by_id(transition.child_id).await?;

        let response = crate::models::enrollment::EditClassTransitionResponse {
            transition_id,
            enrollment_id: transition.enrollment_id,
            child_name: format!("{} {}", child.first_name, child.last_name),
            from_classroom: crate::models::enrollment::ClassroomInfo {
                id: from_classroom.id,
                name: from_classroom.name,
            },
            to_classroom: crate::models::enrollment::ClassroomInfo {
                id: to_classroom.id,
                name: to_classroom.name,
            },
            transitioned_at: updated_transition.transitioned_at,
            reason: updated_transition.reason,
            enrollment_synced,
            message: "Transition record updated successfully".to_string(),
        };

        println!("[DEBUG] EnrollmentService: Successfully edited class transition {}", transition_id);
        Ok(response)
    }

    /// Bulk promote multiple students to new classrooms
    pub async fn bulk_promote_enrollments(
        &self,
        request: crate::models::enrollment::BulkPromoteEnrollmentsRequest,
        changed_by_user_id: Uuid,
        auth_school_id: Uuid,
    ) -> ApiResult<crate::models::enrollment::BulkPromoteEnrollmentsResponse> {
        use crate::models::enrollment::{PromoteEnrollmentResponse, FailedPromotion, PromotionSummary};

        println!("[DEBUG] EnrollmentService: Bulk promoting {} students", request.promotions.len());

        // Validation 1: School ID must match auth school ID
        if request.school_id != auth_school_id {
            return Err(AppError::Validation(
                "School ID in request does not match authenticated user's school".to_string()
            ));
        }

        // Validation 2: Array must have at least 1 promotion
        if request.promotions.is_empty() {
            return Err(AppError::Validation("Promotions array cannot be empty".to_string()));
        }

        // Validation 3: Maximum 100 promotions per request
        if request.promotions.len() > 100 {
            return Err(AppError::Validation(
                format!("Cannot process more than 100 promotions at once. Received: {}", request.promotions.len())
            ));
        }

        // Validation 4: Pre-validate all target classrooms belong to school
        let unique_classroom_ids: std::collections::HashSet<Uuid> = request.promotions
            .iter()
            .map(|p| p.to_classroom_id)
            .collect();

        for classroom_id in unique_classroom_ids {
            if !self.enrollment_dao.verify_classroom_belongs_to_school(classroom_id, request.school_id).await? {
                return Err(AppError::Validation(
                    format!("Classroom {} does not belong to school {}", classroom_id, request.school_id)
                ));
            }
        }

        // Process each promotion individually with user context in transaction
        let mut successful: Vec<PromoteEnrollmentResponse> = Vec::new();
        let mut failed: Vec<FailedPromotion> = Vec::new();

        for promotion in request.promotions {
            match self.promote_single_enrollment_internal(
                promotion.enrollment_id,
                promotion.to_classroom_id,
                promotion.reason,
                promotion.effective_date,
                changed_by_user_id,
                request.school_id,
            ).await {
                Ok(response) => {
                    successful.push(response);
                }
                Err(e) => {
                    // Try to get child name for better error reporting
                    let child_name = self.enrollment_dao.get_enrollment_with_classroom(promotion.enrollment_id, request.school_id)
                        .await
                        .ok()
                        .map(|e| format!("{} {}", e.child_first_name, e.child_last_name));

                    failed.push(FailedPromotion {
                        enrollment_id: promotion.enrollment_id,
                        child_name,
                        to_classroom_id: promotion.to_classroom_id,
                        error: e.to_string(),
                    });
                }
            }
        }

        let summary = PromotionSummary {
            total_requested: successful.len() + failed.len(),
            successful_count: successful.len(),
            failed_count: failed.len(),
        };

        println!("[DEBUG] EnrollmentService: Bulk promotion complete - {}/{} successful",
            summary.successful_count, summary.total_requested);

        Ok(crate::models::enrollment::BulkPromoteEnrollmentsResponse {
            successful,
            failed,
            summary,
        })
    }

    /// Internal helper to promote a single enrollment (used by both individual and bulk endpoints)
    async fn promote_single_enrollment_internal(
        &self,
        enrollment_id: Uuid,
        to_classroom_id: Uuid,
        reason: Option<String>,
        effective_date: Option<DateTime<Utc>>,
        changed_by_user_id: Uuid,
        school_id: Uuid,
    ) -> ApiResult<crate::models::enrollment::PromoteEnrollmentResponse> {
        // Step 1: Get current enrollment details
        // Note: DAO query enforces school_id and is_active constraints via WHERE clause
        let enrollment = self.enrollment_dao.get_enrollment_with_classroom(enrollment_id, school_id).await?;

        // Step 2: Verify different classroom (prevent no-op)
        if enrollment.classroom_id == to_classroom_id {
            // Allow if this is the first transition (backfilling scenario)
            let has_transitions = self.enrollment_dao
                .has_any_transitions_for_enrollment(enrollment_id)
                .await?;

            if has_transitions {
                return Err(AppError::Validation(
                    "Student is already in the target classroom".to_string()
                ));
            }

            // Allow creating first transition record even to same classroom
            println!("[DEBUG] Bulk promotion: Allowing promotion to same classroom - creating first transition record for enrollment {}", enrollment_id);
        }

        // Step 3: Update enrollment with user context (single transaction)
        // This sets the session variable and updates the classroom in one transaction
        // so the database trigger can capture the changed_by user
        self.enrollment_dao.update_enrollment_classroom_with_user_context(
            enrollment_id,
            to_classroom_id,
            effective_date.map(|dt| dt.naive_utc()),
            changed_by_user_id
        ).await?;

        // Step 4: Get the newly created transition record
        let transition = self.enrollment_dao.get_latest_transition_for_enrollment(enrollment_id).await?;

        // Step 5: Update transition reason if provided
        if let Some(reason_text) = &reason {
            self.enrollment_dao.update_transition_reason(transition.id, reason_text).await?;
        }

        // Step 6: Get classroom details for response
        let from_classroom = self.enrollment_dao.get_classroom_by_id(enrollment.classroom_id).await?;
        let to_classroom = self.enrollment_dao.get_classroom_by_id(to_classroom_id).await?;

        // Build response
        let message = format!("Student successfully promoted from {} to {}", from_classroom.name, to_classroom.name);

        Ok(crate::models::enrollment::PromoteEnrollmentResponse {
            enrollment_id,
            child_id: enrollment.child_id,
            child_name: format!("{} {}", enrollment.child_first_name, enrollment.child_last_name),
            from_classroom: crate::models::enrollment::ClassroomInfo {
                id: from_classroom.id,
                name: from_classroom.name,
            },
            to_classroom: crate::models::enrollment::ClassroomInfo {
                id: to_classroom.id,
                name: to_classroom.name,
            },
            transition: crate::models::enrollment::TransitionInfo {
                id: transition.id,
                transitioned_at: transition.transitioned_at,
                changed_by: Some(changed_by_user_id),
                reason,
            },
            message,
        })
    }
}