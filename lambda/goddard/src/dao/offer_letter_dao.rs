use deadpool_postgres::Pool;
use serde_json::Value;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};
use crate::models::offer_letter::OfferLetter;

#[derive(Clone)]
pub struct OfferLetterDao {
    pool: Pool,
}

impl OfferLetterDao {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn row_to_offer_letter(row: &tokio_postgres::Row) -> OfferLetter {
        OfferLetter {
            id: row.get("id"),
            school_id: row.get("school_id"),
            employee_id: row.get("employee_id"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            email: row.get("email"),
            phone: row.get("phone"),
            role: row.get("role"),
            salary: row.get("salary"),
            salary_currency: row.get("salary_currency"),
            joining_date: row.get("joining_date"),
            additional_notes: row.get("additional_notes"),
            custom_fields: row.get("custom_fields"),
            fillout_form_id: row.get("fillout_form_id"),
            fillout_submission_id: row.get("fillout_submission_id"),
            fillout_edit_link: row.get("fillout_edit_link"),
            fillout_pdf_link: row.get("fillout_pdf_link"),
            form_data: row.get("form_data"),
            prefill_url: row.get("prefill_url"),
            status: row.get("status"),
            employee_responded_at: row.get("employee_responded_at"),
            employee_rejection_reason: row.get("employee_rejection_reason"),
            reviewed_by: row.get("reviewed_by"),
            admin_reviewed_at: row.get("admin_reviewed_at"),
            admin_rejection_reason: row.get("admin_rejection_reason"),
            sent_at: row.get("sent_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    pub async fn create(
        &self,
        school_id: Uuid,
        employee_id: Uuid,
        first_name: &str,
        last_name: &str,
        email: &str,
        phone: Option<&str>,
        role: &str,
        salary: Option<f64>,
        salary_currency: Option<&str>,
        joining_date: Option<chrono::NaiveDate>,
        additional_notes: Option<&str>,
        fillout_form_id: Option<&str>,
        custom_fields: Option<&Value>,
    ) -> ApiResult<OfferLetter> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "INSERT INTO offer_letters (school_id, employee_id, first_name, last_name, email, phone, role, salary, salary_currency, joining_date, additional_notes, fillout_form_id, custom_fields, status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'draft', NOW())
             RETURNING *",
            &[&school_id, &employee_id, &first_name, &last_name, &email, &phone, &role,
              &salary, &salary_currency, &joining_date, &additional_notes, &fillout_form_id, &custom_fields],
        ).await.map_err(|e| AppError::Database(format!("Failed to create offer letter: {}", e)))?;

        Ok(Self::row_to_offer_letter(&row))
    }

    pub async fn get_by_employee_id(&self, employee_id: Uuid, school_id: Uuid) -> ApiResult<Option<OfferLetter>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT * FROM offer_letters WHERE employee_id = $1 AND school_id = $2",
            &[&employee_id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch offer letter: {}", e)))?;

        Ok(row.map(|r| Self::row_to_offer_letter(&r)))
    }

    pub async fn get_by_id(&self, id: Uuid, school_id: Uuid) -> ApiResult<Option<OfferLetter>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT * FROM offer_letters WHERE id = $1 AND school_id = $2",
            &[&id, &school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch offer letter: {}", e)))?;

        Ok(row.map(|r| Self::row_to_offer_letter(&r)))
    }

    pub async fn get_by_id_no_school(&self, id: Uuid) -> ApiResult<Option<OfferLetter>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_opt(
            "SELECT * FROM offer_letters WHERE id = $1",
            &[&id],
        ).await.map_err(|e| AppError::Database(format!("Failed to fetch offer letter: {}", e)))?;

        Ok(row.map(|r| Self::row_to_offer_letter(&r)))
    }

