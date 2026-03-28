use axum::{
    Router,
    routing::get,
    extract::{State, Path, Query},
    Json,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::auth::AuthSession;
use crate::auth::permissions::Role;
use crate::routes::AppError;
use crate::services::membership_service::{self, RequestedBy, CreateMembershipInput};

#[derive(Deserialize)]
pub struct ListQuery {
    pub member_id: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_memberships).post(create_membership))
        .route("/:id", get(get_membership).patch(update_membership))
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn parse_role(claims: &crate::auth::SessionClaims) -> Role {
    Role::from_str(&claims.role).unwrap_or(Role::Member)
}

// ---------------------------------------------------------------------------
// GET / — list memberships — MEMBER+ (any authenticated user)
// ---------------------------------------------------------------------------

async fn list_memberships(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Any authenticated user may access memberships; role already validated
    // by AuthSession extractor (returns 401 if unauthenticated).
    let _ = parse_role(&claims); // ensure claims are valid

    if let Some(member_id) = &q.member_id {
        let items = membership_service::get_memberships_by_member(&pool, member_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        return Ok(Json(serde_json::json!(items)));
    }

    // Full paginated list: query memberships table directly with pagination.
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).max(1);
    let offset = (page - 1) * limit;

    let status_filter = q.status.as_deref().unwrap_or("");

    // Build query depending on whether status filter is present
    let items: Vec<serde_json::Value> = if status_filter.is_empty() {
        sqlx::query_as::<_, crate::db::models::Membership>(
            "SELECT id, member_id, \"type\", fee_type, CAST(amount AS REAL) as amount, \
             start_date, end_date, is_application_fee, status, created_at \
             FROM memberships ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .into_iter()
        .map(|m| serde_json::json!(m))
        .collect()
    } else {
        sqlx::query_as::<_, crate::db::models::Membership>(
            "SELECT id, member_id, \"type\", fee_type, CAST(amount AS REAL) as amount, \
             start_date, end_date, is_application_fee, status, created_at \
             FROM memberships WHERE status = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        )
        .bind(status_filter)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .into_iter()
        .map(|m| serde_json::json!(m))
        .collect()
    };

    let total: i64 = if status_filter.is_empty() {
        sqlx::query_scalar("SELECT COUNT(*) FROM memberships")
            .fetch_one(&pool)
            .await
            .unwrap_or(0)
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM memberships WHERE status = ?1")
            .bind(status_filter)
            .fetch_one(&pool)
            .await
            .unwrap_or(0)
    };

    Ok(Json(serde_json::json!({
        "data": items,
        "total": total,
        "page": page,
        "limit": limit,
    })))
}

// ---------------------------------------------------------------------------
// GET /:id — get single membership — MEMBER+
// ---------------------------------------------------------------------------

async fn get_membership(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = parse_role(&claims);
    let m = membership_service::get_membership(&pool, &id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(serde_json::json!(m)))
}

// ---------------------------------------------------------------------------
// POST / — create membership — MEMBER+
// ---------------------------------------------------------------------------

async fn create_membership(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = parse_role(&claims);
    let actor = RequestedBy { id: claims.user_id.clone(), role: claims.role.clone(), name: claims.username.clone() };
    // Parse amount from either JSON number or string (frontend may send as string)
    let amount = body["amount"].as_f64()
        .or_else(|| body["amount"].as_str().and_then(|s| s.parse::<f64>().ok()))
        .unwrap_or(0.0);
    // Support both the legacy feeType field and the new boolean flags sent by the frontend
    let includes_annual_fee = body["includesAnnualFee"].as_bool();
    let includes_subscription = body["includesSubscription"].as_bool();
    // Accept both field name variants for application fee
    let is_application_fee = body["includesApplicationFee"].as_bool()
        .or_else(|| body["isApplicationFee"].as_bool());
    let input = CreateMembershipInput {
        member_id: body["memberId"].as_str().unwrap_or("").to_string(),
        r#type: body["type"].as_str().unwrap_or("").to_string(),
        amount,
        fee_type: body["feeType"].as_str().map(String::from),
        is_application_fee,
        includes_subscription,
        includes_annual_fee,
    };
    let result = membership_service::create_membership(&pool, &input, &actor)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "action": result.action,
        "transactionId": result.transaction_id,
        "approvalId": result.approval_id,
    })))
}

// ---------------------------------------------------------------------------
// PATCH /:id — approve/reject membership — MEMBER+ (internally gated)
// ---------------------------------------------------------------------------

async fn update_membership(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    // Approve/reject actions require at minimum OPERATOR
    if !role.has_at_least(Role::Operator) {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy { id: claims.user_id.clone(), role: claims.role.clone(), name: claims.username.clone() };
    let action = body.get("action").and_then(|v| v.as_str()).unwrap_or("approve");
    let notes = body.get("notes").and_then(|v| v.as_str());
    if action == "reject" {
        membership_service::reject_membership(&pool, &id, &actor, notes)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
    } else {
        membership_service::approve_membership(&pool, &id, &actor)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
