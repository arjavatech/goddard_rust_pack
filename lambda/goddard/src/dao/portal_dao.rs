use uuid::Uuid;
use chrono::{NaiveDate, NaiveDateTime};
use deadpool_postgres::Pool;
use tokio_postgres::Row;

use crate::{
    error::AppError,
    controllers::portal_controller::{
        EnrollmentProgress, PhoneNumbers, MailingAddress,
        LinkedChild, ParentProfileResponse, GuardianReference,
        ChildDemographicsResponse,
    },
};

// =============================
// DAO Models
// =============================

pub struct UserDetails {
    pub parent_id: Option<Uuid>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

pub struct ChildData {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub dob: NaiveDate,
    pub age: i32,
    pub gender: Option<String>,
    pub class_name: String,
    pub enrollment_id: Option<Uuid>,
}

pub struct ChildForm {
    pub assignment_id: Uuid,
    pub form_template_id: Uuid,
    pub title: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub last_updated: chrono::NaiveDateTime,
    pub launch_url: Option<String>,
}

pub struct ClassroomData {
    pub id: Uuid,
    pub name: String,
    pub capacity: Option<i32>,
    pub age_group: Option<String>,
    pub teachers: Vec<String>,
    pub notes: Option<String>,
    pub enrolled_count: i64,
    pub available_spots: i32,
}

pub struct ClassroomForm {
    pub form_template_id: Uuid,
    pub title: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub assigned_by: String,
    pub assigned_at: chrono::NaiveDateTime,
}

pub struct ClassroomFormAssignment {
    pub id: Uuid,
    pub classroom_id: Uuid,
    pub form_template_id: Uuid,
    pub due_date: Option<NaiveDate>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct PortalDao {
    pool: Pool,
}

impl PortalDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    // =============================
    // User & Parent Queries
    // =============================

    pub async fn get_user_details(&self, user_id: &Uuid) -> Result<UserDetails, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        let query = "
            SELECT p.id as parent_id, p.first_name, p.last_name
            FROM users u
            LEFT JOIN parents p ON u.id = p.user_id
            WHERE u.id = $1
        ";

