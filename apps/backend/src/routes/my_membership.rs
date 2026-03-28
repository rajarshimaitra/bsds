use axum::{Router, routing::get, extract::State, Json};
use sqlx::SqlitePool;

use crate::auth::AuthSession;
use crate::repositories::users;
use crate::routes::AppError;

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/", get(my_membership))
}

#[derive(sqlx::FromRow)]
struct TxRow {
    id: String,
    amount: f64,
    approval_status: String,
    includes_subscription: bool,
    includes_annual_fee: bool,
    includes_application_fee: bool,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct SubMemberRow {
    id: String,
    member_id: String,
    name: String,
    email: String,
    phone: String,
    relation: String,
}

async fn my_membership(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
) -> Result<Json<serde_json::Value>, AppError> {
    let err = |e: sqlx::Error| AppError::Internal(e.to_string());

    // Resolve which user's profile to show.
    // For sub-members, we show the parent user's membership.
    let user = users::find_by_id(&pool, &claims.user_id)
        .await
        .map_err(err)?;

    let (user, effective_user_id) = if let Some(u) = user {
        let id = u.id.clone();
        (u, id)
    } else {
        // Try sub_members table — get parent user
        let parent_id: Option<String> = sqlx::query_scalar(
            "SELECT parent_user_id FROM sub_members WHERE id = ?1",
        )
        .bind(&claims.user_id)
        .fetch_optional(&pool)
        .await
        .map_err(err)?
        .flatten();

        let parent_id = parent_id.ok_or(AppError::NotFound)?;

        let parent = users::find_by_id(&pool, &parent_id)
            .await
            .map_err(err)?
            .ok_or(AppError::NotFound)?;

        let id = parent.id.clone();
        (parent, id)
    };

    // Find the linked member record (members.user_id = users.id)
    let member_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM members WHERE user_id = ?1",
    )
    .bind(&effective_user_id)
    .fetch_optional(&pool)
    .await
    .map_err(err)?;

    let not_registered = member_id.is_none();

    // Transaction history — only available when a member record exists
    let tx_history = if let Some(ref mid) = member_id {
        let txs = sqlx::query_as::<_, TxRow>(
            "SELECT id, CAST(amount AS REAL) as amount, approval_status,
                    includes_subscription, includes_annual_fee, includes_application_fee,
                    created_at
             FROM transactions
             WHERE member_id = ?1 AND category = 'MEMBERSHIP'
             ORDER BY created_at DESC",
        )
        .bind(mid)
        .fetch_all(&pool)
        .await
        .map_err(err)?;

        let mut rows = Vec::new();
        for tx in txs {
            let receipt_number: Option<String> = sqlx::query_scalar(
                "SELECT receipt_number FROM receipts WHERE transaction_id = ?1 LIMIT 1",
            )
            .bind(&tx.id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            rows.push(serde_json::json!({
                "id": tx.id,
                "amount": tx.amount.to_string(),
                "approvalStatus": tx.approval_status,
                "includesSubscription": tx.includes_subscription,
                "includesAnnualFee": tx.includes_annual_fee,
                "includesApplicationFee": tx.includes_application_fee,
                "createdAt": tx.created_at,
                "receipt": receipt_number.map(|rn| serde_json::json!({ "receiptNumber": rn })),
            }));
        }
        rows
    } else {
        vec![]
    };

    // Sub-members
    let sub_members = sqlx::query_as::<_, SubMemberRow>(
        "SELECT id, member_id, name, email, phone, relation
         FROM sub_members
         WHERE parent_user_id = ?1
         ORDER BY created_at ASC",
    )
    .bind(&effective_user_id)
    .fetch_all(&pool)
    .await
    .map_err(err)?;

    Ok(Json(serde_json::json!({
        "notRegistered": not_registered,
        "user": {
            "id": user.id,
            "memberId": user.member_id,
            "name": user.name,
            "email": user.email,
            "phone": user.phone,
            "address": user.address,
            "role": user.role,
            "membershipStatus": user.membership_status,
            "membershipType": user.membership_type,
            "membershipStart": user.membership_start,
            "membershipExpiry": user.membership_expiry,
            "totalPaid": user.total_paid.to_string(),
            "applicationFeePaid": user.application_fee_paid,
            "annualFeeStart": user.annual_fee_start,
            "annualFeeExpiry": user.annual_fee_expiry,
            "annualFeePaid": user.annual_fee_paid,
        },
        "member": member_id.map(|id| serde_json::json!({ "id": id })),
        "subMembers": sub_members.into_iter().map(|s| serde_json::json!({
            "id": s.id,
            "memberId": s.member_id,
            "name": s.name,
            "email": s.email,
            "phone": s.phone,
            "relation": s.relation,
        })).collect::<Vec<_>>(),
        "transactionHistory": tx_history,
    })))
}
