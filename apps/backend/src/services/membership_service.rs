//! Membership Service — business logic for membership period management.
//!
//! Fee rules:
//!   Monthly     Rs.250 (30 days)
//!   Half-yearly Rs.1,500 (180 days)
//!   Annual      Rs.3,000 (365 days)
//!   Application fee Rs.10,000 — one-time, only if User.application_fee_paid === false
//!
//! No partial payments: amount must match the fee for the selected type exactly.

use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::db::models::{Membership, Transaction};
use crate::integrations::whatsapp::WhatsappClient;
use crate::repositories::{activity_logs, approvals, audit_logs, memberships, transactions, members, users};
use crate::services::notification_service;
use crate::support::{approval_labels, audit};
use crate::support::membership_rules::{
    MembershipType, calculate_annual_fee_dates, calculate_membership_dates,
};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum MembershipServiceError {
    #[error("Member not found")]
    MemberNotFound,
    #[error("Membership not found")]
    MembershipNotFound,
    #[error("Member has no linked user account")]
    NoLinkedUser,
    #[error("Application fee has already been paid for this member")]
    ApplicationFeeAlreadyPaid,
    #[error("Membership is already {0}")]
    AlreadyProcessed(String),
    #[error("{0}")]
    Validation(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Caller identity.
#[derive(Debug, Clone)]
pub struct RequestedBy {
    pub id: String,
    pub role: String,
    pub name: String,
}

/// Result of membership creation.
#[derive(Debug, Clone)]
pub struct CreateMembershipResult {
    pub action: String,
    pub transaction_id: Option<String>,
    pub approval_id: Option<String>,
}

/// Details about the membership components in a transaction.
#[derive(Debug, Clone, Default)]
pub struct TransactionMembershipDetails {
    pub membership_type: Option<String>,
    pub fee_type: Option<String>,
    pub is_application_fee: bool,
    pub includes_subscription: bool,
    pub includes_annual_fee: bool,
    pub includes_application_fee: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct MembershipCronSummary {
    pub processed: u32,
    pub reminded: u32,
    pub expired: u32,
}

/// Input for creating a membership.
#[derive(Debug, Clone)]
pub struct CreateMembershipInput {
    pub member_id: String,
    pub r#type: String,
    pub amount: f64,
    pub fee_type: Option<String>,
    pub is_application_fee: Option<bool>,
    /// Explicit flag: payment includes a subscription fee component.
    pub includes_subscription: Option<bool>,
    /// Explicit flag: payment includes the annual membership fee component.
    pub includes_annual_fee: Option<bool>,
}

// ---------------------------------------------------------------------------
// List memberships
// ---------------------------------------------------------------------------

/// List all membership periods for a specific member.
pub async fn get_memberships_by_member(
    pool: &SqlitePool,
    member_id: &str,
) -> Result<Vec<Membership>, MembershipServiceError> {
    let member = members::find_by_id(pool, member_id)
        .await?
        .ok_or(MembershipServiceError::MemberNotFound)?;

    memberships::list_by_member_id(pool, &member.id)
        .await
        .map_err(Into::into)
}

/// Get a single membership by ID.
pub async fn get_membership(
    pool: &SqlitePool,
    membership_id: &str,
) -> Result<Membership, MembershipServiceError> {
    memberships::find_by_id(pool, membership_id)
        .await?
        .ok_or(MembershipServiceError::MembershipNotFound)
}

// ---------------------------------------------------------------------------
// Create membership
// ---------------------------------------------------------------------------

/// Create a new membership period for a member.
///
/// Business rules:
/// 1. Amount must exactly match the fee for the selected type.
/// 2. Application fee is one-time and only valid if not already paid.
/// 3. Start date = today or day after current membership expiry.
/// 4. Admin: status = APPROVED immediately.
/// 5. Operator: status = PENDING, Approval record created.
pub async fn create_membership(
    pool: &SqlitePool,
    data: &CreateMembershipInput,
    requested_by: &RequestedBy,
) -> Result<CreateMembershipResult, MembershipServiceError> {
    // 1. Resolve the Member record
    let member = members::find_by_id(pool, &data.member_id)
        .await?
        .ok_or(MembershipServiceError::MemberNotFound)?;

    // 2. Determine fee components.
    // Priority: explicit boolean flags > legacy fee_type string.
    let includes_annual_fee = data.includes_annual_fee.unwrap_or_else(|| {
        data.fee_type.as_deref() == Some("ANNUAL_FEE")
    });
    // Default: include subscription unless annual-only is explicitly requested.
    let includes_subscription = data.includes_subscription.unwrap_or(!includes_annual_fee);

    // Application fee only applies when a subscription component is included.
    let is_application_fee = if includes_annual_fee && !includes_subscription {
        false
    } else {
        data.is_application_fee.unwrap_or(false)
    };

    // Subscription type label (only meaningful when includes_subscription = true).
    let subscription_type_str = data.r#type.clone();

    // 3. Validate application fee usage
    if is_application_fee {
        let user = match &member.user_id {
            Some(uid) => users::find_by_id(pool, uid)
                .await?
                .ok_or(MembershipServiceError::NoLinkedUser)?,
            None => return Err(MembershipServiceError::NoLinkedUser),
        };
        if user.application_fee_paid {
            return Err(MembershipServiceError::ApplicationFeeAlreadyPaid);
        }
    }

    // 4. Validate combined amount: sum of all selected fee components must equal the submitted amount.
    {
        use crate::support::membership_rules::{membership_fee, ANNUAL_MEMBERSHIP_FEE, APPLICATION_FEE};
        let mut expected: u64 = 0;
        if includes_annual_fee {
            expected += ANNUAL_MEMBERSHIP_FEE;
        }
        if includes_subscription {
            let mt = MembershipType::from_str_label(&subscription_type_str)
                .ok_or_else(|| MembershipServiceError::Validation(
                    format!("Invalid membership type: {subscription_type_str}")
                ))?;
            expected += membership_fee(mt);
        }
        if is_application_fee {
            expected += APPLICATION_FEE;
        }
        if expected == 0 {
            return Err(MembershipServiceError::Validation(
                "No fee components selected".to_string(),
            ));
        }
        if data.amount as u64 != expected {
            return Err(MembershipServiceError::Validation(format!(
                "Amount must be exactly \u{20b9}{expected} for the selected fee components"
            )));
        }
    }

    // 5. Build purpose string
    let mut parts = Vec::new();
    if includes_annual_fee {
        parts.push("Annual membership fee".to_string());
    }
    if includes_subscription {
        let label = subscription_type_str
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default()
            + &subscription_type_str[1..].to_lowercase();
        parts.push(format!("{label} subscription"));
    }
    if is_application_fee {
        parts.push("application fee".to_string());
    }
    let purpose = format!("{} — {}", parts.join(" + "), member.name);

    // fee_type for membership record: ANNUAL_FEE when only annual, SUBSCRIPTION otherwise
    let effective_fee_type = if includes_annual_fee && !includes_subscription {
        "ANNUAL_FEE"
    } else {
        "SUBSCRIPTION"
    };

    let is_admin = requested_by.role == "ADMIN";
    let is_pending = !is_admin;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // 6. Create Transaction record
    let tx = transactions::create(
        pool,
        &transactions::CreateTransactionData {
            r#type: "CASH_IN".to_string(),
            category: "MEMBERSHIP".to_string(),
            amount: data.amount,
            payment_mode: "CASH".to_string(),
            purpose,
            remark: None,
            sponsor_purpose: None,
            member_id: Some(data.member_id.clone()),
            sponsor_id: None,
            entered_by_id: requested_by.id.clone(),
            approval_status: if is_pending { "PENDING" } else { "APPROVED" }.to_string(),
            approval_source: "MANUAL".to_string(),
            approved_by_id: if is_pending { None } else { Some(requested_by.id.clone()) },
            approved_at: if is_pending { None } else { Some(now.clone()) },
            razorpay_payment_id: None,
            razorpay_order_id: None,
            sender_name: None,
            sender_phone: None,
            sender_upi_id: None,
            sender_bank_account: None,
            sender_bank_name: None,
            sponsor_sender_name: None,
            sponsor_sender_contact: None,
            receipt_number: None,
            includes_subscription,
            includes_annual_fee,
            includes_application_fee: is_application_fee,
        },
    )
    .await?;

    let mut approval_id: Option<String> = None;
    if is_pending {
        let new_data = serde_json::json!({
            "memberId": data.member_id,
            "type": data.r#type,
            "amount": data.amount,
            "feeType": effective_fee_type,
            "isApplicationFee": is_application_fee,
            "includesSubscription": includes_subscription,
            "includesAnnualFee": includes_annual_fee,
            "includesApplicationFee": is_application_fee,
            "membershipType": if includes_subscription { subscription_type_str.clone() } else { "ANNUAL".to_string() },
        });

        let approval = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "TRANSACTION".to_string(),
                entity_id: tx.id.clone(),
                action: "add_transaction".to_string(),
                previous_data: None,
                new_data: Some(serde_json::to_string(&new_data).unwrap_or_default()),
                requested_by_id: requested_by.id.clone(),
                status: "PENDING".to_string(),
            },
        )
        .await?;
        approval_id = Some(approval.id);
    }

    // Admin direct-approval: create an auto-approved record so it appears in the approval queue history.
    if is_admin {
        let new_data_json = serde_json::to_string(&serde_json::json!({
            "type": "CASH_IN",
            "category": "MEMBERSHIP",
            "amount": data.amount,
            "paymentMode": "CASH",
            "purpose": tx.purpose,
            "membershipType": if includes_subscription { subscription_type_str.clone() } else { "ANNUAL".to_string() },
            "feeType": effective_fee_type,
        }))
        .unwrap_or_default();
        if let Ok(appr) = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "TRANSACTION".to_string(),
                entity_id: tx.id.clone(),
                action: "add_transaction".to_string(),
                previous_data: None,
                new_data: Some(new_data_json),
                requested_by_id: requested_by.id.clone(),
                status: "APPROVED".to_string(),
            },
        )
        .await
        {
            let _ = approvals::update_status(pool, &appr.id, "APPROVED", &requested_by.id, None).await;
            approval_id = Some(appr.id);
        }
    }

    // Admin: activate membership immediately
    if is_admin {
        let details = TransactionMembershipDetails {
            membership_type: if includes_subscription { Some(subscription_type_str.clone()) } else { None },
            fee_type: Some(effective_fee_type.to_string()),
            is_application_fee,
            includes_subscription,
            includes_annual_fee,
            includes_application_fee: is_application_fee,
        };
        apply_membership_status_from_transaction(pool, &data.member_id, "MEMBERSHIP", data.amount, &details).await?;
    }

    // Audit + activity logging
    if is_admin {
        let snapshot = audit::build_transaction_audit_snapshot(&transaction_to_snapshot_source(&tx));
        let _ = audit_logs::create(
            pool,
            &audit_logs::CreateAuditLogData {
                transaction_id: tx.id.clone(),
                event_type: "TRANSACTION_APPROVED".to_string(),
                transaction_snapshot: serde_json::to_string(&snapshot).unwrap_or_default(),
                performed_by_id: requested_by.id.clone(),
            },
        )
        .await;
    }

    let action_str = if is_pending {
        "transaction_created_pending"
    } else {
        "transaction_created"
    };
    let desc = if is_pending {
        format!(
            "{} submitted membership payment Rs.{} for {} (pending approval)",
            requested_by.name, data.amount, member.name
        )
    } else {
        format!(
            "Admin {} recorded membership payment Rs.{} for {}",
            requested_by.name, data.amount, member.name
        )
    };
    log_activity(
        pool,
        &requested_by.id,
        action_str,
        &desc,
        Some(serde_json::json!({
            "transactionId": tx.id.clone(),
            "transactionCategory": "MEMBERSHIP",
            "approvalType": approval_labels::MEMBERSHIP_PAYMENT_APPROVAL,
            "direction": approval_labels::INCOMING,
        })),
    )
    .await;

    Ok(CreateMembershipResult {
        action: if is_pending { "pending_approval" } else { "direct" }.to_string(),
        transaction_id: Some(tx.id),
        approval_id,
    })
}

