//! Transaction Service — append-only cash in/out transaction management.
//!
//! Transactions are immutable after creation. Corrections happen via rejection
//! plus a brand-new replacement entry; update/delete are never allowed.

use sqlx::SqlitePool;

use crate::db::models::Transaction;
use crate::repositories::{activity_logs, approvals, audit_logs, transactions};
use crate::services::membership_service::{
    self, TransactionMembershipDetails,
};
use crate::services::receipt_service;
use crate::support::{approval_labels, audit};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum TransactionServiceError {
    #[error("Transaction not found")]
    NotFound,
    #[error("Only admins can reject transactions")]
    Forbidden,
    #[error("Transaction is already rejected")]
    AlreadyRejected,
    #[error("Transactions are immutable. Reject the existing payment and create a new one.")]
    Immutable,
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

/// Input for creating a transaction.
#[derive(Debug, Clone)]
pub struct CreateTransactionInput {
    pub r#type: String,
    pub category: String,
    pub amount: f64,
    pub payment_mode: String,
    pub purpose: String,
    pub remark: Option<String>,
    pub sponsor_purpose: Option<String>,
    pub member_id: Option<String>,
    pub sponsor_id: Option<String>,
    pub sender_name: Option<String>,
    pub sender_phone: Option<String>,
    pub sponsor_sender_name: Option<String>,
    pub sponsor_sender_contact: Option<String>,
    pub membership_type: Option<String>,
    pub fee_type: Option<String>,
    pub is_application_fee: Option<bool>,
    pub includes_subscription: Option<bool>,
    pub includes_annual_fee: Option<bool>,
    pub includes_application_fee: Option<bool>,
}

/// Result of a create operation.
#[derive(Debug, Clone)]
pub struct CreateTransactionResult {
    pub action: String,
    pub transaction_id: String,
    pub approval_id: Option<String>,
}

/// Aggregated transaction summary.
pub use crate::repositories::transactions::TransactionSummary;

// ---------------------------------------------------------------------------
// List transactions
// ---------------------------------------------------------------------------

/// List transactions with filters and pagination.
pub async fn list_transactions(
    pool: &SqlitePool,
    filters: &transactions::TransactionListFilters,
) -> Result<(Vec<Transaction>, i64), TransactionServiceError> {
    transactions::list(pool, filters)
        .await
        .map_err(Into::into)
}

/// Get a single transaction by ID.
pub async fn get_transaction(
    pool: &SqlitePool,
    id: &str,
) -> Result<Transaction, TransactionServiceError> {
    transactions::find_by_id(pool, id)
        .await?
        .ok_or(TransactionServiceError::NotFound)
}

// ---------------------------------------------------------------------------
// Create transaction
// ---------------------------------------------------------------------------

/// Create a new transaction.
///
/// Operator: creates with PENDING status + Approval record.
/// Admin: creates with APPROVED status, applies membership side effects if applicable.
pub async fn create_transaction(
    pool: &SqlitePool,
    data: &CreateTransactionInput,
    requested_by: &RequestedBy,
) -> Result<CreateTransactionResult, TransactionServiceError> {
    let is_operator = requested_by.role == "OPERATOR";
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let membership_details = TransactionMembershipDetails {
        membership_type: data.membership_type.clone(),
        fee_type: data.fee_type.clone(),
        is_application_fee: data.is_application_fee.unwrap_or(false)
            || data.includes_application_fee.unwrap_or(false),
        includes_subscription: data.includes_subscription.unwrap_or(false),
        includes_annual_fee: data.includes_annual_fee.unwrap_or(false),
        includes_application_fee: data.includes_application_fee.unwrap_or(false)
            || data.is_application_fee.unwrap_or(false),
    };

    let tx = transactions::create(
        pool,
        &transactions::CreateTransactionData {
            r#type: data.r#type.clone(),
            category: data.category.clone(),
            amount: data.amount,
            payment_mode: data.payment_mode.clone(),
            purpose: data.purpose.clone(),
            remark: data.remark.clone(),
            sponsor_purpose: data.sponsor_purpose.clone(),
            member_id: data.member_id.clone(),
            sponsor_id: data.sponsor_id.clone(),
            entered_by_id: requested_by.id.clone(),
            approval_status: if is_operator { "PENDING" } else { "APPROVED" }.to_string(),
            approval_source: "MANUAL".to_string(),
            approved_by_id: if is_operator {
                None
            } else {
                Some(requested_by.id.clone())
            },
            approved_at: if is_operator { None } else { Some(now.clone()) },
            razorpay_payment_id: None,
            razorpay_order_id: None,
            sender_name: data.sender_name.clone(),
            sender_phone: data.sender_phone.clone(),
            sender_upi_id: None,
            sender_bank_account: None,
            sender_bank_name: None,
            sponsor_sender_name: data.sponsor_sender_name.clone(),
            sponsor_sender_contact: data.sponsor_sender_contact.clone(),
            receipt_number: None,
            includes_subscription: data.includes_subscription.unwrap_or(false),
            includes_annual_fee: data.includes_annual_fee.unwrap_or(false),
            includes_application_fee: data.includes_application_fee.unwrap_or(false)
                || data.is_application_fee.unwrap_or(false),
        },
    )
    .await?;

    let approval_type = approval_labels::approval_type_from_entity(
        "TRANSACTION",
        Some(data.category.as_str()),
    );
    let direction = approval_labels::direction_from_transaction_type(Some(data.r#type.as_str()));

    let mut approval_id: Option<String> = None;
    let new_data_json = serde_json::to_string(&serde_json::json!({
        "type": data.r#type,
        "category": data.category,
        "amount": data.amount,
        "paymentMode": data.payment_mode,
        "purpose": data.purpose,
        "membershipType": data.membership_type,
        "feeType": data.fee_type,
    }))
    .unwrap_or_default();

    if is_operator {
        let approval = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "TRANSACTION".to_string(),
                entity_id: tx.id.clone(),
                action: "add_transaction".to_string(),
                previous_data: None,
                new_data: Some(new_data_json),
                requested_by_id: requested_by.id.clone(),
                status: "PENDING".to_string(),
            },
        )
        .await?;
        approval_id = Some(approval.id);
    } else {
        // Admin direct-approval: create an auto-approved record so it appears in the approval queue history.
        if let Ok(approval) = approvals::create(
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
            let _ = approvals::update_status(
                pool,
                &approval.id,
                "APPROVED",
                &requested_by.id,
                None,
            )
            .await;
            approval_id = Some(approval.id);
        }
    }

    // Admin direct path: if membership transaction with linked member,
    // create Membership record(s) and update member lifecycle status.
    if !is_operator && data.member_id.is_some() && data.category == "MEMBERSHIP" {
        let _ = membership_service::apply_membership_status_from_transaction(
            pool,
            data.member_id.as_ref().unwrap(),
            &data.category,
            data.amount,
            &membership_details,
        )
        .await;
    }

    // Audit log — TRANSACTION_CREATED always; TRANSACTION_APPROVED immediately for admin
    {
        let snapshot =
            audit::build_transaction_audit_snapshot(&transaction_to_snapshot_source(&tx));
        let snapshot_str = serde_json::to_string(&snapshot).unwrap_or_default();
        let _ = audit_logs::create(
            pool,
            &audit_logs::CreateAuditLogData {
                transaction_id: tx.id.clone(),
                event_type: "TRANSACTION_CREATED".to_string(),
                transaction_snapshot: snapshot_str.clone(),
                performed_by_id: requested_by.id.clone(),
            },
        )
        .await;
        if !is_operator {
            let _ = audit_logs::create(
                pool,
                &audit_logs::CreateAuditLogData {
                    transaction_id: tx.id.clone(),
                    event_type: "TRANSACTION_APPROVED".to_string(),
                    transaction_snapshot: snapshot_str,
                    performed_by_id: requested_by.id.clone(),
                },
            )
            .await;
        }
    }

    // Activity log
    let action_str = if is_operator {
        "transaction_requested"
    } else {
        "transaction_created"
    };
    let desc = if is_operator {
        format!(
            "Operator {} created pending transaction Rs.{} ({})",
            requested_by.name, tx.amount, tx.category
        )
    } else {
        format!(
            "Admin {} created transaction Rs.{} ({})",
            requested_by.name, tx.amount, tx.category
        )
    };
    log_activity(
        pool,
        &requested_by.id,
        action_str,
        &desc,
        Some(serde_json::json!({
            "transactionId": tx.id,
            "transactionCategory": data.category,
            "approvalType": approval_type,
            "direction": direction,
            "receiverContact": tx.sender_phone,
        })),
    )
    .await;

    // Issue receipt immediately for every CASH_IN MEMBERSHIP/SPONSORSHIP,
    // regardless of approval status (operator pending or admin approved).
    let _ = receipt_service::issue_receipt(
        pool,
        &tx.id,
        &requested_by.id,
        &requested_by.name,
        membership_details.membership_type.as_deref(),
    )
    .await;

    Ok(CreateTransactionResult {
        action: if is_operator {
            "pending_approval"
        } else {
            "direct"
        }
        .to_string(),
        transaction_id: tx.id,
        approval_id,
    })
}

// ---------------------------------------------------------------------------
// Reject transaction
// ---------------------------------------------------------------------------

/// Reject a transaction. Admin only.
pub async fn reject_transaction(
    pool: &SqlitePool,
    id: &str,
    requested_by: &RequestedBy,
) -> Result<(), TransactionServiceError> {
    if requested_by.role != "ADMIN" {
        return Err(TransactionServiceError::Forbidden);
    }

    let existing = transactions::find_by_id(pool, id)
        .await?
        .ok_or(TransactionServiceError::NotFound)?;

    if existing.approval_status == "REJECTED" {
        return Err(TransactionServiceError::AlreadyRejected);
    }

    // Cancel active receipts
    sqlx::query(
        "UPDATE receipts SET status = 'CANCELLED', updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
         WHERE transaction_id = ?1 AND status = 'ACTIVE'",
    )
    .bind(id)
    .execute(pool)
    .await?;

    // Reject pending approvals
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    sqlx::query(
        "UPDATE approvals SET status = 'REJECTED', reviewed_by_id = ?1,
         reviewed_at = ?2, notes = 'Rejected directly from cash management'
         WHERE entity_type = 'TRANSACTION' AND entity_id = ?3 AND status = 'PENDING'",
    )
    .bind(&requested_by.id)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    // Update transaction status
    sqlx::query("UPDATE transactions SET approval_status = 'REJECTED' WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    if let Some(rejected_tx) = transactions::find_by_id(pool, id).await? {
        let snapshot = audit::build_transaction_audit_snapshot(
            &transaction_to_snapshot_source(&rejected_tx),
        );
        let _ = audit_logs::create(
            pool,
            &audit_logs::CreateAuditLogData {
                transaction_id: rejected_tx.id.clone(),
                event_type: "TRANSACTION_REJECTED".to_string(),
                transaction_snapshot: serde_json::to_string(&snapshot).unwrap_or_default(),
                performed_by_id: requested_by.id.clone(),
            },
        )
        .await;
    }

    log_activity(
        pool,
        &requested_by.id,
        "transaction_rejected",
        &format!("Admin {} rejected transaction {}", requested_by.name, id),
        Some(serde_json::json!({
            "transactionId": id,
            "transactionCategory": existing.category,
            "approvalType": approval_labels::approval_type_from_entity(
                "TRANSACTION",
                Some(existing.category.as_str()),
            ),
            "direction": approval_labels::direction_from_transaction_type(Some(existing.r#type.as_str())),
        })),
    )
    .await;

    Ok(())
}

// ---------------------------------------------------------------------------
// Update / delete (immutable)
// ---------------------------------------------------------------------------

/// Update a transaction — always returns an error (transactions are immutable).
pub async fn update_transaction(
    _pool: &SqlitePool,
    _id: &str,
    _requested_by: &RequestedBy,
) -> Result<(), TransactionServiceError> {
    Err(TransactionServiceError::Immutable)
}

/// Delete a transaction — always returns an error (transactions are immutable).
pub async fn delete_transaction(
    _pool: &SqlitePool,
    _id: &str,
    _requested_by: &RequestedBy,
) -> Result<(), TransactionServiceError> {
    Err(TransactionServiceError::Immutable)
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

/// Get aggregated transaction summary.
pub async fn get_transaction_summary(
    pool: &SqlitePool,
) -> Result<TransactionSummary, TransactionServiceError> {
    transactions::summary(pool).await.map_err(Into::into)
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
