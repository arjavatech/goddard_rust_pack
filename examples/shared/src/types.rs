use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(20),
            offset: None,
        }
    }
}

impl PaginationParams {
    pub fn offset(&self) -> u32 {
        self.offset.unwrap_or_else(|| {
            let page = self.page.unwrap_or(1);
            let limit = self.limit.unwrap_or(20);
            (page - 1) * limit
        })
    }

    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(20)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u32,
    pub pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationInfo {
    pub fn new(page: u32, limit: u32, total: u32) -> Self {
        let pages = (total + limit - 1) / limit; // Ceiling division
        Self {
            page,
            limit,
            total,
            pages,
            has_next: page < pages,
            has_prev: page > 1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: Option<T>,
    pub message: String,
    pub success: bool,
    pub timestamp: String,
    pub request_id: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: &str) -> Self {
        Self {
            data: Some(data),
            message: message.to_string(),
            success: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id: None,
        }
    }

    pub fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            data: None,
            message: message.to_string(),
            success: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}