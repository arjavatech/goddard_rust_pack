use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Request DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TargetCountQuery {
    pub school_id: Uuid,
    pub audience: String,
    pub classroom_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSurveyRequest {
    pub school_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub audience: String,
    pub classroom_id: Option<Uuid>,
    pub is_anonymous: bool,
    pub is_template: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub questions: Vec<QuestionInput>,
    /// "draft" or "published" — defaults to "draft"
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuestionInput {
    pub question_type: String,
    pub question_text: String,
    pub is_required: bool,
    pub options: Option<Vec<String>>,
    pub rating_max: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSurveyRequest {
    pub id: Uuid,
    pub title: Option<String>,
    pub description: Option<String>,
    pub audience: Option<String>,
    pub classroom_id: Option<Uuid>,
    pub is_anonymous: Option<bool>,
    pub is_template: Option<bool>,
    pub expires_at: Option<DateTime<Utc>>,
    pub questions: Option<Vec<QuestionInput>>,
}

#[derive(Debug, Deserialize)]
pub struct SurveyListQuery {
    pub school_id: Uuid,
    pub status: Option<String>,
    pub audience: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MySurveysQuery {
    pub school_id: Uuid,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitResponseRequest {
    pub school_id: Uuid,
    pub answers: Vec<AnswerInput>,
}

#[derive(Debug, Deserialize)]
pub struct AnonymousSubmitRequest {
    pub answers: Vec<AnswerInput>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnswerInput {
    pub question_id: Uuid,
    pub value: Option<String>,
    pub values: Option<Vec<String>>,
}

// ── Response DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct SurveyListItem {
    pub id: Uuid,
    pub school_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub audience: String,
    pub classroom_id: Option<Uuid>,
    pub classroom_name: Option<String>,
    pub status: String,
    pub is_anonymous: bool,
    pub is_template: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by_name: Option<String>,
    pub total_targeted: i32,
    pub total_responded: i64,
    pub question_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SurveyQuestionDto {
    pub id: Uuid,
    pub question_type: String,
    pub question_text: String,
    pub is_required: bool,
    pub options: Option<Vec<String>>,
    pub rating_max: Option<i32>,
    pub sort_order: i32,
}

#[derive(Debug, Serialize)]
pub struct SurveyDetail {
    #[serde(flatten)]
    pub survey: SurveyListItem,
    pub questions: Vec<SurveyQuestionDto>,
    pub has_responded: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MySurveyItem {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub audience: String,
    pub status: String,
    pub is_anonymous: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub question_count: i64,
    pub has_responded: bool,
    pub submitted_at: Option<DateTime<Utc>>,
}

// ── Report DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SurveyReportDto {
    #[serde(flatten)]
    pub survey: SurveyListItem,
    pub question_results: Vec<QuestionResultDto>,
    pub individual_responses: Vec<IndividualResponseDto>,
}

#[derive(Debug, Serialize)]
pub struct QuestionResultDto {
    pub question_id: Uuid,
    pub question_text: String,
    pub question_type: String,
    pub total_answered: i64,
    pub choice_results: Option<Vec<ChoiceResult>>,
    pub average_rating: Option<f64>,
    pub rating_distribution: Option<Vec<RatingBucket>>,
    pub text_responses: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ChoiceResult {
    pub option: String,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct RatingBucket {
    pub star: i32,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct IndividualResponseDto {
    pub response_id: Uuid,
    pub respondent_name: Option<String>,
    pub respondent_email: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub answers: Vec<ResponseAnswerDto>,
}

#[derive(Debug, Serialize)]
pub struct ResponseAnswerDto {
    pub question_id: Uuid,
    pub question_text: String,
    pub question_type: String,
    pub value: Option<String>,
    pub values: Option<Vec<String>>,
}
