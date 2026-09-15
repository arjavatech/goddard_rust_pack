use crate::models::fillout::{FilloutSubmissionResponse, FilloutSubmissionsListResponse, FilloutSubmissionDetails, FilloutErrorResponse};
use crate::error::AppError;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct FilloutService {
    client: Client,
    api_key: String,
    base_url: String,
}

impl FilloutService {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.fillout.com".to_string()),
        }
    }

    pub async fn fetch_submission_details(
        &self,
        form_id: &str,
        fillout_submission_id: &str,
    ) -> Result<FilloutSubmissionDetails, AppError> {

        let url = format!(
            "{}/v1/api/forms/{}/submissions/{}?includeEditLink=true",
            self.base_url, form_id, fillout_submission_id
        );


        // Retry logic: 5 attempts with 5-second delays
        for attempt in 1..=5 {

            match self.make_api_request(&url).await {
                Ok(response) => {
                    return Ok(response.into());
                }
                Err(e) => {
                    println!("[WARN] FilloutService: Attempt {} failed: {:?}", attempt, e);

                    if attempt < 5 {
                        sleep(Duration::from_secs(5)).await;
                    } else {
                        println!("[ERROR] FilloutService: All attempts failed, returning error");
                        return Err(e);
                    }
                }
            }
        }

        // This should never be reached, but included for completeness
        Err(AppError::ExternalService("All retry attempts exhausted".to_string()))
    }

    async fn make_api_request(&self, url: &str) -> Result<FilloutSubmissionResponse, AppError> {

        let auth_header = format!("Bearer {}", self.api_key);

        let response = self
            .client
            .get(url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| {
                println!("[ERROR] FilloutService: HTTP request failed: {:?}", e);
                AppError::ExternalService(format!("Failed to send request to Fillout API: {}", e))
            })?;

        let status = response.status();

        if status.is_success() {
            let response_text = response.text().await.map_err(|e| {
                println!("[ERROR] FilloutService: Failed to read response body: {:?}", e);
                AppError::ExternalService(format!("Failed to read response from Fillout API: {}", e))
            })?;

            serde_json::from_str::<FilloutSubmissionResponse>(&response_text)
                .map_err(|e| {
                    println!("[ERROR] FilloutService: Failed to parse JSON response: {:?}", e);
                    println!("[ERROR] FilloutService: Raw response: {}", response_text);
                    AppError::ExternalService(format!("Failed to parse Fillout API response: {}", e))
                })
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            println!("[ERROR] FilloutService: API error response ({}): {}", status, error_text);

            // Try to parse as Fillout error response
            if let Ok(fillout_error) = serde_json::from_str::<FilloutErrorResponse>(&error_text) {
                Err(AppError::ExternalService(format!(
                    "Fillout API error ({}): {}",
                    status,
                    fillout_error.message.unwrap_or(fillout_error.error)
                )))
            } else {
                Err(AppError::ExternalService(format!(
                    "Fillout API error ({}): {}",
                    status,
                    error_text
                )))
            }
        }
    }

    pub async fn get_inprogress_edit_link(
        &self,
        form_id: &str,
        assignment_id: &str,
    ) -> Result<Option<String>, AppError> {

        let url = format!(
            "{}/v1/api/forms/{}/submissions?status=in_progress&includeEditLink=true&limit=150",
            self.base_url, form_id
        );

        let response = self.make_submissions_list_request(&url).await?;


        for submission in response.responses {
            let matches = submission.url_parameters.iter().any(|p| {
                p.name == "student_form_assignment_id"
                    && p.value.as_deref() == Some(assignment_id)
            });
            if matches {
                return Ok(submission.edit_link);
            }
        }

        Ok(None)
    }

    async fn make_submissions_list_request(&self, url: &str) -> Result<FilloutSubmissionsListResponse, AppError> {

        let auth_header = format!("Bearer {}", self.api_key);

        let response = self
            .client
            .get(url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to send request to Fillout API: {}", e)))?;

        let status = response.status();

        if status.is_success() {
            let response_text = response.text().await.map_err(|e| {
                AppError::ExternalService(format!("Failed to read response from Fillout API: {}", e))
            })?;

            serde_json::from_str::<FilloutSubmissionsListResponse>(&response_text)
                .map_err(|e| {
                    println!("[ERROR] FilloutService: Failed to parse submissions list: {:?}", e);
                    println!("[ERROR] FilloutService: Raw response: {}", &response_text[..response_text.len().min(500)]);
                    AppError::ExternalService(format!("Failed to parse Fillout submissions list: {}", e))
                })
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            println!("[ERROR] FilloutService: Submissions list error ({}): {}", status, error_text);

            if let Ok(fillout_error) = serde_json::from_str::<FilloutErrorResponse>(&error_text) {
                Err(AppError::ExternalService(format!(
                    "Fillout API error ({}): {}",
                    status,
                    fillout_error.message.unwrap_or(fillout_error.error)
                )))
            } else {
                Err(AppError::ExternalService(format!("Fillout API error ({}): {}", status, error_text)))
            }
        }
    }

    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fillout_service_creation() {
        let service = FilloutService::new(
            "test-api-key".to_string(),
            None,
        );

        assert_eq!(service.get_base_url(), "https://api.fillout.com");
    }

    #[test]
    fn test_fillout_service_custom_base_url() {
        let service = FilloutService::new(
            "test-api-key".to_string(),
            Some("https://custom-api.fillout.com".to_string()),
        );

        assert_eq!(service.get_base_url(), "https://custom-api.fillout.com");
    }
}