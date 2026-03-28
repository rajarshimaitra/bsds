//! Webhook Sponsor Payment Handler — processes Razorpay webhook events
//! for sponsor payments.
//!
//! Called from the main Razorpay webhook handler when a payment contains
//! sponsor-related metadata in the notes field.
//!
//! On success:
//!   - Creates Transaction (category=SPONSORSHIP, auto-approved, source=RAZORPAY_WEBHOOK)
//!   - Logs to AuditLog + ActivityLog
//!   - Returns created transaction ID

use sqlx::SqlitePool;

use crate::repositories::{activity_logs, audit_logs, sponsors, transactions};
use crate::support::audit;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Subset of Razorpay payment event payload used by this handler.
#[derive(Debug, Clone)]
pub struct RazorpayPaymentPayload {
    /// Razorpay payment ID (e.g. "pay_XXXXXXXXXX")
    pub razorpay_payment_id: String,
    /// Razorpay order ID (e.g. "order_XXXXXXXXXX")
    pub razorpay_order_id: Option<String>,
    /// Amount in paise from Razorpay -- converted to INR by this handler.
    pub amount_paise: u64,
    /// Payment method: "upi", "netbanking", "wallet", etc.
    pub method: String,
    /// UPI VPA if method === "upi"
    pub upi_vpa: Option<String>,
    /// Bank name if netbanking
    pub bank_name: Option<String>,
    /// Masked account number if bank transfer
    pub sender_bank_account: Option<String>,
    /// Payer contact (phone)
    pub contact: Option<String>,
    /// Payer email
    pub email: Option<String>,
    /// Notes object from Razorpay order
    pub notes: std::collections::HashMap<String, String>,
}

/// Result of processing a sponsor webhook payment.
#[derive(Debug, Clone)]
pub struct SponsorWebhookResult {
    pub success: bool,
    pub transaction_id: Option<String>,
    pub receipt_number: Option<String>,
    pub error: Option<String>,
    /// True if this payment was already processed (idempotency).
    pub already_processed: bool,
}

// ---------------------------------------------------------------------------
// Valid sponsor purpose values
// ---------------------------------------------------------------------------

const VALID_SPONSOR_PURPOSES: &[&str] = &[
    "TITLE_SPONSOR",
    "GOLD_SPONSOR",
    "SILVER_SPONSOR",
    "FOOD_PARTNER",
    "MEDIA_PARTNER",
    "STALL_VENDOR",
    "MARKETING_PARTNER",
];

fn is_valid_sponsor_purpose(value: &str) -> bool {
    VALID_SPONSOR_PURPOSES.contains(&value)
}

// ---------------------------------------------------------------------------
// Payment method -> PaymentMode mapper
// ---------------------------------------------------------------------------

fn to_payment_mode(method: &str) -> &'static str {
    match method {
        "upi" => "UPI",
        "netbanking" => "BANK_TRANSFER",
        _ => "UPI", // default for other Razorpay methods
    }
}

// ---------------------------------------------------------------------------
// handleSponsorWebhookPayment
// ---------------------------------------------------------------------------

