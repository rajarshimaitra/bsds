use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path, Query},
    Json,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::auth::AuthSession;
use crate::auth::permissions::Role;
use crate::routes::AppError;
use crate::services::approval_service::{self, ReviewedBy};
use crate::support::approval_labels;

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    #[serde(rename = "approvalType")]
    pub approval_type: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_approvals))
        .route("/:id", get(get_approval))
        .route("/:id/approve", post(approve_entry))
        .route("/:id/reject", post(reject_entry))
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn require_admin(claims: &crate::auth::SessionClaims) -> Result<(), AppError> {
    let role = Role::from_str(&claims.role).unwrap_or(Role::Member);
    if role != Role::Admin {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Row type for the JOIN query
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ApprovalRow {
    id: String,
    entity_type: String,
    entity_id: String,
    action: String,
    previous_data: Option<String>,
    new_data: Option<String>,
    status: String,
    notes: Option<String>,
    reviewed_at: Option<String>,
    created_at: String,
    req_id: Option<String>,
    req_name: Option<String>,
    req_email: Option<String>,
    req_role: Option<String>,
    rev_id: Option<String>,
    rev_name: Option<String>,
    rev_email: Option<String>,
    tx_type: Option<String>,
    tx_category: Option<String>,
}

// ---------------------------------------------------------------------------
// GET / — list approvals — ADMIN only
// ---------------------------------------------------------------------------

async fn list_approvals(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&claims)?;

    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).max(1);
    let offset = (page.saturating_sub(1)) * limit;

    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    match q.status.as_deref() {
        None => conditions.push("a.status = 'PENDING'".to_string()),
        Some("ALL") => {}
        Some(status) => {
            let idx = bind_values.len() + 1;
            conditions.push(format!("a.status = ?{idx}"));
            bind_values.push(status.to_string());
        }
    }

    match q.approval_type.as_deref() {
        Some(approval_labels::MEMBERSHIP_APPROVAL) => {
            conditions.push("a.entity_type != 'TRANSACTION'".to_string());
        }
        Some(approval_labels::MEMBERSHIP_PAYMENT_APPROVAL) => {
            conditions.push(
                "a.entity_type = 'TRANSACTION' AND COALESCE(t.category, '') = 'MEMBERSHIP'"
                    .to_string(),
            );
        }
        Some(approval_labels::TRANSACTION_APPROVAL) => {
            conditions.push(
                "a.entity_type = 'TRANSACTION' AND COALESCE(t.category, '') != 'MEMBERSHIP'"
                    .to_string(),
            );
        }
        _ => {}
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    let count_sql = format!(
        "SELECT COUNT(*)
         FROM approvals a
         LEFT JOIN transactions t ON a.entity_type = 'TRANSACTION' AND a.entity_id = t.id
         WHERE {where_clause}"
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for bind_value in &bind_values {
        count_query = count_query.bind(bind_value);
    }
    let total = count_query.fetch_one(&pool).await.unwrap_or(0);

    let list_sql = format!(
        "SELECT a.id, a.entity_type, a.entity_id, a.action, a.previous_data, a.new_data,
                a.status, a.notes, a.reviewed_at, a.created_at,
                u1.id AS req_id, u1.name AS req_name, u1.email AS req_email, u1.role AS req_role,
                u2.id AS rev_id, u2.name AS rev_name, u2.email AS rev_email,
                t.type AS tx_type, t.category AS tx_category
         FROM approvals a
         LEFT JOIN users u1 ON a.requested_by_id = u1.id
         LEFT JOIN users u2 ON a.reviewed_by_id = u2.id
         LEFT JOIN transactions t ON a.entity_type = 'TRANSACTION' AND a.entity_id = t.id
         WHERE {where_clause}
         ORDER BY a.created_at DESC
         LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2,
    );

    let mut list_query = sqlx::query_as::<_, ApprovalRow>(&list_sql);
    for bind_value in &bind_values {
        list_query = list_query.bind(bind_value);
    }
    list_query = list_query.bind(limit as i64).bind(offset as i64);
    let rows = list_query
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Pending count (always fetched for badge)
    let pending_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM approvals WHERE status = 'PENDING'")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);

    // Build response items
    let data: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let requested_by = serde_json::json!({
                "id": r.req_id,
                "name": r.req_name,
                "email": r.req_email,
                "role": r.req_role,
            });

            let reviewed_by = if r.rev_id.is_some() {
                serde_json::json!({
                    "id": r.rev_id,
                    "name": r.rev_name,
                    "email": r.rev_email,
                })
            } else {
                serde_json::Value::Null
            };

            let previous_data = r.previous_data
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .unwrap_or(serde_json::Value::Null);

            let new_data = r.new_data
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .unwrap_or(serde_json::Value::Null);

            let approval_type = approval_labels::approval_type_from_entity(
                &r.entity_type,
                r.tx_category.as_deref(),
            );
            let direction =
                approval_labels::direction_from_transaction_type(r.tx_type.as_deref());

            serde_json::json!({
                "id": r.id,
                "entityType": r.entity_type,
                "entityId": r.entity_id,
                "action": r.action,
                "approvalType": approval_type,
                "approvalTypeLabel": approval_labels::approval_type_label(approval_type),
                "direction": direction,
                "previousData": previous_data,
                "newData": new_data,
                "status": r.status,
                "notes": r.notes,
                "reviewedAt": r.reviewed_at,
                "createdAt": r.created_at,
                "requestedBy": requested_by,
                "reviewedBy": reviewed_by,
            })
        })
        .collect();

    let total_pages = if limit > 0 { (total as u32 + limit - 1) / limit } else { 0 };

    Ok(Json(serde_json::json!({
        "data": data,
        "total": total,
        "page": page,
        "limit": limit,
        "totalPages": total_pages,
        "pendingCount": pending_count,
    })))
}

// ---------------------------------------------------------------------------
// GET /:id — get single approval — ADMIN only
// ---------------------------------------------------------------------------

async fn get_approval(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&claims)?;
    let approval = approval_service::get_approval(&pool, &id)
        .await
        .map_err(|e| match e {
            approval_service::ApprovalServiceError::NotFound => AppError::NotFound,
            other => AppError::Internal(other.to_string()),
        })?;
    Ok(Json(serde_json::json!(approval)))
}

// ---------------------------------------------------------------------------
// POST /:id/approve — approve entry — ADMIN only
// ---------------------------------------------------------------------------

async fn approve_entry(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&claims)?;
    let reviewer = ReviewedBy {
        id: claims.user_id.clone(),
        name: claims.username.clone(),
    };
    let notes = body.get("notes").and_then(|v| v.as_str());
    approval_service::approve_entry(&pool, &id, &reviewer, notes)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "Approval applied successfully",
    })))
}

// ---------------------------------------------------------------------------
// POST /:id/reject — reject entry — ADMIN only
// ---------------------------------------------------------------------------

async fn reject_entry(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&claims)?;
    let reviewer = ReviewedBy {
        id: claims.user_id.clone(),
        name: claims.username.clone(),
    };
    let notes = body.get("notes").and_then(|v| v.as_str());
    approval_service::reject_entry(&pool, &id, &reviewer, notes)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "Approval rejected successfully",
    })))
}
