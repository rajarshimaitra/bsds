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
use crate::services::transaction_service::{self, RequestedBy, CreateTransactionInput};
use crate::repositories::transactions::TransactionListFilters;

#[derive(Deserialize)]
pub struct ListQuery {
    pub category: Option<String>,
    pub status: Option<String>,
    pub member_id: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_transactions).post(create_transaction))
        // /summary must be registered before /:id to avoid path conflict
        .route("/summary", get(transaction_summary))
        .route("/:id", get(get_transaction))
        .route("/:id/reject", post(reject_transaction_handler))
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn parse_role(claims: &crate::auth::SessionClaims) -> Role {
    Role::from_str(&claims.role).unwrap_or(Role::Member)
}

// ---------------------------------------------------------------------------
// GET / — list transactions — ORGANISER+
// ---------------------------------------------------------------------------

async fn list_transactions(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Organiser) {
        return Err(AppError::Forbidden);
    }
    let page = q.page.unwrap_or(1);
    let limit = q.limit.unwrap_or(50);
    let filters = TransactionListFilters {
        category: q.category,
        status: q.status,
        page,
        limit,
        ..TransactionListFilters::default()
    };
    let (items, total) = transaction_service::list_transactions(&pool, &filters)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Batch-fetch entered_by user info so the frontend can show name instead of raw ID.
    let entered_by_ids: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        items.iter().filter(|t| seen.insert(t.entered_by_id.clone())).map(|t| t.entered_by_id.clone()).collect()
    };
    let user_map: std::collections::HashMap<String, (String, String)> = if entered_by_ids.is_empty() {
        Default::default()
    } else {
        let placeholders: String = (1..=entered_by_ids.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(",");
        let sql = format!("SELECT id, name, email FROM users WHERE id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, (String, String, String)>(&sql);
        for id in &entered_by_ids {
            q = q.bind(id.as_str());
        }
        q.fetch_all(&pool).await.unwrap_or_default().into_iter().map(|(id, name, email)| (id, (name, email))).collect()
    };

    let data: Vec<serde_json::Value> = items
        .into_iter()
        .map(|t| {
            // Build the nested receipt object the frontend expects.
            // If the transaction was rejected, its receipt is cancelled.
            let receipt = t.receipt_number.as_ref().map(|rn| {
                let status = if t.approval_status == "REJECTED" {
                    "CANCELLED"
                } else {
                    "ACTIVE"
                };
                serde_json::json!({"receiptNumber": rn, "status": status})
            });

            serde_json::json!({
                "id": t.id,
                "type": t.r#type,
                "category": t.category,
                "amount": t.amount,
                "paymentMode": t.payment_mode,
                "purpose": t.purpose,
                "remark": t.remark,
                "sponsorPurpose": t.sponsor_purpose,
                "memberId": t.member_id,
                "sponsorId": t.sponsor_id,
                "enteredById": t.entered_by_id,
                "enteredBy": {
                    "id": t.entered_by_id,
                    "name": user_map.get(&t.entered_by_id).map(|(n, _)| n.as_str()).unwrap_or(""),
                    "email": user_map.get(&t.entered_by_id).map(|(_, e)| e.as_str()).unwrap_or(""),
                },
                "approvalStatus": t.approval_status,
                "approvalSource": t.approval_source,
                "approvedById": t.approved_by_id,
                "approvedAt": t.approved_at,
                "razorpayPaymentId": t.razorpay_payment_id,
                "razorpayOrderId": t.razorpay_order_id,
                "senderName": t.sender_name,
                "senderPhone": t.sender_phone,
                "senderUpiId": t.sender_upi_id,
                "senderBankAccount": t.sender_bank_account,
                "senderBankName": t.sender_bank_name,
                "sponsorSenderName": t.sponsor_sender_name,
                "sponsorSenderContact": t.sponsor_sender_contact,
                "receiptNumber": t.receipt_number,
                "receipt": receipt,
                "includesSubscription": t.includes_subscription,
                "includesAnnualFee": t.includes_annual_fee,
                "includesApplicationFee": t.includes_application_fee,
                "createdAt": t.created_at,
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

// ---------------------------------------------------------------------------
// GET /summary — transaction summary — ORGANISER+
// ---------------------------------------------------------------------------

async fn transaction_summary(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Organiser) {
        return Err(AppError::Forbidden);
    }
    let summary = transaction_service::get_transaction_summary(&pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "totalIncome": summary.total_income,
        "totalExpenses": summary.total_expenses,
        "pendingAmount": summary.pending_amount,
        "netBalance": summary.net_balance,
    })))
}

// ---------------------------------------------------------------------------
// GET /:id — get single transaction — ORGANISER+
// ---------------------------------------------------------------------------

async fn get_transaction(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Organiser) {
        return Err(AppError::Forbidden);
    }
    let t = transaction_service::get_transaction(&pool, &id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Fetch member name/phone if linked
    let member_json: serde_json::Value = if let Some(ref mid) = t.member_id {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, name, phone FROM members WHERE id = ?1",
        )
        .bind(mid)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None)
        .map(|(id, name, phone)| serde_json::json!({"id": id, "name": name, "phone": phone}))
        .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Null
    };

    // Fetch sponsor name/company/phone if linked
    let sponsor_json: serde_json::Value = if let Some(ref sid) = t.sponsor_id {
        sqlx::query_as::<_, (String, String, Option<String>, String)>(
            "SELECT id, name, company, phone FROM sponsors WHERE id = ?1",
        )
        .bind(sid)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None)
        .map(|(id, name, company, phone)| {
            serde_json::json!({"id": id, "name": name, "company": company, "phone": phone})
        })
        .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Null
    };

    Ok(Json(serde_json::json!({
        "id": t.id,
        "type": t.r#type,
        "category": t.category,
        "amount": t.amount,
        "paymentMode": t.payment_mode,
        "purpose": t.purpose,
        "remark": t.remark,
        "sponsorPurpose": t.sponsor_purpose,
        "memberId": t.member_id,
        "member": member_json,
        "sponsorId": t.sponsor_id,
        "sponsor": sponsor_json,
        "enteredById": t.entered_by_id,
        "approvalStatus": t.approval_status,
        "approvalSource": t.approval_source,
        "approvedById": t.approved_by_id,
        "approvedAt": t.approved_at,
        "razorpayPaymentId": t.razorpay_payment_id,
        "razorpayOrderId": t.razorpay_order_id,
        "senderName": t.sender_name,
        "senderPhone": t.sender_phone,
        "senderUpiId": t.sender_upi_id,
        "senderBankAccount": t.sender_bank_account,
        "senderBankName": t.sender_bank_name,
        "sponsorSenderName": t.sponsor_sender_name,
        "sponsorSenderContact": t.sponsor_sender_contact,
        "receiptNumber": t.receipt_number,
        "includesSubscription": t.includes_subscription,
        "includesAnnualFee": t.includes_annual_fee,
        "includesApplicationFee": t.includes_application_fee,
        "createdAt": t.created_at,
    })))
}

