use axum::{Router, routing::get, extract::{State, Query}, Json};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::auth::AuthSession;
use crate::auth::permissions::Role;
use crate::routes::AppError;
use crate::support::approval_labels;

#[derive(Deserialize)]
pub struct ListQuery {
    pub transaction_id: Option<String>,
    pub category: Option<String>,
    pub event_type: Option<String>,
    pub performed_by_id: Option<String>,
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
struct AuditRow {
    id: String,
    transaction_id: String,
    event_type: String,
    performed_by_id: String,
    created_at: String,
    transaction_snapshot: String,
    user_id: Option<String>,
    user_name: Option<String>,
    user_role: Option<String>,
    user_member_id: Option<String>,
    tx_id: Option<String>,
    tx_type: Option<String>,
    tx_category: Option<String>,
    tx_amount: Option<f64>,
    tx_payment_mode: Option<String>,
    tx_purpose: Option<String>,
    tx_remark: Option<String>,
    tx_sponsor_purpose: Option<String>,
    tx_approval_status: Option<String>,
    tx_approval_source: Option<String>,
    tx_member_id: Option<String>,
    tx_member_name: Option<String>,
    tx_member_phone: Option<String>,
    tx_sponsor_id: Option<String>,
    tx_sponsor_name: Option<String>,
    tx_sponsor_company: Option<String>,
    tx_sponsor_phone: Option<String>,
    tx_sender_name: Option<String>,
    tx_sender_phone: Option<String>,
    tx_sender_upi_id: Option<String>,
    tx_sender_bank_account: Option<String>,
    tx_sender_bank_name: Option<String>,
    tx_sponsor_sender_name: Option<String>,
    tx_sponsor_sender_contact: Option<String>,
    tx_razorpay_payment_id: Option<String>,
    tx_razorpay_order_id: Option<String>,
    tx_receipt_number: Option<String>,
    tx_created_at: Option<String>,
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

