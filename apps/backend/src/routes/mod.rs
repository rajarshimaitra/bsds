use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

pub mod activity_log;
pub mod approvals;
pub mod audit_log;
pub mod auth;
pub mod cron;
pub mod dashboard;
pub mod members;
pub mod memberships;
pub mod my_membership;
pub mod payments;
pub mod receipts;
pub mod sponsor_links;
pub mod sponsors;
pub mod notifications;
pub mod transactions;
pub mod webhooks;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("resource not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "resource not found".to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden".to_string()),
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            AppError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