// ---------------------------------------------------------------------------
// POST / — create transaction — OPERATOR+
// ---------------------------------------------------------------------------

async fn create_transaction(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Operator) {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy { id: claims.user_id.clone(), role: claims.role.clone(), name: claims.username.clone() };
    let input = CreateTransactionInput {
        r#type: body["type"].as_str().unwrap_or("").to_string(),
        category: body["category"].as_str().unwrap_or("").to_string(),
        amount: body["amount"].as_f64().unwrap_or(0.0),
        payment_mode: body["paymentMode"].as_str().unwrap_or("CASH").to_string(),
        purpose: body["purpose"].as_str().unwrap_or("").to_string(),
        remark: body["remark"].as_str().map(String::from),
        sponsor_purpose: body["sponsorPurpose"].as_str().map(String::from),
        member_id: body["memberId"].as_str().map(String::from),
        sponsor_id: body["sponsorId"].as_str().map(String::from),
        sender_name: body["senderName"].as_str().map(String::from),
        sender_phone: body["senderPhone"].as_str().map(String::from),
        sponsor_sender_name: body["sponsorSenderName"].as_str().map(String::from),
        sponsor_sender_contact: body["sponsorSenderContact"].as_str().map(String::from),
        membership_type: body["membershipType"].as_str().map(String::from),
        fee_type: body["feeType"].as_str().map(String::from),
        is_application_fee: body["isApplicationFee"].as_bool(),
        includes_subscription: body["includesSubscription"].as_bool(),
        includes_annual_fee: body["includesAnnualFee"].as_bool(),
        includes_application_fee: body["includesApplicationFee"].as_bool(),
    };
    let result = transaction_service::create_transaction(&pool, &input, &actor)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "action": result.action,
        "transactionId": result.transaction_id,
        "approvalId": result.approval_id,
    })))
}

// ---------------------------------------------------------------------------
// POST /:id/reject — reject transaction — ADMIN only
// ---------------------------------------------------------------------------

async fn reject_transaction_handler(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if role != Role::Admin {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy {
        id: claims.user_id.clone(),
        role: claims.role.clone(),
        name: claims.username.clone(),
    };
    transaction_service::reject_transaction(&pool, &id, &actor)
        .await
        .map_err(|e| match e {
            transaction_service::TransactionServiceError::NotFound => AppError::NotFound,
            transaction_service::TransactionServiceError::Forbidden => AppError::Forbidden,
            transaction_service::TransactionServiceError::AlreadyRejected => {
                AppError::BadRequest("Transaction is already rejected".to_string())
            }
            other => AppError::BadRequest(other.to_string()),
        })?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "Transaction rejected successfully",
    })))
}