        let row = client.query_one(query, &[user_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get user details: {}", e)))?;

        Ok(UserDetails {
            parent_id: row.get("parent_id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
        })
    }

    pub async fn get_parent_children(&self, parent_id: &Uuid) -> Result<Vec<ChildData>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        let query = "
            SELECT
                c.id as child_id,
                c.first_name,
                c.last_name,
                c.birth_date as dob,
                DATE_PART('year', AGE(c.birth_date))::int as age,
                c.gender,
                COALESCE(cl.name, 'Not Enrolled') as class_name,
                e.id as enrollment_id
            FROM children c
            LEFT JOIN enrollments e ON c.id = e.child_id AND (e.is_active = true OR e.is_active IS NULL)
            LEFT JOIN classrooms cl ON e.classroom_id = cl.id
            WHERE c.parent_id = $1
                AND (c.is_active = true OR c.is_active IS NULL)
            ORDER BY c.first_name
        ";

        let rows = client.query(query, &[parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get children: {}", e)))?;

        let children: Vec<ChildData> = rows.iter().map(|row| {
            ChildData {
                id: row.get("child_id"),
                first_name: row.get("first_name"),
                last_name: row.get("last_name"),
                dob: row.get("dob"),
                age: row.get("age"),
                gender: row.get("gender"),
                class_name: row.get("class_name"),
                enrollment_id: row.get("enrollment_id"),
            }
        }).collect();

        Ok(children)
    }

    pub async fn get_child_profile(&self, parent_id: &Uuid, child_id: &Uuid) -> Result<ChildData, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        let query = "
            SELECT
                c.id,
                c.first_name,
                c.last_name,
                c.gender,
                c.birth_date as dob,
                DATE_PART('year', AGE(c.birth_date))::int as age,
                cl.name as class_name,
                e.id as enrollment_id
            FROM children c
            LEFT JOIN enrollments e ON c.id = e.child_id AND (e.is_active = true OR e.is_active IS NULL)
            LEFT JOIN classrooms cl ON e.classroom_id = cl.id
            WHERE c.id = $1 AND c.parent_id = $2
                AND (c.is_active = true OR c.is_active IS NULL)
        ";

        let row = client.query_one(query, &[child_id, parent_id]).await
            .map_err(|e| AppError::Database(format!("Child not found or access denied: {}", e)))?;

        Ok(ChildData {
            id: row.get("id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            dob: row.get("dob"),
            age: row.get("age"),
            gender: row.get("gender"),
            class_name: row.get("class_name"),
            enrollment_id: row.get("enrollment_id"),
        })
    }

    pub async fn get_child_form_stats(&self, child_id: &Uuid) -> Result<EnrollmentProgress, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        let query = "
            SELECT
                COUNT(*) as total_forms,
                COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_forms
            FROM student_form_assignments
            WHERE child_id = $1
                AND (is_active = true OR is_active IS NULL)
        ";

        let row = client.query_one(query, &[child_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get form stats: {}", e)))?;

        let total_forms: i64 = row.get("total_forms");
        let completed_forms: i64 = row.get("completed_forms");
        let completion_percentage = if total_forms > 0 {
            ((completed_forms as f64 / total_forms as f64) * 100.0) as i32
        } else {
            0
        };

        Ok(EnrollmentProgress {
            total_forms,
            completed_forms,
            completion_percentage,
        })
    }

    pub async fn verify_parent_child_relationship(&self, parent_id: &Uuid, child_id: &Uuid) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        let query = "
            SELECT 1 FROM children
            WHERE id = $1 AND (parent_id = $2 OR secondary_parent_id = $2)
                AND (is_active = true OR is_active IS NULL)
        ";

        client.query_one(query, &[child_id, parent_id]).await
            .map_err(|_| AppError::Authorization("Parent-child relationship not found".to_string()))?;

        Ok(())
    }

    pub async fn get_child_forms(&self, child_id: &Uuid) -> Result<Vec<ChildForm>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        let query = "
            SELECT
                sfa.id as assignment_id,
                sfa.form_template_id,
                ft.form_name as title,
                sfa.status,
                NULL::date as due_date,
                COALESCE(sfa.updated_at, sfa.created_at) as last_updated,
                ft.fillout_form_url as launch_url
            FROM student_form_assignments sfa
            JOIN form_templates ft ON sfa.form_template_id = ft.id
            WHERE sfa.child_id = $1
                AND (sfa.is_active = true OR sfa.is_active IS NULL)
            ORDER BY sfa.created_at DESC
        ";

        let rows = client.query(query, &[child_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get child forms: {}", e)))?;

        let forms: Vec<ChildForm> = rows.iter().map(|row| {
            ChildForm {
                assignment_id: row.get("assignment_id"),
                form_template_id: row.get("form_template_id"),
                title: row.get("title"),
                status: row.get("status"),
                due_date: row.get("due_date"),
                last_updated: row.get("last_updated"),
                launch_url: row.get("launch_url"),
            }
        }).collect();

        Ok(forms)
    }

    // =============================
    // Classroom Queries
    // =============================

    pub async fn get_classroom_details(&self, classroom_id: &Uuid) -> Result<ClassroomData, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        let query = "
            SELECT
                c.id,
                c.name,
                c.capacity,
                c.age_group,
                NULL as notes,
                ARRAY[]::text[] as teachers,
                COUNT(e.id) as enrolled_count
            FROM classrooms c
            LEFT JOIN enrollments e ON c.id = e.classroom_id
                AND e.status = 'active'
                AND (e.is_active = true OR e.is_active IS NULL)
            WHERE c.id = $1
                AND (c.is_active = true OR c.is_active IS NULL)
            GROUP BY c.id, c.name, c.capacity, c.age_group
        ";

        let row = client.query_one(query, &[classroom_id]).await
            .map_err(|e| AppError::Database(format!("Classroom not found: {}", e)))?;

        let capacity: Option<i32> = row.get("capacity");
        let enrolled_count: i64 = row.get("enrolled_count");
        let available_spots = capacity.map_or(0, |cap| cap - enrolled_count as i32);

        Ok(ClassroomData {
            id: row.get("id"),
            name: row.get("name"),
            capacity,
            age_group: row.get("age_group"),
            teachers: row.get("teachers"),
            notes: row.get("notes"),
            enrolled_count,
            available_spots,
        })
    }

    pub async fn get_classroom_forms(&self, classroom_id: &Uuid) -> Result<Vec<ClassroomForm>, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        // First get school_id from classroom
        let school_query = "SELECT school_id FROM classrooms WHERE id = $1";
        let school_row = client.query_one(school_query, &[classroom_id]).await
            .map_err(|e| AppError::Database(format!("Classroom not found: {}", e)))?;
        let school_id: Uuid = school_row.get("school_id");

        let query = "
            SELECT
                cfo.form_template_id,
                ft.form_name AS title,
                'active' AS status,
                NULL::date AS due_date,
                'classroom' AS assigned_by,
                cfo.created_at AS assigned_at
            FROM class_form_overrides cfo
            JOIN form_templates ft ON cfo.form_template_id = ft.id
            WHERE cfo.classroom_id = $1
                AND (cfo.is_active = true OR cfo.is_active IS NULL)
        ";

        let rows = client.query(query, &[classroom_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get classroom forms: {}", e)))?;

        let forms: Vec<ClassroomForm> = rows.iter().map(|row| {
            ClassroomForm {
                form_template_id: row.get("form_template_id"),
                title: row.get("title"),
                status: row.get("status"),
                due_date: row.get("due_date"),
                assigned_by: row.get("assigned_by"),
                assigned_at: row.get("assigned_at"),
            }
        }).collect();

        Ok(forms)
    }

    pub async fn assign_classroom_form(
        &self,
        classroom_id: &Uuid,
        form_template_id: &Uuid,
        due_date: Option<NaiveDate>,
        notes: Option<String>,
        assigned_by: &str
    ) -> Result<ClassroomFormAssignment, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        // First get school_id from classroom
        let school_query = "SELECT school_id FROM classrooms WHERE id = $1";
        let school_row = client.query_one(school_query, &[classroom_id]).await
            .map_err(|e| AppError::Database(format!("Classroom not found: {}", e)))?;
        let school_id: Uuid = school_row.get("school_id");

        let query = "
            INSERT INTO class_form_overrides
                (id, school_id, classroom_id, form_template_id, action, is_required, created_at)
            VALUES
                (gen_random_uuid(), $1, $2, $3, 'include', true, NOW())
            RETURNING id, classroom_id, form_template_id, created_at
        ";

        let row = client.query_one(
            query,
            &[&school_id, classroom_id, form_template_id]
        ).await
        .map_err(|e| AppError::Database(format!("Failed to assign form: {}", e)))?;

        Ok(ClassroomFormAssignment {
            id: row.get("id"),
            classroom_id: row.get("classroom_id"),
            form_template_id: row.get("form_template_id"),
            due_date: due_date,
            created_at: row.get("created_at"),
        })
    }

    pub async fn remove_classroom_form(
        &self,
        classroom_id: &Uuid,
        form_id: &Uuid
    ) -> Result<(), AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        let query = "
            UPDATE class_form_overrides
            SET is_active = false, updated_at = NOW()
            WHERE classroom_id = $1 AND form_template_id = $2
        ";

        let rows_affected = client.execute(query, &[classroom_id, form_id]).await
            .map_err(|e| AppError::Database(format!("Failed to remove form: {}", e)))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound("Form assignment not found".to_string()));
        }

        Ok(())
    }

