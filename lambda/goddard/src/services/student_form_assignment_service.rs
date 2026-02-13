use crate::dao::StudentFormAssignmentDao;
use crate::models::student_form_assignment::{
    StudentFormAssignment, StudentFormAssignmentResponse, CreateStudentFormAssignmentRequest,
    UpdateStudentFormAssignmentRequest, DeleteStudentFormAssignmentResponse,
    BulkAssignFormRequest, BulkAssignFormResponse, FailedAssignment
};
use crate::models::student_form_assignment_review::{
    ReviewStudentFormAssignmentRequest, ReviewStudentFormAssignmentResponse
};
use crate::error::AppError;
use uuid::Uuid;

pub struct StudentFormAssignmentService {
    dao: StudentFormAssignmentDao,
}

impl StudentFormAssignmentService {
    pub fn new(dao: StudentFormAssignmentDao) -> Self {
        Self { dao }
    }

    pub async fn create_student_form_assignment(
        &self,
        request: CreateStudentFormAssignmentRequest,
    ) -> Result<StudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting assignment creation");
        println!("[DEBUG] StudentFormAssignmentService: Request data: {:?}", request);

        // Create the assignment
        let assignment = self.dao
            .create_student_form_assignment(&request)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Assignment created successfully with ID: {}", assignment.id);
        Ok(assignment.into())
    }

    pub async fn get_assignments_by_school(
        &self,
        school_id: Uuid,
    ) -> Result<Vec<StudentFormAssignmentResponse>, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Getting assignments for school: {}", school_id);

        let assignments = self.dao
            .get_assignments_by_school(school_id)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Found {} assignments", assignments.len());
        Ok(assignments.into_iter().map(|a| a.into()).collect())
    }

    pub async fn update_student_form_assignment(
        &self,
        request: UpdateStudentFormAssignmentRequest,
    ) -> Result<StudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting assignment update");
        println!("[DEBUG] StudentFormAssignmentService: Update request: {:?}", request);

        let assignment = self.dao
            .update_student_form_assignment(&request)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Assignment updated successfully");
        Ok(assignment.into())
    }

    pub async fn delete_student_form_assignment(
        &self,
        assignment_id: Uuid,
        school_id: Uuid,
    ) -> Result<DeleteStudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting assignment deletion");
        println!("[DEBUG] StudentFormAssignmentService: Assignment ID: {}, School ID: {}", assignment_id, school_id);

        self.dao
            .delete_student_form_assignment(assignment_id, school_id)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Assignment deleted successfully");
        Ok(DeleteStudentFormAssignmentResponse {
            message: "Student form assignment successfully deleted".to_string(),
            assignment_id,
            school_id,
        })
    }

    pub async fn review_student_form_assignment(
        &self,
        request: ReviewStudentFormAssignmentRequest,
    ) -> Result<ReviewStudentFormAssignmentResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting assignment review");
        println!("[DEBUG] StudentFormAssignmentService: Review request: {:?}", request);

        // Validate that the status is either Approved or Rejected
        match request.status {
            crate::models::student_form_assignment::StudentFormAssignmentStatus::Approved |
            crate::models::student_form_assignment::StudentFormAssignmentStatus::Rejected => {
                // Status is valid for review
            }
            _ => {
                println!("[ERROR] StudentFormAssignmentService: Invalid review status: {:?}", request.status);
                return Err(AppError::Validation("Review status must be 'approved' or 'rejected'".to_string()));
            }
        }

        let response = self.dao
            .review_student_form_assignment(&request)
            .await?;

        println!("[DEBUG] StudentFormAssignmentService: Assignment reviewed successfully");
        Ok(response)
    }

    pub async fn validate_api_key(&self, api_key: &str) -> Result<(), AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Validating API key");

        // Use the same API key validation as other endpoints
        let expected_api_key = match std::env::var("OWNER_API_KEY") {
            Ok(key) => {
                println!("[DEBUG] StudentFormAssignmentService: OWNER_API_KEY found");
                key
            }
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentService: OWNER_API_KEY not configured: {:?}", e);
                return Err(AppError::Internal("OWNER_API_KEY not configured".to_string()));
            }
        };

        if api_key != expected_api_key {
            println!("[ERROR] StudentFormAssignmentService: API key mismatch");
            return Err(AppError::Authentication("Invalid API key".to_string()));
        }

        println!("[DEBUG] StudentFormAssignmentService: API key validation successful");
        Ok(())
    }

    pub async fn bulk_assign_forms(
        &self,
        request: BulkAssignFormRequest,
    ) -> Result<BulkAssignFormResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Starting bulk form assignment");
        println!("[DEBUG] StudentFormAssignmentService: School ID: {}, Number of assignments: {}",
                 request.school_id, request.assignments.len());

        // Validate request has at least one assignment
        if request.assignments.is_empty() {
            println!("[ERROR] StudentFormAssignmentService: No assignments provided");
            return Err(AppError::Validation("At least one assignment is required".to_string()));
        }

        // Extract unique form template IDs for validation
        let form_template_ids: Vec<Uuid> = request.assignments
            .iter()
            .map(|a| a.form_template_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        println!("[DEBUG] StudentFormAssignmentService: Validating {} unique form templates", form_template_ids.len());

        // Step 1: Validate all form templates are active
        match self.dao.validate_form_templates_active(&form_template_ids).await {
            Ok(_) => println!("[DEBUG] StudentFormAssignmentService: All form templates validated as active"),
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentService: Form template validation failed: {:?}", e);
                return Err(e);
            }
        }

        // Step 2: Check for duplicate assignments
        match self.dao.check_duplicate_assignments(request.school_id, &request.assignments).await {
            Ok(_) => println!("[DEBUG] StudentFormAssignmentService: No duplicate assignments found"),
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentService: Duplicate assignment check failed: {:?}", e);
                return Err(e);
            }
        }

        // Step 3: Create assignments in bulk (within transaction)
        match self.dao.bulk_create_assignments(request.school_id, request.assignments).await {
            Ok(created_assignments) => {
                println!("[DEBUG] StudentFormAssignmentService: Successfully created {} assignments", created_assignments.len());

                let successful: Vec<StudentFormAssignmentResponse> = created_assignments
                    .into_iter()
                    .map(|a| a.into())
                    .collect();

                Ok(BulkAssignFormResponse {
                    successful,
                    failed: Vec::new(), // No failures in current implementation
                })
            }
            Err(e) => {
                println!("[ERROR] StudentFormAssignmentService: Bulk creation failed: {:?}", e);
                Err(e)
            }
        }
    }

    /// Assign a form template to all active students in a school
    pub async fn assign_form_to_school_students(
        &self,
        request: crate::models::student_form_assignment::AssignFormToSchoolStudentsRequest,
    ) -> Result<crate::models::student_form_assignment::AssignFormToSchoolStudentsResponse, AppError> {
        println!("[DEBUG] StudentFormAssignmentService: Assigning form {} to all active students in school {}",
            request.form_template_id, request.school_id);

        // Validate that the form template is active
        let form_template_ids = vec![request.form_template_id];
        self.dao.validate_form_templates_active(&form_template_ids).await?;
        println!("[DEBUG] StudentFormAssignmentService: Form template validated");

        // Call DAO to assign forms to all active students
        let is_required = request.is_required.unwrap_or(false);
        let (created_assignments, total_active_students, students_already_assigned) = self.dao
            .assign_form_to_school_students(
                request.school_id,
                request.form_template_id,
                is_required,
            )
            .await?;

        // Convert to response DTOs
        let successful: Vec<crate::models::student_form_assignment::StudentFormAssignmentResponse> =
            created_assignments.into_iter()
                .map(|assignment| assignment.into())
                .collect();

        let newly_assigned = successful.len() as i64;

        println!("[DEBUG] StudentFormAssignmentService: Assignment complete. Total: {}, Already assigned: {}, Newly assigned: {}",
            total_active_students, students_already_assigned, newly_assigned);

        Ok(crate::models::student_form_assignment::AssignFormToSchoolStudentsResponse {
            school_id: request.school_id,
            form_template_id: request.form_template_id,
            total_active_students,
            students_already_assigned,
            newly_assigned,
            failed_assignments: 0,
            successful,
            failed: Vec::new(),
        })
    }
}