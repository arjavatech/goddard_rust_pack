use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;
use crate::models::classroom::{Classroom, CreateClassroomRequest, UpdateClassroomRequest};
use crate::error::error_types::AppError;

#[derive(Clone)]
pub struct ClassroomDao {
    pool: Pool,
}

impl ClassroomDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_classroom(row: &Row) -> Classroom {
        Classroom {
            id: row.get("id"),
            school_id: row.get("school_id"),
            name: row.get("name"),
            age_group: row.get("age_group"),
            capacity: row.get("capacity"),
            enrolled_count: row.get("enrolled_count"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    pub async fn create_classroom(&self, request: &CreateClassroomRequest) -> Result<Classroom, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let stmt = client.prepare(
            r#"
            INSERT INTO classrooms (id, school_id, name, age_group, capacity, enrolled_count, is_active, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, null, null, 0, true, NOW(), NOW())
            RETURNING id, school_id, name, age_group, capacity, enrolled_count, is_active,
                     created_at, updated_at
            "#
        ).await
        .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let row = client.query_one(&stmt, &[&request.school_id, &request.class_name])
            .await
            .map_err(|e| AppError::Database(format!("Failed to create classroom: {}", e)))?;

        Ok(Self::row_to_classroom(&row))
    }

    pub async fn get_classrooms_by_school(&self, school_id: &Uuid) -> Result<Vec<Classroom>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let stmt = client.prepare(
            r#"
            SELECT id, name, school_id, age_group, capacity, enrolled_count, is_active,
                   created_at, updated_at
            FROM classrooms
            WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
            ORDER BY name ASC
            "#
        ).await
        .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows = client.query(&stmt, &[school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch classrooms: {}", e)))?;

        let classrooms = rows.iter().map(Self::row_to_classroom).collect();
        Ok(classrooms)
    }

    pub async fn update_classroom(&self, request: &UpdateClassroomRequest) -> Result<Classroom, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // Build the SQL query with escaped values to avoid prepared statement conflicts
        let query = format!(
            r#"
            UPDATE classrooms
            SET name = '{}', updated_at = NOW()
            WHERE id = '{}' AND school_id = '{}' AND (is_active = true OR is_active IS NULL)
            RETURNING id, school_id, name, age_group, capacity, enrolled_count, is_active,
                     created_at, updated_at
            "#,
            request.class_name.replace('\'', "''"), // SQL escape single quotes
            request.class_id,
            request.school_id
        );

        // Use simple_query to avoid prepared statements entirely
        let result = client.simple_query(&query).await
            .map_err(|e| AppError::Database(format!("Failed to update classroom: {}", e)))?;

        // Parse the result from simple_query
        for message in result {
            if let tokio_postgres::SimpleQueryMessage::Row(row) = message {
                // Extract values from SimpleQueryRow by index since column names might not work
                let classroom = Classroom {
                    id: row.get(0).unwrap().parse().unwrap(),
                    school_id: row.get(1).unwrap().parse().unwrap(),
                    name: row.get(2).unwrap().to_string(),
                    age_group: row.get(3).map(|s| s.to_string()),
                    capacity: row.get(4).and_then(|s| s.parse().ok()),
                    enrolled_count: row.get(5).and_then(|s| s.parse().ok()),
                    is_active: row.get(6).and_then(|s| s.parse().ok()),
                    created_at: row.get(7).and_then(|s| s.parse().ok()),
                    updated_at: row.get(8).and_then(|s| s.parse().ok()),
                };
                return Ok(classroom);
            }
        }
        Err(AppError::NotFound("Classroom not found".to_string()))
    }

    pub async fn name_exists_for_school(&self, name: &str, school_id: &Uuid) -> Result<bool, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM classrooms WHERE LOWER(name) = LOWER($1) AND school_id = $2 AND (is_active = true OR is_active IS NULL))",
            &[&name, school_id],
        ).await
        .map_err(|e| AppError::Database(format!("Failed to check class name: {}", e)))?;

        Ok(row.get(0))
    }

    /// Look up just the classroom name by id (used by notifications to render
    /// "Classroom 'X' deleted" before the delete UPDATE runs).
    pub async fn get_classroom_name(&self, classroom_id: &Uuid) -> Result<Option<String>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;
        let row = client
            .query_opt("SELECT name FROM classrooms WHERE id = $1", &[classroom_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch classroom name: {}", e)))?;
        Ok(row.map(|r| r.get::<_, String>("name")))
    }

    pub async fn has_enrollments(&self, classroom_id: &Uuid) -> Result<bool, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM enrollments WHERE classroom_id = $1)",
            &[classroom_id],
        ).await
        .map_err(|e| AppError::Database(format!("Failed to check enrollments: {}", e)))?;

        Ok(row.get(0))
    }

    pub async fn delete_classroom(&self, classroom_id: &Uuid, school_id: &Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let stmt = client.prepare(
            r#"
            UPDATE classrooms
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND school_id = $2
            "#
        ).await
        .map_err(|e| AppError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows_affected = client.execute(&stmt, &[classroom_id, school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete classroom: {}", e)))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Classroom not found".to_string()));
        }

        Ok(())
    }
}