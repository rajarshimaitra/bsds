//! Notification Service — high-level WhatsApp notification functions.
//!
//! Each function:
//!   1. Resolves the entity from the DB
//!   2. Determines the recipient set (admins, operators, member, sub-members)
//!   3. Calls WhatsappClient::send_message for each recipient
//!   4. Logs the outcome to ActivityLog (regardless of WhatsApp success)
//!   5. Returns { sent, failed }
//!
//! Design rules:
//!   - Never panics — all DB and WhatsApp failures are caught and logged
//!   - If WhatsApp is not configured, messages are skipped silently
//!   - All notifications are logged to ActivityLog for audit trail

use sqlx::SqlitePool;

use crate::db::models::{Approval, Sponsor, Transaction, User};
use crate::integrations::whatsapp::WhatsappClient;
use crate::repositories::activity_logs;

// ---------------------------------------------------------------------------
// Template registry
// ---------------------------------------------------------------------------

/// Pre-approved Meta WhatsApp Business template names.
pub mod templates {
    pub const NEW_APPROVAL: &str = "new_approval_request";
    pub const PAYMENT_RECEIVED: &str = "payment_received";
    pub const NEW_MEMBER: &str = "new_member_registration";
    pub const MEMBERSHIP_APPROVED: &str = "membership_approved";
    pub const EXPIRY_REMINDER: &str = "expiry_reminder";
    pub const MEMBERSHIP_EXPIRED: &str = "membership_expired";
    pub const SPONSOR_PAYMENT: &str = "sponsor_payment";
    pub const REJECTION: &str = "rejection_notice";
}

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NotificationResult {
    pub sent: u32,
    pub failed: u32,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Fetch all admin and operator users with a phone number.
async fn get_admins_and_operators(
    pool: &SqlitePool,
) -> Vec<(String, String, String)> {
    sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, phone, name FROM users WHERE role IN ('ADMIN', 'OPERATOR')",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Find a system user ID for activity logging.
async fn get_system_user_id(pool: &SqlitePool) -> String {
    let system_user: Option<String> =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'system@bsds.local' LIMIT 1")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    if let Some(id) = system_user {
        return id;
    }

    let admin: Option<String> =
        sqlx::query_scalar("SELECT id FROM users WHERE role = 'ADMIN' LIMIT 1")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    admin.unwrap_or_else(|| "system".to_string())
}

/// Send a template message to a list of recipients.
async fn send_to_recipients(
    client: &WhatsappClient,
    recipients: &[(String, String, String)],
    template_name: &str,
    params: &[String],
) -> NotificationResult {
    let mut result = NotificationResult { sent: 0, failed: 0 };

    for (_, phone, _) in recipients {
        let r = client
            .send_message(phone, template_name, params, None)
            .await;
        if r.success {
            result.sent += 1;
        } else {
            result.failed += 1;
        }
    }

    result
}

/// Formats a date string as DD/MM/YYYY for display in messages.
fn format_date_display(date_str: &str) -> String {
    // Input: "YYYY-MM-DD..." or "YYYY-MM-DDTHH:MM:SSZ"
    if date_str.len() >= 10 {
        let parts: Vec<&str> = date_str[..10].split('-').collect();
        if parts.len() == 3 {
            return format!("{}/{}/{}", parts[2], parts[1], parts[0]);
        }
    }
    date_str.to_string()
}

// ---------------------------------------------------------------------------
// Public notification functions
// ---------------------------------------------------------------------------

/// Notify admins and operators about a new approval request.
pub async fn notify_new_approval_request(
    pool: &SqlitePool,
    client: Option<&WhatsappClient>,
    approval: &Approval,
    requester_name: &str,
) -> NotificationResult {
    let system_user_id = get_system_user_id(pool).await;
    let mut result = NotificationResult { sent: 0, failed: 0 };

    if let Some(client) = client {
        let recipients = get_admins_and_operators(pool).await;
        if !recipients.is_empty() {
            let entity_type = approval.entity_type.replace('_', " ");
            let r = send_to_recipients(
                client,
                &recipients,
                templates::NEW_APPROVAL,
                &[entity_type, requester_name.to_string()],
            )
            .await;
            result.sent += r.sent;
            result.failed += r.failed;
        }
    }

    log_activity(
        pool,
        &system_user_id,
        "whatsapp_notification_sent",
        &format!(
            "New approval request notification: sent={} failed={}",
            result.sent, result.failed
        ),
        Some(serde_json::json!({
            "approvalId": approval.id,
            "entityType": approval.entity_type,
            "sent": result.sent,
            "failed": result.failed,
        })),
    )
    .await;

    result
}

/// Notify admins and operators that a payment was received.
pub async fn notify_payment_received(
    pool: &SqlitePool,
    client: Option<&WhatsappClient>,
    transaction: &Transaction,
    member_name: Option<&str>,
) -> NotificationResult {
    let system_user_id = get_system_user_id(pool).await;
    let mut result = NotificationResult { sent: 0, failed: 0 };

    if let Some(client) = client {
        let recipients = get_admins_and_operators(pool).await;
        if !recipients.is_empty() {
            let amount = format!("Rs. {}", transaction.amount as i64);
            let name = member_name
                .or(transaction.sender_name.as_deref())
                .unwrap_or("Unknown")
                .to_string();
            let payment_mode = transaction.payment_mode.clone();

            let r = send_to_recipients(
                client,
                &recipients,
                templates::PAYMENT_RECEIVED,
                &[amount, name, payment_mode],
            )
            .await;
            result.sent += r.sent;
            result.failed += r.failed;
        }
    }

    log_activity(
        pool,
        &system_user_id,
        "whatsapp_notification_sent",
        &format!(
            "Payment received notification: sent={} failed={}",
            result.sent, result.failed
        ),
        Some(serde_json::json!({
            "transactionId": transaction.id,
            "amount": transaction.amount.to_string(),
            "sent": result.sent,
            "failed": result.failed,
        })),
    )
    .await;

    result
}

/// Notify admins and operators about a new member registration.
pub async fn notify_new_member_registration(
    pool: &SqlitePool,
    client: Option<&WhatsappClient>,
    user: &User,
) -> NotificationResult {
    let system_user_id = get_system_user_id(pool).await;
    let mut result = NotificationResult { sent: 0, failed: 0 };

    if let Some(client) = client {
        let recipients = get_admins_and_operators(pool).await;
        if !recipients.is_empty() {
            let r = send_to_recipients(
                client,
                &recipients,
                templates::NEW_MEMBER,
                &[user.name.clone(), user.member_id.clone()],
            )
            .await;
            result.sent += r.sent;
            result.failed += r.failed;
        }
    }

    log_activity(
        pool,
        &system_user_id,
        "whatsapp_notification_sent",
        &format!(
            "New member registration notification: sent={} failed={}",
            result.sent, result.failed
        ),
        None,
    )
    .await;

    result
}

/// Notify staff, member, and sub-members that a membership was approved.
pub async fn notify_membership_approved(
    pool: &SqlitePool,
    client: Option<&WhatsappClient>,
    user: &User,
    temp_password: &str,
    login_url: &str,
) -> NotificationResult {
    let system_user_id = get_system_user_id(pool).await;
    let mut result = NotificationResult { sent: 0, failed: 0 };

    if let Some(client) = client {
        let params = vec![
            user.name.clone(),
            login_url.to_string(),
            user.email.clone(),
            temp_password.to_string(),
        ];

        // Staff
        let staff = get_admins_and_operators(pool).await;
        if !staff.is_empty() {
            let r = send_to_recipients(client, &staff, templates::MEMBERSHIP_APPROVED, &params)
                .await;
            result.sent += r.sent;
            result.failed += r.failed;
        }

        // Member themselves
        let r = client
            .send_message(&user.phone, templates::MEMBERSHIP_APPROVED, &params, None)
            .await;
        if r.success {
            result.sent += 1;
        } else {
            result.failed += 1;
        }

        // Sub-members
        let sub_phones: Vec<String> = sqlx::query_scalar(
            "SELECT phone FROM sub_members WHERE parent_user_id = ?1",
        )
        .bind(&user.id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for phone in &sub_phones {
            let r = client
                .send_message(phone, templates::MEMBERSHIP_APPROVED, &params, None)
                .await;
            if r.success {
                result.sent += 1;
            } else {
                result.failed += 1;
            }
        }
    }

    log_activity(
        pool,
        &system_user_id,
        "whatsapp_notification_sent",
        &format!(
            "Membership approved notification: sent={} failed={}",
            result.sent, result.failed
        ),
        None,
    )
    .await;

    result
}

/// Remind a member and sub-members about upcoming membership expiry.
pub async fn notify_membership_expiry_reminder(
    pool: &SqlitePool,
    client: Option<&WhatsappClient>,
    user: &User,
    days_left: i64,
) -> NotificationResult {
    let system_user_id = get_system_user_id(pool).await;
    let mut result = NotificationResult { sent: 0, failed: 0 };

    if let Some(client) = client {
        let expiry_date = user
            .membership_expiry
            .as_deref()
            .map(format_date_display)
            .unwrap_or_else(|| "N/A".to_string());

        let params = vec![
            user.name.clone(),
            days_left.to_string(),
            expiry_date,
        ];

        // Member
        let r = client
            .send_message(&user.phone, templates::EXPIRY_REMINDER, &params, None)
            .await;
        if r.success {
            result.sent += 1;
        } else {
            result.failed += 1;
        }

        // Sub-members
        let sub_phones: Vec<String> = sqlx::query_scalar(
            "SELECT phone FROM sub_members WHERE parent_user_id = ?1",
        )
        .bind(&user.id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for phone in &sub_phones {
            let r = client
                .send_message(phone, templates::EXPIRY_REMINDER, &params, None)
                .await;
            if r.success {
                result.sent += 1;
            } else {
                result.failed += 1;
            }
        }
    }

    log_activity(
        pool,
        &system_user_id,
        "whatsapp_notification_sent",
        &format!(
            "Membership expiry reminder notification ({days_left} days): sent={} failed={}",
            result.sent, result.failed
        ),
        None,
    )
    .await;

    result
}

/// Notify member, sub-members, admins, and operators that a membership has expired.
pub async fn notify_membership_expired(
    pool: &SqlitePool,
    client: Option<&WhatsappClient>,
    user: &User,
) -> NotificationResult {
    let system_user_id = get_system_user_id(pool).await;
    let mut result = NotificationResult { sent: 0, failed: 0 };

    if let Some(client) = client {
        let params = vec![user.name.clone(), user.member_id.clone()];

        // Member
        let r = client
            .send_message(&user.phone, templates::MEMBERSHIP_EXPIRED, &params, None)
            .await;
        if r.success {
            result.sent += 1;
        } else {
            result.failed += 1;
        }

        // Sub-members
        let sub_phones: Vec<String> = sqlx::query_scalar(
            "SELECT phone FROM sub_members WHERE parent_user_id = ?1",
        )
        .bind(&user.id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for phone in &sub_phones {
            let r = client
                .send_message(phone, templates::MEMBERSHIP_EXPIRED, &params, None)
                .await;
            if r.success {
                result.sent += 1;
            } else {
                result.failed += 1;
            }
        }

        // Staff
        let staff = get_admins_and_operators(pool).await;
        if !staff.is_empty() {
            let r = send_to_recipients(client, &staff, templates::MEMBERSHIP_EXPIRED, &params)
                .await;
            result.sent += r.sent;
            result.failed += r.failed;
        }
    }

    log_activity(
        pool,
        &system_user_id,
        "whatsapp_notification_sent",
        &format!(
            "Membership expired notification: sent={} failed={}",
            result.sent, result.failed
        ),
        None,
    )
    .await;

    result
}

/// Notify admins and operators about a sponsor payment.
pub async fn notify_sponsor_payment(
    pool: &SqlitePool,
    client: Option<&WhatsappClient>,
    transaction: &Transaction,
    sponsor: &Sponsor,
) -> NotificationResult {
    let system_user_id = get_system_user_id(pool).await;
    let mut result = NotificationResult { sent: 0, failed: 0 };

    if let Some(client) = client {
        let recipients = get_admins_and_operators(pool).await;
        if !recipients.is_empty() {
            let amount = format!("Rs. {}", transaction.amount as i64);
            let purpose = transaction
                .sponsor_purpose
                .as_deref()
                .map(|p| p.replace('_', " "))
                .unwrap_or_else(|| "Sponsorship".to_string());

            let r = send_to_recipients(
                client,
                &recipients,
                templates::SPONSOR_PAYMENT,
                &[sponsor.name.clone(), amount, purpose],
            )
            .await;
            result.sent += r.sent;
            result.failed += r.failed;
        }
    }

    log_activity(
        pool,
        &system_user_id,
        "whatsapp_notification_sent",
        &format!(
            "Sponsor payment notification: sent={} failed={}",
            result.sent, result.failed
        ),
        None,
    )
    .await;

    result
}

/// Notify the operator who submitted a request that it was rejected.
pub async fn notify_rejection(
    pool: &SqlitePool,
    client: Option<&WhatsappClient>,
    approval: &Approval,
    operator_phone: &str,
) -> NotificationResult {
    let system_user_id = get_system_user_id(pool).await;
    let mut result = NotificationResult { sent: 0, failed: 0 };

    if let Some(client) = client {
        let entity_type = approval.entity_type.replace('_', " ");
        let reason = approval
            .notes
            .as_deref()
            .unwrap_or("No reason provided")
            .to_string();

        let r = client
            .send_message(
                operator_phone,
                templates::REJECTION,
                &[entity_type, reason],
                None,
            )
            .await;
        if r.success {
            result.sent += 1;
        } else {
            result.failed += 1;
        }
    }

    log_activity(
        pool,
        &system_user_id,
        "whatsapp_notification_sent",
        &format!(
            "Rejection notification to operator: sent={} failed={}",
            result.sent, result.failed
        ),
        None,
    )
    .await;

    result
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn log_activity(
    pool: &SqlitePool,
    user_id: &str,
    action: &str,
    description: &str,
    metadata: Option<serde_json::Value>,
) {
    let _ = activity_logs::create(
        pool,
        &activity_logs::CreateActivityLogData {
            user_id: user_id.to_string(),
            action: action.to_string(),
            description: description.to_string(),
            metadata: metadata.map(|m| serde_json::to_string(&m).unwrap_or_default()),
        },
    )
    .await;
}
