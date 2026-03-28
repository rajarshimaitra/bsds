use axum::{
    extract::{State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use sqlx::SqlitePool;

use crate::{
    auth::AuthSession,
    services::membership_service,
};

use super::AppError;

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/", post(run_daily_cron))
}

pub async fn run_daily_cron(
    State(pool): State<SqlitePool>,
    headers: HeaderMap,
    session: Option<AuthSession>,
) -> Result<Json<membership_service::MembershipCronSummary>, AppError> {
    let is_secret_auth = std::env::var("CRON_SECRET")
        .ok()
        .filter(|secret| !secret.is_empty())
        .and_then(|secret| {
            headers
                .get("x-cron-secret")
                .and_then(|value| value.to_str().ok())
                .filter(|value| *value == secret)
        })
        .is_some();

    if !is_secret_auth {
        let AuthSession(claims) = session.ok_or(AppError::Unauthorized)?;
        if claims.role != "ADMIN" {
            return Err(AppError::Forbidden);
        }
    }

    let summary = membership_service::run_daily_membership_cron(&pool)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(summary))
}