    // =============================
    // Parent Profile Queries
    // =============================

    pub async fn get_parent_profile(&self, parent_id: &Uuid) -> Result<ParentProfileResponse, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        // Get parent info from users table
        let parent_query = "
            SELECT
                u.id,
                u.first_name,
                u.last_name,
                u.email,
                NULL as primary_phone,
                NULL as secondary_phone,
                NULL as address_street,
                NULL as address_city,
                NULL as address_state,
                NULL as address_zip
            FROM users u
            WHERE u.id = $1 AND u.role IN ('Parent', 'primary-parent', 'secondary-parent')
        ";

        let parent_row = client.query_one(parent_query, &[parent_id]).await
            .map_err(|e| AppError::Database(format!("Parent not found: {}", e)))?;

        // Get linked children
        let children_query = "
            SELECT
                c.id as child_id,
                c.first_name,
                c.last_name,
                cl.name as classroom
            FROM children c
            LEFT JOIN enrollments e ON c.id = e.child_id AND e.status = 'active'
            LEFT JOIN classrooms cl ON e.classroom_id = cl.id
            WHERE c.parent_id = $1
                AND (c.is_active = true OR c.is_active IS NULL)
        ";

        let children_rows = client.query(children_query, &[parent_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get children: {}", e)))?;

        let linked_children: Vec<LinkedChild> = children_rows.iter().map(|row| {
            LinkedChild {
                child_id: row.get("child_id"),
                first_name: row.get("first_name"),
                last_name: row.get("last_name"),
                classroom: row.get("classroom"),
            }
        }).collect();

        Ok(ParentProfileResponse {
            id: parent_row.get("id"),
            first_name: parent_row.get("first_name"),
            last_name: parent_row.get("last_name"),
            email: parent_row.get("email"),
            phone_numbers: PhoneNumbers {
                primary: parent_row.get("primary_phone"),
                secondary: parent_row.get("secondary_phone"),
            },
            mailing_address: MailingAddress {
                street: parent_row.get("address_street"),
                city: parent_row.get("address_city"),
                state: parent_row.get("address_state"),
                zip: parent_row.get("address_zip"),
            },
            linked_children,
        })
    }

    // =============================
    // Child Demographics Queries
    // =============================

    pub async fn get_child_demographics(&self, child_id: &Uuid) -> Result<ChildDemographicsResponse, AppError> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Connection pool error: {}", e)))?;

        // Get child info with classroom
        let child_query = "
            SELECT
                c.id,
                c.first_name,
                c.last_name,
                c.birth_date as dob,
                c.gender,
                DATE_PART('year', AGE(c.birth_date))::int as age,
                e.classroom_id,
                cl.name as classroom_name
            FROM children c
            LEFT JOIN enrollments e ON c.id = e.child_id AND e.status = 'active'
            LEFT JOIN classrooms cl ON e.classroom_id = cl.id
            WHERE c.id = $1
        ";

        let child_row = client.query_one(child_query, &[child_id]).await
            .map_err(|e| AppError::Database(format!("Child not found: {}", e)))?;

        // Get guardian references - using children table parent relationships
        let guardian_query = "
            SELECT
                u.id as parent_id,
                'primary' as relationship,
                CONCAT(u.first_name, ' ', u.last_name) as name
            FROM children c
            JOIN users u ON c.parent_id = u.id
            WHERE c.id = $1

            UNION ALL

            SELECT
                u.id as parent_id,
                'secondary' as relationship,
                CONCAT(u.first_name, ' ', u.last_name) as name
            FROM children c
            JOIN users u ON c.secondary_parent_id = u.id
            WHERE c.id = $1 AND c.secondary_parent_id IS NOT NULL
        ";

        let guardian_rows = client.query(guardian_query, &[child_id]).await
            .map_err(|e| AppError::Database(format!("Failed to get guardians: {}", e)))?;

        let guardian_references: Vec<GuardianReference> = guardian_rows.iter().map(|row| {
            GuardianReference {
                parent_id: row.get("parent_id"),
                relationship: row.get("relationship"),
                name: row.get("name"),
            }
        }).collect();

        Ok(ChildDemographicsResponse {
            id: child_row.get("id"),
            first_name: child_row.get("first_name"),
            last_name: child_row.get("last_name"),
            dob: child_row.get("dob"),
            age: child_row.get("age"),
            classroom_id: child_row.get("classroom_id"),
            classroom_name: child_row.get("classroom_name"),
            guardian_references,
        })
    }
}