// ---------------------------------------------------------------------------
// Approve / reject membership
// ---------------------------------------------------------------------------

/// Approve a pending membership.
pub async fn approve_membership(
    pool: &SqlitePool,
    membership_id: &str,
    approved_by: &RequestedBy,
) -> Result<(), MembershipServiceError> {
    let membership = memberships::find_by_id(pool, membership_id)
        .await?
        .ok_or(MembershipServiceError::MembershipNotFound)?;

    if membership.status != "PENDING" {
        return Err(MembershipServiceError::AlreadyProcessed(
            membership.status.clone(),
        ));
    }

    memberships::update_status(pool, membership_id, "APPROVED").await?;

    // Update User fields if member has a linked User
    let member = members::find_by_id(pool, &membership.member_id).await?;
    if let Some(member) = member {
        if let Some(ref user_id) = member.user_id {
            if membership.fee_type == "ANNUAL_FEE" {
                sqlx::query(
                    "UPDATE users SET membership_status = 'ACTIVE',
                     annual_fee_start = ?1, annual_fee_expiry = ?2, annual_fee_paid = 1,
                     total_paid = total_paid + ?3,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                     WHERE id = ?4",
                )
                .bind(&membership.start_date)
                .bind(&membership.end_date)
                .bind(membership.amount)
                .bind(user_id)
                .execute(pool)
                .await?;
            } else {
                let mut extra_set = String::new();
                if membership.is_application_fee {
                    extra_set = ", application_fee_paid = 1".to_string();
                }
                let sql = format!(
                    "UPDATE users SET membership_status = 'ACTIVE',
                     membership_type = ?1, membership_start = ?2, membership_expiry = ?3,
                     total_paid = total_paid + ?4{extra_set},
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                     WHERE id = ?5"
                );
                sqlx::query(&sql)
                    .bind(&membership.r#type)
                    .bind(&membership.start_date)
                    .bind(&membership.end_date)
                    .bind(membership.amount)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
        }
    }

    log_activity(
        pool,
        &approved_by.id,
        "membership_approved",
        &format!(
            "Admin {} approved membership {} ({})",
            approved_by.name, membership_id, membership.r#type
        ),
        Some(serde_json::json!({
            "membershipId": membership_id,
            "approvalType": approval_labels::MEMBERSHIP_APPROVAL,
        })),
    )
    .await;

    Ok(())
}

/// Reject a pending membership.
pub async fn reject_membership(
    pool: &SqlitePool,
    membership_id: &str,
    rejected_by: &RequestedBy,
    notes: Option<&str>,
) -> Result<(), MembershipServiceError> {
    let membership = memberships::find_by_id(pool, membership_id)
        .await?
        .ok_or(MembershipServiceError::MembershipNotFound)?;

    if membership.status != "PENDING" {
        return Err(MembershipServiceError::AlreadyProcessed(
            membership.status.clone(),
        ));
    }

    memberships::update_status(pool, membership_id, "REJECTED").await?;

    log_activity(
        pool,
        &rejected_by.id,
        "membership_rejected",
        &format!(
            "Admin {} rejected membership {}",
            rejected_by.name, membership_id
        ),
        Some(serde_json::json!({
            "membershipId": membership_id,
            "approvalType": approval_labels::MEMBERSHIP_APPROVAL,
            "notes": notes,
        })),
    )
    .await;

    Ok(())
}

// ---------------------------------------------------------------------------
// Expiry logic (for cron jobs)
// ---------------------------------------------------------------------------

/// List memberships expiring within the next N days.
pub async fn list_expiring_soon(
    pool: &SqlitePool,
    days_ahead: i64,
) -> Result<Vec<Membership>, MembershipServiceError> {
    memberships::list_expiring_soon(pool, days_ahead)
        .await
        .map_err(Into::into)
}

/// List memberships that have already expired.
pub async fn list_expired(
    pool: &SqlitePool,
) -> Result<Vec<Membership>, MembershipServiceError> {
    memberships::list_expired(pool).await.map_err(Into::into)
}

/// Auto-expire memberships and update User status to EXPIRED.
///
/// Called by the cron scheduler.
pub async fn auto_expire_memberships(
    pool: &SqlitePool,
) -> Result<u32, MembershipServiceError> {
    let expired = memberships::list_expired(pool).await?;
    let mut count = 0u32;

    for membership in &expired {
        // Update the User record to EXPIRED
        let member = members::find_by_id(pool, &membership.member_id).await?;
        if let Some(member) = member {
            if let Some(ref user_id) = member.user_id {
                sqlx::query(
                    "UPDATE users SET membership_status = 'EXPIRED',
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                     WHERE id = ?1 AND membership_status = 'ACTIVE'",
                )
                .bind(user_id)
                .execute(pool)
                .await?;
                count += 1;
            }
        }
    }

    Ok(count)
}

pub async fn run_daily_membership_cron(
    pool: &SqlitePool,
) -> Result<MembershipCronSummary, MembershipServiceError> {
    let reminder_window_days = crate::support::membership_rules::EXPIRY_REMINDER_DAYS as i64;
    let today = chrono::Utc::now().date_naive();
    let client = WhatsappClient::from_env();

    let expiring = memberships::list_expiring_soon(pool, reminder_window_days).await?;
    let expired = memberships::list_expired(pool).await?;

    let mut summary = MembershipCronSummary {
        processed: (expiring.len() + expired.len()) as u32,
        ..MembershipCronSummary::default()
    };

    for membership in &expiring {
        let member = match members::find_by_id(pool, &membership.member_id).await? {
            Some(member) => member,
            None => continue,
        };
        let user_id = match member.user_id.as_deref() {
            Some(user_id) => user_id,
            None => continue,
        };
        let user = match users::find_by_id(pool, user_id).await? {
            Some(user) => user,
            None => continue,
        };
        let end_date = match NaiveDate::parse_from_str(&membership.end_date[..10], "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => continue,
        };
        let days_left = (end_date - today).num_days();
        if days_left < 0 {
            continue;
        }

        notification_service::notify_membership_expiry_reminder(
            pool,
            client.as_ref(),
            &user,
            days_left,
        )
        .await;
        summary.reminded += 1;
    }

    for membership in &expired {
        let member = match members::find_by_id(pool, &membership.member_id).await? {
            Some(member) => member,
            None => continue,
        };
        let user_id = match member.user_id.as_deref() {
            Some(user_id) => user_id,
            None => continue,
        };
        let user = match users::find_by_id(pool, user_id).await? {
            Some(user) => user,
            None => continue,
        };

        notification_service::notify_membership_expired(pool, client.as_ref(), &user).await;
        summary.expired += 1;
    }

    auto_expire_memberships(pool).await?;

    Ok(summary)
}

// ---------------------------------------------------------------------------
// Apply membership status from transaction
// ---------------------------------------------------------------------------

/// Given an approved membership transaction, create the appropriate Membership
/// record(s) and update User status fields.
///
/// Called from approval service and transaction service.
pub async fn apply_membership_status_from_transaction(
    pool: &SqlitePool,
    member_id: &str,
    category: &str,
    amount: f64,
    details: &TransactionMembershipDetails,
) -> Result<(), MembershipServiceError> {
    if category != "MEMBERSHIP" {
        return Ok(());
    }

    let member = members::find_by_id(pool, member_id).await?;
    let member = match member {
        Some(m) => m,
        None => return Ok(()), // defensive
    };

    let user = match &member.user_id {
        Some(uid) => users::find_by_id(pool, uid).await?,
        None => None,
    };

    let membership_type = details.membership_type.as_deref();
    let is_application_fee = details.is_application_fee || details.includes_application_fee;
    let includes_annual_fee = details.includes_annual_fee;
    let includes_subscription = details.includes_subscription;

    let sub_type = if includes_subscription {
        membership_type
    } else {
        None
    };

    let today = chrono::Utc::now().date_naive();

    // Update total_paid and membership_status ONCE here, regardless of how many
    // fee components are in this payment (prevents double-counting on combined payments).
    let will_run_annual = includes_annual_fee;
    let will_run_subscription = sub_type.map(|st| MembershipType::from_str_label(st).is_some()).unwrap_or(false);
    if will_run_annual || will_run_subscription {
        if let Some(ref uid) = member.user_id {
            sqlx::query(
                "UPDATE users SET membership_status = 'ACTIVE',
                 total_paid = total_paid + ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE id = ?2",
            )
            .bind(amount)
            .bind(uid)
            .execute(pool)
            .await?;
        }
    }

    // Annual fee — create membership record and update date/status fields only.
    if includes_annual_fee {
        let current_expiry = user
            .as_ref()
            .and_then(|u| u.annual_fee_expiry.as_deref())
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().or_else(|| NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok()));

        let annual_dates = calculate_annual_fee_dates(current_expiry, today);

        memberships::create(
            pool,
            &memberships::CreateMembershipData {
                member_id: member_id.to_string(),
                r#type: "ANNUAL".to_string(),
                fee_type: "ANNUAL_FEE".to_string(),
                amount,
                start_date: annual_dates.start_date.to_string(),
                end_date: annual_dates.end_date.to_string(),
                is_application_fee: false,
                status: "APPROVED".to_string(),
            },
        )
        .await?;

        if let Some(ref uid) = member.user_id {
            sqlx::query(
                "UPDATE users SET annual_fee_start = ?1, annual_fee_expiry = ?2, annual_fee_paid = 1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE id = ?3",
            )
            .bind(annual_dates.start_date.to_string())
            .bind(annual_dates.end_date.to_string())
            .bind(uid)
            .execute(pool)
            .await?;
        }
    }

    // Subscription — create membership record and update date/status fields only.
    if let Some(st) = sub_type {
        let mt = MembershipType::from_str_label(st);
        if let Some(mt) = mt {
            let current_expiry = user
                .as_ref()
                .and_then(|u| u.membership_expiry.as_deref())
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().or_else(|| NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok()));

            let sub_dates = calculate_membership_dates(mt, current_expiry, today);

            memberships::create(
                pool,
                &memberships::CreateMembershipData {
                    member_id: member_id.to_string(),
                    r#type: st.to_string(),
                    fee_type: "SUBSCRIPTION".to_string(),
                    amount,
                    start_date: sub_dates.start_date.to_string(),
                    end_date: sub_dates.end_date.to_string(),
                    is_application_fee,
                    status: "APPROVED".to_string(),
                },
            )
            .await?;

            if let Some(ref uid) = member.user_id {
                let mut extra_set = String::new();
                if is_application_fee {
                    let already_paid = user.as_ref().map(|u| u.application_fee_paid).unwrap_or(false);
                    if !already_paid {
                        extra_set = ", application_fee_paid = 1".to_string();
                    }
                }
                let sql = format!(
                    "UPDATE users SET membership_type = ?1, membership_start = ?2, membership_expiry = ?3{extra_set},
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                     WHERE id = ?4"
                );
                sqlx::query(&sql)
                    .bind(st)
                    .bind(sub_dates.start_date.to_string())
                    .bind(sub_dates.end_date.to_string())
                    .bind(uid)
                    .execute(pool)
                    .await?;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn transaction_to_snapshot_source(tx: &Transaction) -> audit::TransactionSnapshotSource {
    audit::TransactionSnapshotSource {
        id: tx.id.clone(),
        r#type: tx.r#type.clone(),
        category: tx.category.clone(),
        amount: tx.amount.to_string(),
        payment_mode: tx.payment_mode.clone(),
        purpose: tx.purpose.clone(),
        remark: tx.remark.clone(),
        sponsor_purpose: tx.sponsor_purpose.clone(),
        approval_status: tx.approval_status.clone(),
        approval_source: tx.approval_source.clone(),
        entered_by_id: Some(tx.entered_by_id.clone()),
        approved_by_id: tx.approved_by_id.clone(),
        approved_at: tx.approved_at.clone(),
        razorpay_payment_id: tx.razorpay_payment_id.clone(),
        razorpay_order_id: tx.razorpay_order_id.clone(),
        sender_name: tx.sender_name.clone(),
        sender_phone: tx.sender_phone.clone(),
        sender_upi_id: tx.sender_upi_id.clone(),
        sender_bank_account: tx.sender_bank_account.clone(),
        sender_bank_name: tx.sender_bank_name.clone(),
        sponsor_sender_name: tx.sponsor_sender_name.clone(),
        sponsor_sender_contact: tx.sponsor_sender_contact.clone(),
        receipt_number: tx.receipt_number.clone(),
        member_id: tx.member_id.clone(),
        sponsor_id: tx.sponsor_id.clone(),
        created_at: Some(tx.created_at.clone()),
    }
}

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