    // Build dynamic WHERE clause
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref tid) = q.transaction_id {
        let idx = bind_values.len() + 1;
        conditions.push(format!("al.transaction_id = ?{idx}"));
        bind_values.push(tid.clone());
    }
    if let Some(ref et) = q.event_type {
        let idx = bind_values.len() + 1;
        conditions.push(format!("al.event_type = ?{idx}"));
        bind_values.push(et.clone());
    }
    if let Some(ref pid) = q.performed_by_id {
        let idx = bind_values.len() + 1;
        conditions.push(format!("al.performed_by_id = ?{idx}"));
        bind_values.push(pid.clone());
    }
    if let Some(ref category) = q.category {
        let idx = bind_values.len() + 1;
        conditions.push(format!("t.category = ?{idx}"));
        bind_values.push(category.clone());
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

    // Count query
    let count_sql = format!(
        "SELECT COUNT(*)
         FROM audit_logs al
         LEFT JOIN transactions t ON al.transaction_id = t.id
         WHERE {where_clause}"
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for v in &bind_values {
        count_query = count_query.bind(v);
    }
    let total = count_query.fetch_one(&pool).await.unwrap_or(0);

    // List query with JOIN
    let list_sql = format!(
        "SELECT al.id, al.transaction_id, al.event_type, al.performed_by_id, al.created_at, al.transaction_snapshot,
                u.id AS user_id, u.name AS user_name, u.role AS user_role, u.member_id AS user_member_id,
                t.id AS tx_id, t.type AS tx_type, t.category AS tx_category, CAST(t.amount AS REAL) AS tx_amount,
                t.payment_mode AS tx_payment_mode, t.purpose AS tx_purpose, t.remark AS tx_remark,
                t.sponsor_purpose AS tx_sponsor_purpose, t.approval_status AS tx_approval_status,
                t.approval_source AS tx_approval_source,
                t.member_id AS tx_member_id, m.name AS tx_member_name, m.phone AS tx_member_phone,
                t.sponsor_id AS tx_sponsor_id, sp.name AS tx_sponsor_name, sp.company AS tx_sponsor_company, sp.phone AS tx_sponsor_phone,
                t.sender_name AS tx_sender_name,
                t.sender_phone AS tx_sender_phone, t.sender_upi_id AS tx_sender_upi_id,
                t.sender_bank_account AS tx_sender_bank_account, t.sender_bank_name AS tx_sender_bank_name,
                t.sponsor_sender_name AS tx_sponsor_sender_name, t.sponsor_sender_contact AS tx_sponsor_sender_contact,
                t.razorpay_payment_id AS tx_razorpay_payment_id, t.razorpay_order_id AS tx_razorpay_order_id,
                t.receipt_number AS tx_receipt_number, t.created_at AS tx_created_at
         FROM audit_logs al
         LEFT JOIN users u ON al.performed_by_id = u.id
         LEFT JOIN transactions t ON al.transaction_id = t.id
         LEFT JOIN members m ON t.member_id = m.id
         LEFT JOIN sponsors sp ON t.sponsor_id = sp.id
         WHERE {where_clause}
         ORDER BY al.created_at DESC
         LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2,
    );

    let mut list_query = sqlx::query_as::<_, AuditRow>(&list_sql);
    for v in &bind_values {
        list_query = list_query.bind(v);
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
            let transaction_snapshot = serde_json::from_str::<serde_json::Value>(&r.transaction_snapshot)
                .unwrap_or(serde_json::Value::Null);
            let approval_type = approval_labels::approval_type_from_entity(
                "TRANSACTION",
                r.tx_category.as_deref().or_else(|| {
                    transaction_snapshot
                        .get("category")
                        .and_then(|value| value.as_str())
                }),
            );
            let direction = approval_labels::direction_from_transaction_type(
                r.tx_type.as_deref().or_else(|| {
                    transaction_snapshot
                        .get("type")
                        .and_then(|value| value.as_str())
                }),
            );

            let performed_by = if r.user_id.is_some() {
                serde_json::json!({
                    "id": r.user_id,
                    "name": r.user_name,
                    "role": r.user_role,
                    "memberId": r.user_member_id,
                })
            } else {
                serde_json::Value::Null
            };

            let member_json = if r.tx_member_name.is_some() {
                serde_json::json!({
                    "id": r.tx_member_id,
                    "name": r.tx_member_name,
                    "phone": r.tx_member_phone,
                })
            } else {
                serde_json::Value::Null
            };

            let sponsor_json = if r.tx_sponsor_name.is_some() {
                serde_json::json!({
                    "id": r.tx_sponsor_id,
                    "name": r.tx_sponsor_name,
                    "company": r.tx_sponsor_company,
                    "phone": r.tx_sponsor_phone,
                })
            } else {
                serde_json::Value::Null
            };

            let transaction = r.tx_id.as_ref().map(|tx_id| {
                serde_json::json!({
                    "id": tx_id,
                    "type": r.tx_type,
                    "category": r.tx_category,
                    "amount": r.tx_amount.map(|amount| amount.to_string()),
                    "paymentMode": r.tx_payment_mode,
                    "purpose": r.tx_purpose,
                    "remark": r.tx_remark,
                    "sponsorPurpose": r.tx_sponsor_purpose,
                    "approvalStatus": r.tx_approval_status,
                    "approvalSource": r.tx_approval_source,
                    "memberId": r.tx_member_id,
                    "member": member_json,
                    "sponsorId": r.tx_sponsor_id,
                    "sponsor": sponsor_json,
                    "senderName": r.tx_sender_name,
                    "senderPhone": r.tx_sender_phone,
                    "senderUpiId": r.tx_sender_upi_id,
                    "senderBankAccount": r.tx_sender_bank_account,
                    "senderBankName": r.tx_sender_bank_name,
                    "sponsorSenderName": r.tx_sponsor_sender_name,
                    "sponsorSenderContact": r.tx_sponsor_sender_contact,
                    "razorpayPaymentId": r.tx_razorpay_payment_id,
                    "razorpayOrderId": r.tx_razorpay_order_id,
                    "receiptNumber": r.tx_receipt_number,
                    "receipt": serde_json::Value::Null,
                    "createdAt": r.tx_created_at,
                })
            }).unwrap_or(serde_json::Value::Null);

            serde_json::json!({
                "id": r.id,
                "transactionId": r.transaction_id,
                "eventType": r.event_type,
                "performedById": r.performed_by_id,
                "createdAt": r.created_at,
                "approvalType": approval_type,
                "approvalTypeLabel": approval_labels::approval_type_label(approval_type),
                "direction": direction,
                "transactionSnapshot": transaction_snapshot,
                "performedBy": performed_by,
                "transaction": transaction,
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