    pub async fn update(
        &self,
        employee_id: Uuid,
        school_id: Uuid,
        first_name: Option<&str>,
        last_name: Option<&str>,
        phone: Option<&str>,
        role: Option<&str>,
        salary: Option<f64>,
        salary_currency: Option<&str>,
        joining_date: Option<chrono::NaiveDate>,
        additional_notes: Option<&str>,
        fillout_form_id: Option<&str>,
        custom_fields: Option<&Value>,
    ) -> ApiResult<OfferLetter> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "UPDATE offer_letters
             SET first_name = COALESCE($3, first_name),
                 last_name = COALESCE($4, last_name),
                 phone = COALESCE($5, phone),
                 role = COALESCE($6, role),
                 salary = COALESCE($7, salary),
                 salary_currency = COALESCE($8, salary_currency),
                 joining_date = COALESCE($9, joining_date),
                 additional_notes = COALESCE($10, additional_notes),
                 fillout_form_id = COALESCE($11, fillout_form_id),
                 custom_fields = COALESCE($12, custom_fields),
                 updated_at = NOW()
             WHERE employee_id = $1 AND school_id = $2 AND status = 'draft'
             RETURNING *",
            &[&employee_id, &school_id, &first_name, &last_name, &phone, &role,
              &salary, &salary_currency, &joining_date, &additional_notes, &fillout_form_id, &custom_fields],
        ).await.map_err(|e| AppError::Database(format!("Failed to update offer letter: {}", e)))?;

        Ok(Self::row_to_offer_letter(&row))
    }

    pub async fn set_sent(&self, employee_id: Uuid, school_id: Uuid, prefill_url: &str) -> ApiResult<OfferLetter> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let row = client.query_one(
            "UPDATE offer_letters SET status = 'offer_sent', prefill_url = $3, sent_at = NOW(), updated_at = NOW()
             WHERE employee_id = $1 AND school_id = $2
             RETURNING *",
            &[&employee_id, &school_id, &prefill_url],
        ).await.map_err(|e| AppError::Database(format!("Failed to mark offer letter sent: {}", e)))?;

        Ok(Self::row_to_offer_letter(&row))
    }

    pub async fn set_fillout_submission(
        &self,
        offer_letter_id: Uuid,
        fillout_submission_id: &str,
        pdf_link: Option<&str>,
        edit_link: Option<&str>,
        form_data: Option<&Value>,
        accepted: bool,
        rejection_reason: Option<&str>,
    ) -> ApiResult<OfferLetter> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let new_status = if accepted { "offer_accepted" } else { "offer_rejected" };

        let row = client.query_one(
            "UPDATE offer_letters
             SET fillout_submission_id = $2,
                 fillout_pdf_link = COALESCE($3, fillout_pdf_link),
                 fillout_edit_link = COALESCE($4, fillout_edit_link),
                 form_data = COALESCE($5, form_data),
                 status = $6,
                 employee_responded_at = NOW(),
                 employee_rejection_reason = $7,
                 updated_at = NOW()
             WHERE id = $1
             RETURNING *",
            &[&offer_letter_id, &fillout_submission_id, &pdf_link, &edit_link, &form_data,
              &new_status, &rejection_reason],
        ).await.map_err(|e| AppError::Database(format!("Failed to update offer letter from webhook: {}", e)))?;

        Ok(Self::row_to_offer_letter(&row))
    }

    pub async fn set_admin_review(
        &self,
        employee_id: Uuid,
        school_id: Uuid,
        reviewer_id: Uuid,
        approved: bool,
        rejection_reason: Option<&str>,
    ) -> ApiResult<OfferLetter> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let new_status = if approved { "approved" } else { "offer_rejected" };

        let row = client.query_one(
            "UPDATE offer_letters
             SET status = $3,
                 reviewed_by = $4,
                 admin_reviewed_at = NOW(),
                 admin_rejection_reason = $5,
                 updated_at = NOW()
             WHERE employee_id = $1 AND school_id = $2
             RETURNING *",
            &[&employee_id, &school_id, &new_status, &reviewer_id, &rejection_reason],
        ).await.map_err(|e| AppError::Database(format!("Failed to record admin review: {}", e)))?;

        Ok(Self::row_to_offer_letter(&row))
    }

    pub async fn list_by_school(&self, school_id: Uuid) -> ApiResult<Vec<OfferLetter>> {
        let client = self.pool.get().await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let rows = client.query(
            "SELECT * FROM offer_letters WHERE school_id = $1 ORDER BY created_at DESC",
            &[&school_id],
        ).await.map_err(|e| AppError::Database(format!("Failed to list offer letters: {}", e)))?;

        Ok(rows.iter().map(|r| Self::row_to_offer_letter(r)).collect())
    }
}
