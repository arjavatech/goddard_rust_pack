use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilloutSubmissionResponse {
    pub submission: FilloutSubmission,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FilloutSubmission {
    pub submission_id: String,
    pub submission_time: DateTime<Utc>,
    pub last_updated_at: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub questions: Vec<FilloutQuestion>,
    pub calculations: Vec<serde_json::Value>,
    pub url_parameters: Vec<FilloutUrlParameter>,
    pub quiz: serde_json::Value,
    pub documents: Vec<FilloutDocument>,
    pub scheduling: Vec<serde_json::Value>,
    pub payments: Vec<serde_json::Value>,
    #[serde(rename = "editLink")]
    pub edit_link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilloutQuestion {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub question_type: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilloutUrlParameter {
    pub id: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilloutDocument {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilloutErrorResponse {
    pub error: String,
    pub message: Option<String>,
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct FilloutSubmissionDetails {
    pub edit_link: Option<String>,
    pub pdf_link: Option<String>,
    pub submission_id: String,
}

impl From<FilloutSubmissionResponse> for FilloutSubmissionDetails {
    fn from(response: FilloutSubmissionResponse) -> Self {
        let pdf_link = response
            .submission
            .documents
            .first()
            .map(|doc| doc.url.clone());

        Self {
            edit_link: response.submission.edit_link,
            pdf_link,
            submission_id: response.submission.submission_id,
        }
    }
}