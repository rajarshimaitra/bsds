use std::collections::HashMap;

use axum::{
    extract::State,
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use sqlx::SqlitePool;

use crate::{
    integrations::razorpay::{paise_to_rupees, verify_webhook_signature},
    repositories::{members, sponsor_links, transactions, users},
    services::{
        membership_service::{self, TransactionMembershipDetails},
        webhook_sponsor_handler::{self, RazorpayPaymentPayload},
    },
};

use super::AppError;

#[derive(Debug, Deserialize)]
struct RazorpayWebhookEvent {
    event: String,
    payload: RazorpayPayloadWrapper,
}

#[derive(Debug, Deserialize)]
struct RazorpayPayloadWrapper {
    payment: Option<RazorpayPaymentWrapper>,
}

#[derive(Debug, Deserialize)]
struct RazorpayPaymentWrapper {
    entity: RazorpayPaymentEntity,
}

#[derive(Debug, Deserialize)]
struct RazorpayPaymentEntity {
    id: String,
    order_id: Option<String>,
    amount: u64,
    status: String,
    method: String,
    vpa: Option<String>,
    bank: Option<String>,
    contact: Option<String>,
    email: Option<String>,
    #[serde(default)]
    notes: HashMap<String, String>,
    bank_transfer: Option<RazorpayBankTransfer>,
}

#[derive(Debug, Deserialize)]
struct RazorpayBankTransfer {
    payer_bank_account: Option<RazorpayPayerBankAccount>,
}

#[derive(Debug, Deserialize)]
struct RazorpayPayerBankAccount {
    account_number: Option<String>,
    bank_name: Option<String>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/razorpay", post(handle_razorpay_webhook))
}

async fn handle_razorpay_webhook(
    State(pool): State<SqlitePool>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<serde_json::Value>, AppError> {
    let signature = headers
        .get("x-razorpay-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let verified = verify_webhook_signature(&body, signature)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    if !verified {
        return Err(AppError::Unauthorized);
    }

    let event: RazorpayWebhookEvent =
        serde_json::from_str(&body).map_err(|error| AppError::BadRequest(error.to_string()))?;

    if event.event != "payment.captured" {
        return Ok(Json(serde_json::json!({
            "ok": true,
            "ignored": true,
            "event": event.event,
        })));
    }

    let payment = event
        .payload
        .payment
        .map(|item| item.entity)
        .ok_or_else(|| AppError::BadRequest("Missing payment payload".to_string()))?;

    if payment.status != "captured" {
        return Ok(Json(serde_json::json!({
            "ok": true,
            "ignored": true,
            "reason": "payment_not_captured",
        })));
    }

    let system_user_id = get_or_create_system_user(&pool).await?;

    if is_sponsor_payment(&payment.notes) {
        let result = webhook_sponsor_handler::handle_sponsor_webhook_payment(
            &pool,
            &RazorpayPaymentPayload {
                razorpay_payment_id: payment.id.clone(),
                razorpay_order_id: payment.order_id.clone(),
                amount_paise: payment.amount,
                method: payment.method.clone(),
                upi_vpa: payment.vpa.clone(),
                bank_name: payment
                    .bank_transfer
                    .as_ref()
                    .and_then(|item| item.payer_bank_account.as_ref())
                    .and_then(|item| item.bank_name.clone())
                    .or(payment.bank.clone()),
                sender_bank_account: payment
                    .bank_transfer
                    .as_ref()
                    .and_then(|item| item.payer_bank_account.as_ref())
                    .and_then(|item| item.account_number.clone()),
                contact: payment.contact.clone(),
                email: payment.email.clone(),
                notes: payment.notes.clone(),
            },
            &system_user_id,
        )
        .await;

        if result.success && !result.already_processed {
            if let Some(token) = payment.notes.get("sponsorLinkToken") {
                if let Ok(Some(link)) = sponsor_links::find_by_token(&pool, token).await {
                    let _ = sponsor_links::update_payment_status(
                        &pool,
                        &link.id,
                        false,
                        Some(&payment.id),
                    )
                    .await;
                }
            }
        }

        return Ok(Json(serde_json::json!({
            "ok": result.success,
            "kind": "sponsor",
            "alreadyProcessed": result.already_processed,
            "transactionId": result.transaction_id,
            "receiptNumber": result.receipt_number,
            "error": result.error,
        })));
    }

    let member_id = payment
        .notes
        .get("memberId")
        .cloned()
        .ok_or_else(|| AppError::BadRequest("Missing memberId in payment notes".to_string()))?;

    let existing = sqlx::query_scalar::<_, String>(
        "SELECT id FROM transactions WHERE razorpay_payment_id = ?1 LIMIT 1",
    )
    .bind(&payment.id)
    .fetch_optional(&pool)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    if let Some(transaction_id) = existing {
        return Ok(Json(serde_json::json!({
            "ok": true,
            "kind": "membership",
            "alreadyProcessed": true,
            "transactionId": transaction_id,
        })));
    }

    let member = members::find_by_id(&pool, &member_id)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .ok_or(AppError::NotFound)?;

    let amount = paise_to_rupees(payment.amount);
    let fee_type = payment
        .notes
        .get("feeType")
        .cloned()
        .unwrap_or_else(|| "SUBSCRIPTION".to_string());
    let membership_type = payment.notes.get("membershipType").cloned();
    let includes_annual_fee = parse_note_bool(&payment.notes, "includesAnnualFee");
    let includes_subscription = parse_note_bool(&payment.notes, "includesSubscription");
    let includes_application_fee = parse_note_bool(&payment.notes, "isApplicationFee");
        // legacy notes use isApplicationFee; keep that signal as the source of truth

    let tx = transactions::create(
        &pool,
        &transactions::CreateTransactionData {
            r#type: "CASH_IN".to_string(),
            category: "MEMBERSHIP".to_string(),
            amount,
            payment_mode: payment_mode_from_method(&payment.method).to_string(),
            purpose: build_membership_purpose(&member.name, &fee_type, membership_type.as_deref()),
            remark: None,
            sponsor_purpose: None,
            member_id: Some(member_id.clone()),
            sponsor_id: None,
            entered_by_id: system_user_id.clone(),
            approval_status: "APPROVED".to_string(),
            approval_source: "RAZORPAY_WEBHOOK".to_string(),
            approved_by_id: Some(system_user_id.clone()),
            approved_at: Some(now_iso()),
            razorpay_payment_id: Some(payment.id.clone()),
            razorpay_order_id: payment.order_id.clone(),
            sender_name: payment.notes.get("memberName").cloned(),
            sender_phone: payment.contact.clone(),
            sender_upi_id: payment.vpa.clone(),
            sender_bank_account: payment
                .bank_transfer
                .as_ref()
                .and_then(|item| item.payer_bank_account.as_ref())
                .and_then(|item| item.account_number.clone()),
            sender_bank_name: payment
                .bank_transfer
                .as_ref()
                .and_then(|item| item.payer_bank_account.as_ref())
                .and_then(|item| item.bank_name.clone())
                .or(payment.bank.clone()),
            sponsor_sender_name: None,
            sponsor_sender_contact: None,
            receipt_number: None,
            includes_subscription,
            includes_annual_fee,
            includes_application_fee,
        },
    )
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    let details = TransactionMembershipDetails {
        membership_type,
        fee_type: Some(fee_type),
        is_application_fee: includes_application_fee,
        includes_subscription,
        includes_annual_fee,
        includes_application_fee,
    };

    membership_service::apply_membership_status_from_transaction(
        &pool,
        &member_id,
        "MEMBERSHIP",
        amount,
        &details,
    )
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "kind": "membership",
        "alreadyProcessed": false,
        "transactionId": tx.id,
    })))
}

