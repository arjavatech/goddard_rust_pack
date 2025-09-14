use axum::{
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use crate::models::schema::ApiResponse;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiEndpoint {
    pub path: String,
    pub method: String,
    pub description: String,
    pub category: String,
    pub auth_required: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiDiscovery {
    pub service_name: String,
    pub version: String,
    pub base_url: String,
    pub total_endpoints: usize,
    pub endpoints: Vec<ApiEndpoint>,
}

pub async fn get_all_endpoints() -> Result<Json<ApiResponse<ApiDiscovery>>, StatusCode> {
    let endpoints = vec![
        // Health & Info
        ApiEndpoint {
            path: "/".to_string(),
            method: "GET".to_string(),
            description: "Welcome message".to_string(),
            category: "Health".to_string(),
            auth_required: false,
        },
        ApiEndpoint {
            path: "/health".to_string(),
            method: "GET".to_string(),
            description: "Health check endpoint".to_string(),
            category: "Health".to_string(),
            auth_required: false,
        },
        ApiEndpoint {
            path: "/api/endpoints".to_string(),
            method: "GET".to_string(),
            description: "List all API endpoints (this endpoint)".to_string(),
            category: "Discovery".to_string(),
            auth_required: false,
        },

        // School Management
        ApiEndpoint {
            path: "/school".to_string(),
            method: "GET".to_string(),
            description: "Get current user's school details".to_string(),
            category: "Schools".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/schools".to_string(),
            method: "GET".to_string(),
            description: "List all schools (super admin only)".to_string(),
            category: "Schools".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/schools".to_string(),
            method: "POST".to_string(),
            description: "Create new school (super admin only)".to_string(),
            category: "Schools".to_string(),
            auth_required: true,
        },

        // User Management
        ApiEndpoint {
            path: "/users".to_string(),
            method: "GET".to_string(),
            description: "List users in the school".to_string(),
            category: "Users".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/users".to_string(),
            method: "POST".to_string(),
            description: "Create new user (admin only)".to_string(),
            category: "Users".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/users/{user_id}".to_string(),
            method: "GET".to_string(),
            description: "Get specific user details".to_string(),
            category: "Users".to_string(),
            auth_required: true,
        },

        // Children Management
        ApiEndpoint {
            path: "/children".to_string(),
            method: "GET".to_string(),
            description: "List children (parents see own, admins see all)".to_string(),
            category: "Children".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/children".to_string(),
            method: "POST".to_string(),
            description: "Add new child to parent account".to_string(),
            category: "Children".to_string(),
            auth_required: true,
        },

        // Classroom Management
        ApiEndpoint {
            path: "/classrooms".to_string(),
            method: "GET".to_string(),
            description: "List classrooms in the school".to_string(),
            category: "Classrooms".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/classrooms".to_string(),
            method: "POST".to_string(),
            description: "Create new classroom (admin only)".to_string(),
            category: "Classrooms".to_string(),
            auth_required: true,
        },

        // Enrollment Management
        ApiEndpoint {
            path: "/enrollments".to_string(),
            method: "GET".to_string(),
            description: "List enrollments".to_string(),
            category: "Enrollments".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/enrollments".to_string(),
            method: "POST".to_string(),
            description: "Create new enrollment".to_string(),
            category: "Enrollments".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/enrollments/{enrollment_id}".to_string(),
            method: "GET".to_string(),
            description: "Get enrollment details".to_string(),
            category: "Enrollments".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/enrollments/{enrollment_id}".to_string(),
            method: "PATCH".to_string(),
            description: "Update enrollment".to_string(),
            category: "Enrollments".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/enrollments/{enrollment_id}/approve".to_string(),
            method: "POST".to_string(),
            description: "Approve enrollment (admin only)".to_string(),
            category: "Enrollments".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/enrollments/{enrollment_id}/reject".to_string(),
            method: "POST".to_string(),
            description: "Reject enrollment (admin only)".to_string(),
            category: "Enrollments".to_string(),
            auth_required: true,
        },

        // Form Management
        ApiEndpoint {
            path: "/form-templates".to_string(),
            method: "GET".to_string(),
            description: "List form templates for the school".to_string(),
            category: "Forms".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/form-templates".to_string(),
            method: "POST".to_string(),
            description: "Create form template (admin only)".to_string(),
            category: "Forms".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/form-submissions".to_string(),
            method: "GET".to_string(),
            description: "List form submissions".to_string(),
            category: "Forms".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/form-submissions/webhook".to_string(),
            method: "POST".to_string(),
            description: "Fillout webhook endpoint".to_string(),
            category: "Forms".to_string(),
            auth_required: false,
        },

        // Communications
        ApiEndpoint {
            path: "/notifications/emails".to_string(),
            method: "GET".to_string(),
            description: "List additional email addresses".to_string(),
            category: "Communications".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/notifications/emails".to_string(),
            method: "POST".to_string(),
            description: "Add additional email for notifications".to_string(),
            category: "Communications".to_string(),
            auth_required: true,
        },

        // Documents
        ApiEndpoint {
            path: "/documents".to_string(),
            method: "GET".to_string(),
            description: "List uploaded documents".to_string(),
            category: "Documents".to_string(),
            auth_required: true,
        },
        ApiEndpoint {
            path: "/documents".to_string(),
            method: "POST".to_string(),
            description: "Upload document (multipart/form-data)".to_string(),
            category: "Documents".to_string(),
            auth_required: true,
        },

        // Admin Dashboard
        ApiEndpoint {
            path: "/admin/dashboard".to_string(),
            method: "GET".to_string(),
            description: "Get dashboard overview (admin only)".to_string(),
            category: "Administration".to_string(),
            auth_required: true,
        },
    ];

    let discovery = ApiDiscovery {
        service_name: "Goddard School Enrollment Management API".to_string(),
        version: "1.0.0".to_string(),
        base_url: "https://api.goddard.com/v1".to_string(),
        total_endpoints: endpoints.len(),
        endpoints,
    };

    Ok(Json(ApiResponse {
        data: discovery,
    }))
}

pub async fn get_endpoints_by_category() -> Result<Json<ApiResponse<std::collections::HashMap<String, Vec<ApiEndpoint>>>>, StatusCode> {
    let endpoints = get_all_endpoints().await?.0.data.endpoints;

    let mut grouped: std::collections::HashMap<String, Vec<ApiEndpoint>> = std::collections::HashMap::new();

    for endpoint in endpoints {
        grouped.entry(endpoint.category.clone())
            .or_insert_with(Vec::new)
            .push(endpoint);
    }

    Ok(Json(ApiResponse {
        data: grouped,
    }))
}