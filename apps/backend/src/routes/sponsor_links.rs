use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use sqlx::SqlitePool;

use crate::{
    auth::AuthSession,
    db::models::SponsorLink,
    repositories::sponsor_links,
    services::sponsor_service::{self, ActorInfo, CreateSponsorLinkInput},
};

use super::AppError;

#[derive(Debug, Deserialize)]
struct SponsorLinkListQuery {
    #[serde(rename = "sponsorId")]
    sponsor_id: Option<String>,
    #[serde(rename = "isActive")]
    is_active: Option<bool>,
    page: Option<u32>,
    limit: Option<u32>,
}

#[derive(Debug, Serialize)]
struct PaginatedSponsorLinks {
    data: Vec<SponsorLink>,
    total: usize,
    page: u32,
    limit: u32,
    #[serde(rename = "totalPages")]
    total_pages: u32,
}

#[derive(Debug, Deserialize)]
struct CreateSponsorLinkRequest {
    #[serde(rename = "sponsorId")]
    sponsor_id: Option<String>,
    amount: Option<f64>,
    #[serde(rename = "upiId")]
    upi_id: String,
    #[serde(rename = "bankDetails")]
    bank_details: Option<serde_json::Value>,
    #[serde(rename = "expiresAt")]
    expires_at: Option<String>,
    #[serde(rename = "sponsorPurpose")]
    sponsor_purpose: String,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_sponsor_links).post(create_sponsor_link))
        .route("/:token", get(get_public_sponsor_link).patch(deactivate_sponsor_link))
}

async fn list_sponsor_links(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Query(query): Query<SponsorLinkListQuery>,
) -> Result<Json<PaginatedSponsorLinks>, AppError> {
    require_staff(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1);
    let mut links = if let Some(ref sponsor_id) = query.sponsor_id {
        sponsor_service::list_sponsor_links(&pool, sponsor_id)
            .await
            .map_err(map_sponsor_error)?
    } else {
        sqlx::query_as::<_, SponsorLink>("SELECT * FROM sponsor_links ORDER BY created_at DESC")
            .fetch_all(&pool)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?
    };

    if let Some(is_active) = query.is_active {
        links.retain(|link| link.is_active == is_active);
    }

    let total = links.len();
    let offset = ((page - 1) * limit) as usize;
    let data = links
        .into_iter()
        .skip(offset)
        .take(limit as usize)
        .collect::<Vec<_>>();
    let total_pages = if total == 0 {
        0
    } else {
        ((total as f64) / (limit as f64)).ceil() as u32
    };

    Ok(Json(PaginatedSponsorLinks {
        data,
        total,
        page,
        limit,
        total_pages,
    }))
}

async fn create_sponsor_link(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Json(body): Json<CreateSponsorLinkRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_staff(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    let (link, url) = sponsor_service::generate_sponsor_link(
        &pool,
        &CreateSponsorLinkInput {
            sponsor_id: body.sponsor_id,
            amount: body.amount,
            upi_id: body.upi_id,
            bank_details: body.bank_details,
            expires_at: body.expires_at,
            sponsor_purpose: body.sponsor_purpose,
        },
        &actor_from_claims(&claims),
    )
    .await
    .map_err(map_sponsor_error)?;

    Ok(Json(serde_json::json!({
        "message": "Sponsor payment link generated successfully",
        "link": link,
        "url": url,
    })))
}

async fn get_public_sponsor_link(
    State(pool): State<SqlitePool>,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let data = sponsor_service::get_public_sponsor_link(&pool, &token)
        .await
        .map_err(map_sponsor_error)?;

    if !data.is_active || data.is_expired {
        return Ok(Json(serde_json::json!({
            "error": if data.is_expired {
                "This payment link has expired"
            } else {
                "This payment link is no longer active"
            },
            "data": data,
        })));
    }

    Ok(Json(serde_json::to_value(data).unwrap_or_default()))
}

async fn deactivate_sponsor_link(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_staff(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    let link = sponsor_links::find_by_token(&pool, &token)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .ok_or(AppError::NotFound)?;

    sponsor_service::deactivate_sponsor_link(&pool, &link.id, &actor_from_claims(&claims))
        .await
        .map_err(map_sponsor_error)?;

    Ok(Json(serde_json::json!({
        "message": "Sponsor link deactivated successfully"
    })))
}

fn actor_from_claims(claims: &crate::auth::SessionClaims) -> ActorInfo {
    ActorInfo {
        id: claims.user_id.clone(),
        name: claims.username.clone(),
    }
}

fn require_staff(role: &str) -> Result<(), AppError> {
    if matches!(role, "ADMIN" | "OPERATOR") {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn require_password_changed(must_change_password: bool) -> Result<(), AppError> {
    if must_change_password {
        Err(AppError::Forbidden)
    } else {
        Ok(())
    }
}

fn map_sponsor_error(error: sponsor_service::SponsorServiceError) -> AppError {
    match error {
        sponsor_service::SponsorServiceError::SponsorNotFound
        | sponsor_service::SponsorServiceError::LinkNotFound => AppError::NotFound,
        sponsor_service::SponsorServiceError::LinkAlreadyInactive
        | sponsor_service::SponsorServiceError::HasTransactions
        | sponsor_service::SponsorServiceError::NoFieldsProvided => {
            AppError::BadRequest(error.to_string())
        }
        sponsor_service::SponsorServiceError::Database(_) => AppError::Internal(error.to_string()),
    }
}

use serde::{Deserialize, Serialize};
