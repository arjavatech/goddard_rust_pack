use crate::dao::StudentFormAssignmentDao;
use crate::models::student_form_assignment::{
    StudentFormAssignment, StudentFormAssignmentResponse, CreateStudentFormAssignmentRequest,
    UpdateStudentFormAssignmentRequest, DeleteStudentFormAssignmentResponse
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
}