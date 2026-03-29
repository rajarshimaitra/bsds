use axum::{Router, routing::get, extract::State, Json};
use sqlx::SqlitePool;

use crate::auth::AuthSession;
use crate::routes::AppError;

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/stats", get(stats))
}

async fn stats(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = &claims.role;
    let user_id = &claims.user_id;

    if role == "ADMIN" || role == "OPERATOR" || role == "ORGANISER" {
        return Ok(Json(admin_stats(&pool).await?));
    }

    Ok(Json(member_stats(&pool, user_id).await?))
}

// ---------------------------------------------------------------------------
// Admin / Operator / Organiser stats
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ActivityJoinRow {
    id: String,
    action: String,
    description: String,
    created_at: String,
    user_id: Option<String>,
    user_name: Option<String>,
    user_role: Option<String>,
    user_member_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct AuditJoinRow {
    id: String,
    created_at: String,
    transaction_snapshot: String,
    user_id: Option<String>,
    user_name: Option<String>,
    user_role: Option<String>,
    user_member_id: Option<String>,
}

async fn admin_stats(pool: &SqlitePool) -> Result<serde_json::Value, AppError> {
    let err = |e: sqlx::Error| AppError::Internal(e.to_string());

    let total_members: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = 'MEMBER'"
    ).fetch_one(pool).await.map_err(err)?;

    let active_members: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = 'MEMBER' AND membership_status = 'ACTIVE'"
    ).fetch_one(pool).await.map_err(err)?;

    let pending_members: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = 'MEMBER' AND membership_status IN ('PENDING_APPROVAL', 'PENDING_PAYMENT')"
    ).fetch_one(pool).await.map_err(err)?;

    let expired_members: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role = 'MEMBER' AND membership_status = 'EXPIRED'"
    ).fetch_one(pool).await.map_err(err)?;

    let total_income: f64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(amount), 0) AS REAL) FROM transactions WHERE type = 'CASH_IN' AND approval_status = 'APPROVED'"
    ).fetch_one(pool).await.unwrap_or(0.0);

    let total_expenses: f64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(amount), 0) AS REAL) FROM transactions WHERE type = 'CASH_OUT' AND approval_status = 'APPROVED'"
    ).fetch_one(pool).await.unwrap_or(0.0);

    let pending_amount: f64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(amount), 0) AS REAL) FROM transactions WHERE approval_status = 'PENDING'"
    ).fetch_one(pool).await.unwrap_or(0.0);

    let pending_approvals: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM approvals WHERE status = 'PENDING'"
    ).fetch_one(pool).await.map_err(err)?;

    let activity_rows = sqlx::query_as::<_, ActivityJoinRow>(
        "SELECT a.id, a.action, a.description, a.created_at,
                u.id         AS user_id,
                u.name       AS user_name,
                u.role       AS user_role,
                u.member_id  AS user_member_id
         FROM activity_logs a
         LEFT JOIN users u ON a.user_id = u.id
         ORDER BY a.created_at DESC
         LIMIT 10"
    )
    .fetch_all(pool)
    .await
    .map_err(err)?;

    let recent_activity: Vec<serde_json::Value> = activity_rows
        .into_iter()
        .map(|r| serde_json::json!({
            "id": r.id,
            "action": r.action,
            "description": r.description,
            "createdAt": r.created_at,
            "user": {
                "id": r.user_id,
                "name": r.user_name,
                "role": r.user_role,
                "memberId": r.user_member_id,
            }
        }))
        .collect();

    let audit_rows = sqlx::query_as::<_, AuditJoinRow>(
        "SELECT al.id, al.created_at, al.transaction_snapshot,
                u.id         AS user_id,
                u.name       AS user_name,
                u.role       AS user_role,
                u.member_id  AS user_member_id
         FROM audit_logs al
         JOIN transactions t ON al.transaction_id = t.id AND t.approval_status = 'APPROVED'
         LEFT JOIN users u ON al.performed_by_id = u.id
         ORDER BY al.created_at DESC
         LIMIT 10"
    )
    .fetch_all(pool)
    .await
    .map_err(err)?;

    let recent_audit: Vec<serde_json::Value> = audit_rows
        .into_iter()
        .map(|r| {
            let snapshot: serde_json::Value =
                serde_json::from_str(&r.transaction_snapshot).unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "id": r.id,
                "transactionSnapshot": snapshot,
                "createdAt": r.created_at,
                "performedBy": {
                    "id": r.user_id,
                    "name": r.user_name,
                    "role": r.user_role,
                    "memberId": r.user_member_id,
                }
            })
        })
        .collect();

    Ok(serde_json::json!({
        "members": {
            "total": total_members,
            "active": active_members,
            "pending": pending_members,
            "expired": expired_members,
        },
        "financial": {
            "totalIncome": total_income,
            "totalExpenses": total_expenses,
            "pendingApprovals": pending_amount,
            "netBalance": total_income - total_expenses,
        },
        "approvals": {
            "pending": pending_approvals,
        },
        "recentActivity": recent_activity,
        "recentAudit": recent_audit,
    }))
}

