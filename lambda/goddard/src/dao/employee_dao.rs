use deadpool_postgres::Pool;
use uuid::Uuid;
use crate::models::employee::{Employee, EmployeeWithUser, UpdateEmployeeRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct EmployeeDao {
    pool: Pool,
}

impl EmployeeDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create_employee(
        &self,
        user_id: Uuid,
        school_id: Uuid,
        phone: Option<&str>,
        address: Option<&str>,
        employee_type: Option<&str>,
        joined_on: Option<chrono::NaiveDate>,
    ) -> Result<Employee, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "INSERT INTO employees (id, user_id, school_id, phone, address, employee_type, joined_on, is_active, created_at, updated_at)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, true, NOW(), NOW())
             RETURNING id, user_id, school_id, phone, address, employee_type, joined_on, is_active, created_at, updated_at",
            &[&user_id, &school_id, &phone, &address, &employee_type, &joined_on],
        ).await.map_err(|e| AppError::Database(format!("Failed to create employee: {}", e)))?;

        Ok(self.row_to_employee(&row))
    }

    pub async fn get_employees_by_school(&self, school_id: Uuid) -> Result<Vec<EmployeeWithUser>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "SELECT e.id, e.user_id, e.school_id, u.first_name, u.last_name, u.email,
                    e.phone, e.address, e.employee_type, e.joined_on,
                    e.is_active, u.is_verified, e.created_at
             FROM employees e
             JOIN users u ON e.user_id = u.id
             WHERE e.school_id = $1 AND (e.is_active = true OR e.is_active IS NULL)
             ORDER BY u.first_name ASC",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch employees: {}", e)))?;

        Ok(rows.iter().map(|r| self.row_to_employee_with_user(r)).collect())
    }

    pub async fn get_employee_by_id(&self, employee_id: Uuid, school_id: Uuid) -> Result<Option<EmployeeWithUser>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT e.id, e.user_id, e.school_id, u.first_name, u.last_name, u.email,
                    e.phone, e.address, e.employee_type, e.joined_on,
                    e.is_active, u.is_verified, e.created_at
             FROM employees e
             JOIN users u ON e.user_id = u.id
             WHERE e.id = $1 AND e.school_id = $2",
            &[&employee_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch employee: {}", e)))?;

        Ok(row.map(|r| self.row_to_employee_with_user(&r)))
    }

    pub async fn get_employee_by_user_id(&self, user_id: Uuid, school_id: Uuid) -> Result<Option<Employee>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT id, user_id, school_id, phone, address, employee_type, joined_on, is_active, created_at, updated_at
             FROM employees WHERE user_id = $1 AND school_id = $2",
            &[&user_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch employee by user_id: {}", e)))?;

        Ok(row.map(|r| self.row_to_employee(&r)))
    }

    pub async fn update_employee(&self, employee_id: Uuid, school_id: Uuid, req: &UpdateEmployeeRequest) -> Result<Employee, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "UPDATE employees SET phone = COALESCE($3, phone), address = COALESCE($4, address),
             employee_type = COALESCE($5, employee_type), joined_on = COALESCE($6, joined_on), updated_at = NOW()
             WHERE id = $1 AND school_id = $2
             RETURNING id, user_id, school_id, phone, address, employee_type, joined_on, is_active, created_at, updated_at",
            &[&employee_id, &school_id, &req.phone, &req.address, &req.employee_type, &req.joined_on],
        ).await.map_err(|e| AppError::Database(format!("Failed to update employee: {}", e)))?;

        Ok(self.row_to_employee(&row))
    }

    pub async fn deactivate_employee(&self, employee_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let n = client.execute(
            "UPDATE employees SET is_active = false, updated_at = NOW() WHERE id = $1 AND school_id = $2",
            &[&employee_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to deactivate employee: {}", e)))?;

        if n == 0 { return Err(AppError::NotFound("Employee not found".to_string())); }
        Ok(())
    }

    pub async fn get_employee_by_user_id_with_user(&self, user_id: Uuid, school_id: Uuid) -> Result<Option<EmployeeWithUser>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT e.id, e.user_id, e.school_id, u.first_name, u.last_name, u.email,
                    e.phone, e.address, e.employee_type, e.joined_on,
                    e.is_active, u.is_verified, e.created_at
             FROM employees e
             JOIN users u ON e.user_id = u.id
             WHERE e.user_id = $1 AND e.school_id = $2",
            &[&user_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch employee by user_id: {}", e)))?;

        Ok(row.map(|r| self.row_to_employee_with_user(&r)))
    }

    pub async fn get_employee_by_email_and_school(&self, email: &str, school_id: Uuid) -> Result<Option<Employee>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT e.id, e.user_id, e.school_id, e.phone, e.address, e.employee_type, e.joined_on, e.is_active, e.created_at, e.updated_at
             FROM employees e
             JOIN users u ON e.user_id = u.id
             WHERE u.email = $1 AND e.school_id = $2
             LIMIT 1",
            &[&email, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch employee by email: {}", e)))?;

        Ok(row.map(|r| self.row_to_employee(&r)))
    }

    pub async fn activate_employee(&self, employee_id: Uuid, school_id: Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let n = client.execute(
            "UPDATE employees SET is_active = true, updated_at = NOW() WHERE id = $1 AND school_id = $2",
            &[&employee_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to activate employee: {}", e)))?;

        if n == 0 { return Err(AppError::NotFound("Employee not found".to_string())); }
        Ok(())
    }

    fn row_to_employee(&self, row: &tokio_postgres::Row) -> Employee {
        Employee {
            id: row.get("id"),
            user_id: row.get("user_id"),
            school_id: row.get("school_id"),
            phone: row.get("phone"),
            address: row.get("address"),
            employee_type: row.get("employee_type"),
            joined_on: row.get("joined_on"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_employee_with_user(&self, row: &tokio_postgres::Row) -> EmployeeWithUser {
        EmployeeWithUser {
            id: row.get("id"),
            user_id: row.get("user_id"),
            school_id: row.get("school_id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            email: row.get("email"),
            phone: row.get("phone"),
            address: row.get("address"),
            employee_type: row.get("employee_type"),
            joined_on: row.get("joined_on"),
            is_active: row.get("is_active"),
            is_verified: row.get("is_verified"),
            created_at: row.get("created_at"),
        }
    }
}