/// Process a Razorpay payment webhook event for a sponsor payment.
///
/// Idempotent: checks razorpay_payment_id for duplicates before creating.
pub async fn handle_sponsor_webhook_payment(
    pool: &SqlitePool,
    payload: &RazorpayPaymentPayload,
    system_user_id: &str,
) -> SponsorWebhookResult {
    let sponsor_id = payload.notes.get("sponsorId").cloned();
    let sponsor_purpose = payload.notes.get("sponsorPurpose").cloned();
    let sender_name = payload
        .notes
        .get("sponsorName")
        .or_else(|| payload.notes.get("name"))
        .cloned();

    // Validate sponsorPurpose
    let sponsor_purpose = match sponsor_purpose {
        Some(ref sp) if is_valid_sponsor_purpose(sp) => sp.clone(),
        _ => {
            return SponsorWebhookResult {
                success: false,
                transaction_id: None,
                receipt_number: None,
                error: Some(format!(
                    "Invalid or missing sponsorPurpose in webhook notes: {}",
                    sponsor_purpose.as_deref().unwrap_or("(none)")
                )),
                already_processed: false,
            };
        }
    };

    // Idempotency: check if this payment was already processed
    let existing: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT id, receipt_number FROM transactions WHERE razorpay_payment_id = ?1 LIMIT 1",
    )
    .bind(&payload.razorpay_payment_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if let Some((id, receipt_number)) = existing {
        return SponsorWebhookResult {
            success: true,
            transaction_id: Some(id),
            receipt_number,
            error: None,
            already_processed: true,
        };
    }

    // Validate sponsorId if provided
    if let Some(ref sid) = sponsor_id {
        if sponsors::find_by_id(pool, sid).await.ok().flatten().is_none() {
            tracing::warn!(
                "[webhook-sponsor] Sponsor ID {} from notes not found in DB — creating transaction without sponsor link",
                sid
            );
        }
    }

    let amount_inr = payload.amount_paise as f64 / 100.0;
    let payment_mode = to_payment_mode(&payload.method);
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let tx_result = transactions::create(
        pool,
        &transactions::CreateTransactionData {
            r#type: "CASH_IN".to_string(),
            category: "SPONSORSHIP".to_string(),
            amount: amount_inr,
            payment_mode: payment_mode.to_string(),
            purpose: format!(
                "Sponsor payment via Razorpay — {}",
                sponsor_purpose.replace('_', " ")
            ),
            remark: None,
            sponsor_purpose: Some(sponsor_purpose.clone()),
            member_id: None,
            sponsor_id: sponsor_id.clone(),
            entered_by_id: system_user_id.to_string(),
            approval_status: "APPROVED".to_string(),
            approval_source: "RAZORPAY_WEBHOOK".to_string(),
            approved_by_id: None,
            approved_at: Some(now),
            razorpay_payment_id: Some(payload.razorpay_payment_id.clone()),
            razorpay_order_id: payload.razorpay_order_id.clone(),
            sender_name: sender_name.clone(),
            sender_phone: payload.contact.clone(),
            sender_upi_id: if payment_mode == "UPI" {
                payload.upi_vpa.clone()
            } else {
                None
            },
            sender_bank_account: if payment_mode == "BANK_TRANSFER" {
                payload.sender_bank_account.clone()
            } else {
                None
            },
            sender_bank_name: if payment_mode == "BANK_TRANSFER" {
                payload.bank_name.clone()
            } else {
                None
            },
            sponsor_sender_name: sender_name.clone(),
            sponsor_sender_contact: payload.contact.clone(),
            receipt_number: None,
            includes_subscription: false,
            includes_annual_fee: false,
            includes_application_fee: false,
        },
    )
    .await;

    let tx = match tx_result {
        Ok(tx) => tx,
        Err(err) => {
            return SponsorWebhookResult {
                success: false,
                transaction_id: None,
                receipt_number: None,
                error: Some(format!("Failed to create transaction: {err}")),
                already_processed: false,
            };
        }
    };

    // Audit log
    let snapshot = audit::build_transaction_audit_snapshot(&audit::TransactionSnapshotSource {
        id: tx.id.clone(),
        r#type: tx.r#type.clone(),
        category: tx.category.clone(),
        amount: tx.amount.to_string(),
        payment_mode: tx.payment_mode.clone(),
        purpose: tx.purpose.clone(),
        remark: tx.remark.clone(),
        sponsor_purpose: Some(sponsor_purpose.clone()),
        approval_status: tx.approval_status.clone(),
        approval_source: tx.approval_source.clone(),
        entered_by_id: Some(system_user_id.to_string()),
        approved_by_id: tx.approved_by_id.clone(),
        approved_at: tx.approved_at.clone(),
        razorpay_payment_id: Some(payload.razorpay_payment_id.clone()),
        razorpay_order_id: payload.razorpay_order_id.clone(),
        sender_name: sender_name.clone(),
        sender_phone: payload.contact.clone(),
        sender_upi_id: tx.sender_upi_id.clone(),
        sender_bank_account: tx.sender_bank_account.clone(),
        sender_bank_name: tx.sender_bank_name.clone(),
        sponsor_sender_name: tx.sponsor_sender_name.clone(),
        sponsor_sender_contact: tx.sponsor_sender_contact.clone(),
        receipt_number: tx.receipt_number.clone(),
        member_id: None,
        sponsor_id: sponsor_id.clone(),
        created_at: Some(tx.created_at.clone()),
    });

    let _ = audit_logs::create(
        pool,
        &audit_logs::CreateAuditLogData {
            transaction_id: tx.id.clone(),
            event_type: "TRANSACTION_APPROVED".to_string(),
            transaction_snapshot: serde_json::to_string(&snapshot).unwrap_or_default(),
            performed_by_id: system_user_id.to_string(),
        },
    )
    .await;

    // Activity log
    let sender_str = sender_name
        .as_deref()
        .map(|n| format!(" from {n}"))
        .unwrap_or_default();

    let _ = activity_logs::create(
        pool,
        &activity_logs::CreateActivityLogData {
            user_id: system_user_id.to_string(),
            action: "sponsor_payment_received".to_string(),
            description: format!(
                "Sponsor payment received via Razorpay: Rs.{} — {}{}",
                amount_inr,
                sponsor_purpose.replace('_', " "),
                sender_str,
            ),
            metadata: Some(
                serde_json::to_string(&serde_json::json!({
                    "transactionId": tx.id,
                    "razorpayPaymentId": payload.razorpay_payment_id,
                    "amount": amount_inr.to_string(),
                    "sponsorPurpose": sponsor_purpose,
                    "sponsorId": sponsor_id,
                    "paymentMode": payment_mode,
                }))
                .unwrap_or_default(),
            ),
        },
    )
    .await;

    SponsorWebhookResult {
        success: true,
        transaction_id: Some(tx.id),
        receipt_number: tx.receipt_number,
        error: None,
        already_processed: false,
    }
}

// ---------------------------------------------------------------------------
// isSponsorPayment
// ---------------------------------------------------------------------------

/// Check if a Razorpay webhook payment notes object indicates a sponsor payment.
pub fn is_sponsor_payment(notes: &std::collections::HashMap<String, String>) -> bool {
    notes
        .get("sponsorPurpose")
        .map(|sp| is_valid_sponsor_purpose(sp))
        .unwrap_or(false)
}