// ---------------------------------------------------------------------------
// Member stats
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    membership_status: String,
    membership_type: Option<String>,
    membership_expiry: Option<String>,
    total_paid: f64,
}

#[derive(sqlx::FromRow)]
struct SubMemberRow {
    id: String,
    member_id: String,
    name: String,
    relation: String,
    created_at: String,
}

fn days_until(expiry: &str) -> Option<i64> {
    let expiry_date = expiry.get(..10)?;
    let today = chrono::Utc::now().date_naive();
    let exp = chrono::NaiveDate::parse_from_str(expiry_date, "%Y-%m-%d").ok()?;
    Some((exp - today).num_days())
}

async fn member_stats(pool: &SqlitePool, user_id: &str) -> Result<serde_json::Value, AppError> {
    let err = |e: sqlx::Error| AppError::Internal(e.to_string());

    let primary = sqlx::query_as::<_, UserRow>(
        "SELECT id, membership_status, membership_type, membership_expiry, CAST(total_paid AS REAL) AS total_paid FROM users WHERE id = ?1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(err)?;

    let (effective_id, membership_status, membership_type, membership_expiry, total_paid) =
        if let Some(u) = primary {
            (u.id, u.membership_status, u.membership_type, u.membership_expiry, u.total_paid)
        } else {
            // Try sub_members table
            let parent_id: Option<String> = sqlx::query_scalar(
                "SELECT parent_user_id FROM sub_members WHERE id = ?1"
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(err)?
            .flatten();

            let parent_id = parent_id.ok_or(AppError::NotFound)?;

            let parent = sqlx::query_as::<_, UserRow>(
                "SELECT id, membership_status, membership_type, membership_expiry, CAST(total_paid AS REAL) AS total_paid FROM users WHERE id = ?1"
            )
            .bind(&parent_id)
            .fetch_one(pool)
            .await
            .map_err(err)?;

            (parent.id, parent.membership_status, parent.membership_type, parent.membership_expiry, parent.total_paid)
        };

    let last_payment: Option<String> = sqlx::query_scalar(
        "SELECT t.created_at
         FROM transactions t
         JOIN members m ON t.member_id = m.id AND m.user_id = ?1
         WHERE t.type = 'CASH_IN' AND t.approval_status = 'APPROVED'
         ORDER BY t.created_at DESC
         LIMIT 1"
    )
    .bind(&effective_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let sub_members = sqlx::query_as::<_, SubMemberRow>(
        "SELECT id, member_id, name, relation, created_at FROM sub_members WHERE parent_user_id = ?1 ORDER BY created_at ASC"
    )
    .bind(&effective_id)
    .fetch_all(pool)
    .await
    .map_err(err)?;

    let days_left = membership_expiry.as_deref().and_then(days_until);

    Ok(serde_json::json!({
        "membership": {
            "status": membership_status,
            "type": membership_type,
            "expiry": membership_expiry,
            "daysLeft": days_left,
        },
        "payments": {
            "total": total_paid,
            "lastPayment": last_payment,
        },
        "subMembers": sub_members.into_iter().map(|s| serde_json::json!({
            "id": s.id,
            "memberId": s.member_id,
            "name": s.name,
            "relation": s.relation,
            "createdAt": s.created_at,
        })).collect::<Vec<_>>(),
    }))
}