async fn get_or_create_system_user(pool: &SqlitePool) -> Result<String, AppError> {
    if let Some(user) = users::find_by_email(pool, "system@bsds.local")
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
    {
        return Ok(user.id);
    }

    if let Some(admin_id) = sqlx::query_scalar::<_, String>(
        "SELECT id FROM users WHERE role = 'ADMIN' LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?
    {
        return Ok(admin_id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (id, member_id, name, email, phone, address, password, is_temp_password, role, membership_status, application_fee_paid)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 'ADMIN', 'ACTIVE', 1)",
    )
    .bind(&id)
    .bind("BSDS-SYSTEM-0000-00")
    .bind("SYSTEM")
    .bind("system@bsds.local")
    .bind("+910000000000")
    .bind("System")
    .bind("NOT_A_REAL_PASSWORD")
    .execute(pool)
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(id)
}

fn is_sponsor_payment(notes: &HashMap<String, String>) -> bool {
    notes.contains_key("sponsorLinkToken") || notes.contains_key("sponsorPurpose")
}

fn parse_note_bool(notes: &HashMap<String, String>, key: &str) -> bool {
    notes.get(key).map(|value| value == "true").unwrap_or(false)
}

fn payment_mode_from_method(method: &str) -> &'static str {
    match method {
        "netbanking" | "bank_transfer" => "BANK_TRANSFER",
        "cash" => "CASH",
        _ => "UPI",
    }
}

fn build_membership_purpose(member_name: &str, fee_type: &str, membership_type: Option<&str>) -> String {
    if fee_type == "ANNUAL_FEE" {
        return format!("Annual membership fee — {member_name}");
    }

    match membership_type.unwrap_or("Membership") {
        "MONTHLY" => format!("Monthly subscription — {member_name}"),
        "HALF_YEARLY" => format!("Half yearly subscription — {member_name}"),
        "ANNUAL" => format!("Annual subscription — {member_name}"),
        other => format!("{other} subscription — {member_name}"),
    }
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

use serde::Deserialize;
