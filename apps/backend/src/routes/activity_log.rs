use axum::{Router, routing::get, extract::{State, Query}, Json};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::auth::AuthSession;
use crate::auth::permissions::Role;
use crate::routes::AppError;
use crate::support::approval_labels;

#[derive(Deserialize)]
pub struct ListQuery {
    pub action: Option<String>,
    #[serde(rename = "dateFrom")]
    pub date_from: Option<String>,
    #[serde(rename = "dateTo")]
    pub date_to: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/", get(list))
}

// ---------------------------------------------------------------------------
// Row type for the JOIN query
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: String,
    action: String,
    description: String,
    created_at: String,
    metadata: Option<String>,
    user_id: Option<String>,
    user_name: Option<String>,
    user_role: Option<String>,
    user_member_id: Option<String>,
}

async fn list(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = Role::from_str(&claims.role).unwrap_or(Role::Member);
    if !role.has_at_least(Role::Organiser) {
        return Err(AppError::Forbidden);
    }

    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).max(1);
    let offset = (page.saturating_sub(1)) * limit;

    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref action) = q.action {
        let idx = bind_values.len() + 1;
        conditions.push(format!("al.action = ?{idx}"));
        bind_values.push(action.clone());
    }
    if let Some(ref date_from) = q.date_from {
        let idx = bind_values.len() + 1;
        conditions.push(format!("al.created_at >= ?{idx}"));
        bind_values.push(format!("{date_from}T00:00:00Z"));
    }
    if let Some(ref date_to) = q.date_to {
        let idx = bind_values.len() + 1;
        conditions.push(format!("al.created_at <= ?{idx}"));
        bind_values.push(format!("{date_to}T23:59:59Z"));
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    let count_sql = format!("SELECT COUNT(*) FROM activity_logs al WHERE {where_clause}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for bind_value in &bind_values {
        count_query = count_query.bind(bind_value);
    }
    let total = count_query.fetch_one(&pool).await.unwrap_or(0);

    // List query with JOIN
    let list_sql = format!(
        "SELECT al.id, al.action, al.description, al.created_at, al.metadata,
                u.id AS user_id, u.name AS user_name, u.role AS user_role, u.member_id AS user_member_id
         FROM activity_logs al
         LEFT JOIN users u ON al.user_id = u.id
         WHERE {where_clause}
         ORDER BY al.created_at DESC
         LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2,
    );

    let mut list_query = sqlx::query_as::<_, ActivityRow>(&list_sql);
    for bind_value in &bind_values {
        list_query = list_query.bind(bind_value);
    }
    list_query = list_query.bind(limit as i64).bind(offset as i64);
    let rows = list_query
        .fetch_all(&pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Build response
    let data: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let metadata = r
                .metadata
                .as_deref()
                .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                .unwrap_or(serde_json::Value::Null);
            let approval_type = approval_labels::read_metadata_approval_type(Some(&metadata))
                .map(str::to_string)
                .or_else(|| {
                    if matches!(
                        r.action.as_str(),
                        "approval_approved"
                            | "approval_rejected"
                            | "membership_approved"
                            | "membership_rejected"
                    ) {
                        Some(approval_labels::MEMBERSHIP_APPROVAL.to_string())
                    } else {
                        None
                    }
                });
            let direction = approval_labels::read_metadata_direction(Some(&metadata))
                .map(str::to_string);
            let user = if r.user_id.is_some() {
                serde_json::json!({
                    "id": r.user_id,
                    "name": r.user_name,
                    "role": r.user_role,
                    "memberId": r.user_member_id,
                })
            } else {
                serde_json::Value::Null
            };

            serde_json::json!({
                "id": r.id,
                "action": r.action,
                "description": r.description,
                "createdAt": r.created_at,
                "metadata": metadata,
                "approvalType": approval_type.as_deref(),
                "approvalTypeLabel": approval_type.as_deref().map(approval_labels::approval_type_label),
                "direction": direction,
                "user": user,
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
    })))
}
