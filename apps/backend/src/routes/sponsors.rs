use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use sqlx::SqlitePool;

use crate::{
    auth::AuthSession,
    db::models::{Sponsor, SponsorLink, Transaction},
    services::sponsor_service::{self, ActorInfo, CreateSponsorInput, UpdateSponsorInput},
};

use super::AppError;

#[derive(Debug, Deserialize)]
struct SponsorListQuery {
    search: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
}

#[derive(Debug, Serialize)]
struct PaginatedSponsors {
    data: Vec<Sponsor>,
    total: usize,
    page: u32,
    limit: u32,
    #[serde(rename = "totalPages")]
    total_pages: u32,
}

#[derive(Debug, Deserialize)]
struct CreateSponsorRequest {
    name: String,
    phone: String,
    email: String,
    company: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateSponsorRequest {
    name: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    company: Option<String>,
}

#[derive(Debug, Serialize)]
struct SponsorDetailResponse {
    sponsor: Sponsor,
    links: Vec<SponsorLink>,
    transactions: Vec<Transaction>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_sponsors).post(create_sponsor))
        .route("/:id", get(get_sponsor).put(update_sponsor).delete(delete_sponsor))
}

async fn list_sponsors(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Query(query): Query<SponsorListQuery>,
) -> Result<Json<PaginatedSponsors>, AppError> {
    require_staff(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1);
    let search = query.search.as_deref().map(str::trim).filter(|s| !s.is_empty());

    let sponsors = sponsor_service::list_sponsors(&pool)
        .await
        .map_err(map_sponsor_error)?;

    let filtered: Vec<Sponsor> = sponsors
        .into_iter()
        .filter(|sponsor| match search {
            Some(search_term) => contains_ignore_case(&sponsor.name, search_term)
                || contains_ignore_case(&sponsor.email, search_term)
                || sponsor
                    .company
                    .as_deref()
                    .map(|company| contains_ignore_case(company, search_term))
                    .unwrap_or(false),
            None => true,
        })
        .collect();

    let total = filtered.len();
    let offset = ((page - 1) * limit) as usize;
    let data = filtered
        .into_iter()
        .skip(offset)
        .take(limit as usize)
        .collect::<Vec<_>>();

    let total_pages = if total == 0 {
        0
    } else {
        ((total as f64) / (limit as f64)).ceil() as u32
    };

    Ok(Json(PaginatedSponsors {
        data,
        total,
        page,
        limit,
        total_pages,
    }))
}

async fn create_sponsor(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Json(body): Json<CreateSponsorRequest>,
) -> Result<Json<Sponsor>, AppError> {
    require_staff(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    let sponsor = sponsor_service::create_sponsor(
        &pool,
        &CreateSponsorInput {
            name: body.name,
            phone: body.phone,
            email: body.email,
            company: body.company,
        },
        &actor_from_claims(&claims),
    )
    .await
    .map_err(map_sponsor_error)?;

    Ok(Json(sponsor))
}

async fn get_sponsor(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Path(id): Path<String>,
) -> Result<Json<SponsorDetailResponse>, AppError> {
    require_staff(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    let sponsor = sponsor_service::get_sponsor(&pool, &id)
        .await
        .map_err(map_sponsor_error)?;
    let links = sponsor_service::list_sponsor_links(&pool, &id)
        .await
        .unwrap_or_default();
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE sponsor_id = ?1 ORDER BY created_at DESC",
    )
    .bind(&id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    Ok(Json(SponsorDetailResponse {
        sponsor,
        links,
        transactions,
    }))
}

async fn update_sponsor(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Path(id): Path<String>,
    Json(body): Json<UpdateSponsorRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_staff(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    let sponsor = sponsor_service::update_sponsor(
        &pool,
        &id,
        &UpdateSponsorInput {
            name: body.name,
            phone: body.phone,
            email: body.email,
            company: body.company.map(Some),
        },
        &actor_from_claims(&claims),
    )
    .await
    .map_err(map_sponsor_error)?;

    Ok(Json(serde_json::json!({
        "message": "Sponsor updated successfully",
        "sponsor": sponsor,
    })))
}

async fn delete_sponsor(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_staff(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    sponsor_service::delete_sponsor(&pool, &id, &actor_from_claims(&claims))
        .await
        .map_err(map_sponsor_error)?;

    Ok(Json(serde_json::json!({
        "message": "Sponsor deleted successfully"
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

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
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
