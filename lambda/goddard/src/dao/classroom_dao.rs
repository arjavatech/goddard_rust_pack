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

        let query = r#"
            INSERT INTO classrooms (id, school_id, name, age_group, capacity, enrolled_count, is_active, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, null, null, 0, true, NOW(), NOW())
            RETURNING id, school_id, name, age_group, capacity, enrolled_count, is_active,
                     created_at, updated_at
        "#;

        let row = client.query_one(query, &[&request.school_id, &request.class_name])
            .await
            .map_err(|e| AppError::Database(format!("Failed to create classroom: {}", e)))?;

        Ok(Self::row_to_classroom(&row))
    }

    pub async fn get_classrooms_by_school(&self, school_id: &Uuid) -> Result<Vec<Classroom>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT id, name, school_id, age_group, capacity, enrolled_count, is_active,
                   created_at, updated_at
            FROM classrooms
            WHERE school_id = $1 AND (is_active = true OR is_active IS NULL)
            ORDER BY name ASC
        "#;

        let rows = client.query(query, &[school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch classrooms: {}", e)))?;

        let classrooms = rows.iter().map(Self::row_to_classroom).collect();
        Ok(classrooms)
    }

    pub async fn update_classroom(&self, request: &UpdateClassroomRequest) -> Result<Classroom, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE classrooms
            SET name = $3, updated_at = NOW()
            WHERE id = $2 AND school_id = $1 AND (is_active = true OR is_active IS NULL)
            RETURNING id, school_id, name, age_group, capacity, enrolled_count, is_active,
                     created_at, updated_at
        "#;

        let rows = client.query(query, &[&request.school_id, &request.class_id, &request.class_name])
            .await
            .map_err(|e| AppError::Database(format!("Failed to update classroom: {}", e)))?;

        if rows.is_empty() {
            return Err(AppError::NotFound("Classroom not found".to_string()));
        }

        Ok(Self::row_to_classroom(&rows[0]))
    }

    pub async fn delete_classroom(&self, classroom_id: &Uuid, school_id: &Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            UPDATE classrooms
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND school_id = $2
        "#;

        let rows_affected = client.execute(query, &[classroom_id, school_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete classroom: {}", e)))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Classroom not found".to_string()));
        }

        Ok(())
    }
}