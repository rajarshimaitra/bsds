//! Approval Service — core approval queue business logic.
//!
//! Responsibilities:
//!   - List pending approvals (admin only, with filters + pagination)
//!   - Get a single approval with full details
//!   - approve_entry: apply proposed change to DB, log to activity and audit
//!   - reject_entry: discard change, update approval record, log to activity
//!
//! Entity-type dispatch on approval:
//!   MEMBER_ADD     -> create User + Member from new_data
//!   MEMBER_EDIT    -> apply new_data fields to existing Member (and User)
//!   MEMBER_DELETE  -> set User.membership_status = SUSPENDED (soft-delete)
//!   TRANSACTION    -> set Transaction.approval_status = APPROVED
//!   MEMBERSHIP     -> set Membership.status = APPROVED, update User fields

use sqlx::SqlitePool;

use crate::db::models::Approval;
use crate::repositories::{activity_logs, approvals, audit_logs, members, memberships, transactions};
use crate::services::membership_service::{self, TransactionMembershipDetails};
use crate::support::{approval_labels, audit};
use crate::support::member_id;
use crate::auth::temp_password::generate_temp_password_default;
use crate::auth::hash_password;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ApprovalServiceError {
    #[error("Approval not found")]
    NotFound,
    #[error("Approval is already {0}")]
    AlreadyProcessed(String),
    #[error("Unknown entity type: {0}")]
    UnknownEntityType(String),
    #[error("{0}")]
    Internal(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Password hash error: {0}")]
    PasswordHash(String),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Reviewer identity.
#[derive(Debug, Clone)]
pub struct ReviewedBy {
    pub id: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// List / get approvals
// ---------------------------------------------------------------------------

/// List approvals with filters and pagination.
pub async fn list_approvals(
    pool: &SqlitePool,
    filters: &approvals::ApprovalListFilters,
) -> Result<(Vec<Approval>, i64, i64), ApprovalServiceError> {
    approvals::list_all(pool, filters).await.map_err(Into::into)
}

/// Get a single approval by ID.
pub async fn get_approval(
    pool: &SqlitePool,
    id: &str,
) -> Result<Approval, ApprovalServiceError> {
    approvals::find_by_id(pool, id)
        .await?
        .ok_or(ApprovalServiceError::NotFound)
}

// ---------------------------------------------------------------------------
// Approve entry
// ---------------------------------------------------------------------------

/// Approve an approval entry.
///
/// Dispatches to entity-type handler, updates the Approval record,
/// logs audit events and activity.
pub async fn approve_entry(
    pool: &SqlitePool,
    id: &str,
    reviewed_by: &ReviewedBy,
    notes: Option<&str>,
) -> Result<Approval, ApprovalServiceError> {
    let approval = approvals::find_by_id(pool, id)
        .await?
        .ok_or(ApprovalServiceError::NotFound)?;

    if approval.status != "PENDING" {
        return Err(ApprovalServiceError::AlreadyProcessed(
            approval.status.to_lowercase(),
        ));
    }

    let mut log_entity_id = approval.entity_id.clone();

    match approval.entity_type.as_str() {
        "MEMBER_ADD" => {
            handle_member_add(pool, &approval).await?;
        }
        "MEMBER_EDIT" => {
            handle_member_edit(pool, &approval).await?;
        }
        "MEMBER_DELETE" => {
            handle_member_delete(pool, &approval).await?;
        }
        "TRANSACTION" => {
            log_entity_id = handle_transaction_approve(pool, &approval, &reviewed_by.id).await?;
        }
        "MEMBERSHIP" => {
            handle_membership_approve(pool, &approval).await?;
        }
        other => {
            return Err(ApprovalServiceError::UnknownEntityType(other.to_string()));
        }
    }

    // Update the Approval record
    approvals::update_status(pool, id, "APPROVED", &reviewed_by.id, notes).await?;

    // Update entity_id if it changed (e.g. for transactions)
    if log_entity_id != approval.entity_id {
        sqlx::query("UPDATE approvals SET entity_id = ?1 WHERE id = ?2")
            .bind(&log_entity_id)
            .bind(id)
            .execute(pool)
            .await?;
    }

    let approval_tx = if approval.entity_type == "TRANSACTION" {
        transactions::find_by_id(pool, &log_entity_id).await?
    } else {
        None
    };

    // Audit log for TRANSACTION approvals
    if approval.entity_type == "TRANSACTION" {
        if let Some(tx) = approval_tx.as_ref() {
            let snapshot = audit::build_transaction_audit_snapshot(
                &transaction_to_snapshot_source(tx),
            );
            let _ = audit_logs::create(
                pool,
                &audit_logs::CreateAuditLogData {
                    transaction_id: log_entity_id.clone(),
                    event_type: "TRANSACTION_APPROVED".to_string(),
                    transaction_snapshot: serde_json::to_string(&snapshot).unwrap_or_default(),
                    performed_by_id: reviewed_by.id.clone(),
                },
            )
            .await;
        }
    }

    // Activity log
    let action = get_approval_activity_action(&approval.entity_type, &approval.action, "approved");
    let desc = if approval.entity_type == "TRANSACTION" {
        format!(
            "Admin {} approved transaction {} ({})",
            reviewed_by.name, log_entity_id, approval.action
        )
    } else {
        format!(
            "Admin {} approved {} request ({})",
            reviewed_by.name, approval.entity_type, approval.action
        )
    };
    let approval_type = approval_labels::approval_type_from_entity(
        &approval.entity_type,
        approval_tx.as_ref().map(|tx| tx.category.as_str()),
    );
    let direction = approval_labels::direction_from_transaction_type(
        approval_tx.as_ref().map(|tx| tx.r#type.as_str()),
    );
    let prev = approval.previous_data.as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let next = approval.new_data.as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let is_txn = approval.entity_type == "TRANSACTION";
    log_activity(
        pool,
        &reviewed_by.id,
        &action,
        &desc,
        Some(serde_json::json!({
            "approvalId": id,
            "entityId": log_entity_id,
            "transactionId": if is_txn { Some(log_entity_id.as_str()) } else { None },
            "approvalType": approval_type,
            "direction": direction,
            "previousData": prev,
            "newData": next,
        })),
    )
    .await;

    approvals::find_by_id(pool, id)
        .await?
        .ok_or(ApprovalServiceError::NotFound)
}

// ---------------------------------------------------------------------------
// Reject entry
// ---------------------------------------------------------------------------

/// Reject an approval entry.
///
/// For TRANSACTION: marks the transaction as REJECTED, cancels receipts.
/// For MEMBERSHIP: marks the membership as REJECTED.
/// For MEMBER_ADD (add_member): deletes the placeholder Member record.
/// For MEMBER_EDIT / MEMBER_DELETE: no entity change (discard).
pub async fn reject_entry(
    pool: &SqlitePool,
    id: &str,
    reviewed_by: &ReviewedBy,
    notes: Option<&str>,
) -> Result<Approval, ApprovalServiceError> {
    let approval = approvals::find_by_id(pool, id)
        .await?
        .ok_or(ApprovalServiceError::NotFound)?;

    if approval.status != "PENDING" {
        return Err(ApprovalServiceError::AlreadyProcessed(
            approval.status.to_lowercase(),
        ));
    }

    match approval.entity_type.as_str() {
        "TRANSACTION" => {
            // Cancel active receipts
            sqlx::query(
                "UPDATE receipts SET status = 'CANCELLED', updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE transaction_id = ?1 AND status = 'ACTIVE'",
            )
            .bind(&approval.entity_id)
            .execute(pool)
            .await?;

            // Reject the transaction
            sqlx::query("UPDATE transactions SET approval_status = 'REJECTED' WHERE id = ?1")
                .bind(&approval.entity_id)
                .execute(pool)
                .await?;
        }
        "MEMBERSHIP" => {
            memberships::update_status(pool, &approval.entity_id, "REJECTED").await?;
        }
        "MEMBER_ADD" if approval.action == "add_member" => {
            // Delete the placeholder Member record (only if not yet linked to a user)
            sqlx::query("DELETE FROM members WHERE id = ?1 AND user_id IS NULL")
                .bind(&approval.entity_id)
                .execute(pool)
                .await?;
        }
        _ => {
            // MEMBER_EDIT / MEMBER_DELETE / add_sub_member: no entity change
        }
    }

    // Update the Approval record
    approvals::update_status(pool, id, "REJECTED", &reviewed_by.id, notes).await?;

    let rejected_tx = if approval.entity_type == "TRANSACTION" {
        transactions::find_by_id(pool, &approval.entity_id).await?
    } else {
        None
    };

    if let Some(tx) = rejected_tx.as_ref() {
        let snapshot = audit::build_transaction_audit_snapshot(
            &transaction_to_snapshot_source(tx),
        );
        let _ = audit_logs::create(
            pool,
            &audit_logs::CreateAuditLogData {
                transaction_id: approval.entity_id.clone(),
                event_type: "TRANSACTION_REJECTED".to_string(),
                transaction_snapshot: serde_json::to_string(&snapshot).unwrap_or_default(),
                performed_by_id: reviewed_by.id.clone(),
            },
        )
        .await;
    }

    // Activity log
    let action = get_approval_activity_action(&approval.entity_type, &approval.action, "rejected");
    let desc = if approval.entity_type == "TRANSACTION" {
        format!(
            "Admin {} rejected transaction {} ({})",
            reviewed_by.name, approval.entity_id, approval.action
        )
    } else {
        format!(
            "Admin {} rejected {} request ({})",
            reviewed_by.name, approval.entity_type, approval.action
        )
    };
    let approval_type = approval_labels::approval_type_from_entity(
        &approval.entity_type,
        rejected_tx.as_ref().map(|tx| tx.category.as_str()),
    );
    let direction = approval_labels::direction_from_transaction_type(
        rejected_tx.as_ref().map(|tx| tx.r#type.as_str()),
    );
    let prev = approval.previous_data.as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let next = approval.new_data.as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let is_txn_reject = approval.entity_type == "TRANSACTION";
    log_activity(
        pool,
        &reviewed_by.id,
        &action,
        &desc,
        Some(serde_json::json!({
            "approvalId": id,
            "entityId": approval.entity_id,
            "transactionId": if is_txn_reject { Some(approval.entity_id.as_str()) } else { None },
            "approvalType": approval_type,
            "direction": direction,
            "previousData": prev,
            "newData": next,
        })),
    )
    .await;

    approvals::find_by_id(pool, id)
        .await?
        .ok_or(ApprovalServiceError::NotFound)
}

// ---------------------------------------------------------------------------
// Entity-type handlers
// ---------------------------------------------------------------------------

async fn handle_member_add(
    pool: &SqlitePool,
    approval: &Approval,
) -> Result<(), ApprovalServiceError> {
    let data: serde_json::Value = approval
        .new_data
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .ok_or_else(|| ApprovalServiceError::Internal("MEMBER_ADD approval has no newData".into()))?;

    if approval.action == "add_sub_member" {
        handle_add_sub_member(pool, &data).await
    } else {
        handle_add_primary_member(pool, approval, &data).await
    }
}

async fn handle_add_primary_member(
    pool: &SqlitePool,
    approval: &Approval,
    data: &serde_json::Value,
) -> Result<(), ApprovalServiceError> {
    let name = data["name"].as_str().unwrap_or("");
    let email = data["email"].as_str().unwrap_or("").to_lowercase();
    let phone = data["phone"].as_str().unwrap_or("");
    let address = data["address"].as_str().unwrap_or("");

    let year = chrono::Utc::now().format("%Y").to_string().parse::<u32>().unwrap_or(2026);
    let max_seq = crate::repositories::members::get_max_sequence_for_year(pool, year).await.unwrap_or(0) as u32;
    let generated_member_id = member_id::generate_member_id(year, if max_seq == 0 { None } else { Some(max_seq) });

    let temp_password = generate_temp_password_default();
    let hashed_password = hash_password(&temp_password)
        .map_err(|e| ApprovalServiceError::PasswordHash(e.to_string()))?;

    let user_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (id, member_id, name, email, phone, address, password, is_temp_password, role, membership_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 'MEMBER', 'PENDING_PAYMENT')",
    )
    .bind(&user_id)
    .bind(&generated_member_id)
    .bind(name)
    .bind(&email)
    .bind(phone)
    .bind(address)
    .bind(&hashed_password)
    .execute(pool)
    .await?;

    // Update the placeholder Member: link User and sync profile fields
    sqlx::query(
        "UPDATE members SET user_id = ?1, name = ?2, email = ?3, phone = ?4, address = ?5,
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?6",
    )
    .bind(&user_id)
    .bind(name)
    .bind(&email)
    .bind(phone)
    .bind(address)
    .bind(&approval.entity_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn handle_add_sub_member(
    pool: &SqlitePool,
    data: &serde_json::Value,
) -> Result<(), ApprovalServiceError> {
    let parent_user_id = data["parentUserId"]
        .as_str()
        .ok_or_else(|| ApprovalServiceError::Internal("add_sub_member missing parentUserId".into()))?;
    let parent_member_id_str = data["parentMemberId"].as_str();
    let name = data["name"].as_str().unwrap_or("");
    let email = data["email"].as_str().unwrap_or("").to_lowercase();
    let phone = data["phone"].as_str().unwrap_or("");
    let relation = data["relation"].as_str().unwrap_or("");

    // Verify parent still exists
    let parent_user = crate::repositories::users::find_by_id(pool, parent_user_id)
        .await?
        .ok_or_else(|| ApprovalServiceError::Internal("Parent user not found".into()))?;

    // Find used sub-member indexes
    let existing_subs: Vec<crate::db::models::SubMember> = sqlx::query_as(
        "SELECT * FROM sub_members WHERE parent_user_id = ?1",
    )
    .bind(parent_user_id)
    .fetch_all(pool)
    .await?;

    let used_indexes: std::collections::HashSet<u32> = existing_subs
        .iter()
        .filter_map(|sm| {
            let parts: Vec<&str> = sm.member_id.split('-').collect();
            parts.last()?.parse::<u32>().ok()
        })
        .collect();

    let index = (1..=3u32)
        .find(|i| !used_indexes.contains(i))
        .ok_or_else(|| ApprovalServiceError::Internal("Maximum of 3 sub-members already reached".into()))?;

    let sub_member_id = member_id::generate_sub_member_id(&parent_user.member_id, index)
        .map_err(|e| ApprovalServiceError::Internal(e))?;

    let temp_password = generate_temp_password_default();
    let hashed_password = hash_password(&temp_password)
        .map_err(|e| ApprovalServiceError::PasswordHash(e.to_string()))?;

    let sm_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sub_members (id, member_id, parent_user_id, name, email, phone, password, is_temp_password, relation, can_login)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, 1)",
    )
    .bind(&sm_id)
    .bind(&sub_member_id)
    .bind(parent_user_id)
    .bind(name)
    .bind(&email)
    .bind(phone)
    .bind(&hashed_password)
    .bind(relation)
    .execute(pool)
    .await?;

    // Also create a child Member record if parentMemberId is given
    if let Some(pmid) = parent_member_id_str {
        let _ = members::create(
            pool,
            &members::CreateMemberData {
                user_id: None,
                name: name.to_string(),
                phone: phone.to_string(),
                email: email.clone(),
                address: data["address"].as_str().unwrap_or("").to_string(),
                parent_member_id: Some(pmid.to_string()),
            },
        )
        .await;
    }

    Ok(())
}

async fn handle_member_edit(
    pool: &SqlitePool,
    approval: &Approval,
) -> Result<(), ApprovalServiceError> {
    let data: serde_json::Value = approval
        .new_data
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .ok_or_else(|| ApprovalServiceError::Internal("MEMBER_EDIT approval has no newData".into()))?;

    let is_sub_member = approval.action == "edit_sub_member";

    if is_sub_member {
        // Update SubMember record
        let mut set_clauses = Vec::new();
        let mut bind_vals: Vec<String> = Vec::new();

        if let Some(name) = data["name"].as_str() {
            bind_vals.push(name.to_string());
            set_clauses.push(format!("name = ?{}", bind_vals.len()));
        }
        if let Some(email) = data["email"].as_str() {
            bind_vals.push(email.to_lowercase());
            set_clauses.push(format!("email = ?{}", bind_vals.len()));
        }
        if let Some(phone) = data["phone"].as_str() {
            bind_vals.push(phone.to_string());
            set_clauses.push(format!("phone = ?{}", bind_vals.len()));
        }
        if let Some(relation) = data["relation"].as_str() {
            bind_vals.push(relation.to_string());
            set_clauses.push(format!("relation = ?{}", bind_vals.len()));
        }
        if let Some(can_login) = data["canLogin"].as_bool() {
            bind_vals.push(if can_login { "1" } else { "0" }.to_string());
            set_clauses.push(format!("can_login = ?{}", bind_vals.len()));
        }

        if !set_clauses.is_empty() {
            bind_vals.push(approval.entity_id.clone());
            let sql = format!(
                "UPDATE sub_members SET {} WHERE id = ?{}",
                set_clauses.join(", "),
                bind_vals.len()
            );
            let mut query = sqlx::query(&sql);
            for v in &bind_vals {
                query = query.bind(v);
            }
            query.execute(pool).await?;
        }
    } else {
        // Update Member record
        let member = members::find_by_id(pool, &approval.entity_id)
            .await?
            .ok_or_else(|| ApprovalServiceError::Internal("Member not found".into()))?;

        let update_data = members::UpdateMemberData {
            name: data["name"].as_str().map(String::from),
            email: data["email"].as_str().map(|e| e.to_lowercase()),
            phone: data["phone"].as_str().map(String::from),
            address: data["address"].as_str().map(String::from),
        };
        members::update(pool, &approval.entity_id, &update_data).await?;

        // Mirror to User if linked
        if let Some(ref user_id) = member.user_id {
            let mut set_clauses = Vec::new();
            let mut bind_vals: Vec<String> = Vec::new();

            if let Some(name) = data["name"].as_str() {
                bind_vals.push(name.to_string());
                set_clauses.push(format!("name = ?{}", bind_vals.len()));
            }
            if let Some(email) = data["email"].as_str() {
                bind_vals.push(email.to_lowercase());
                set_clauses.push(format!("email = ?{}", bind_vals.len()));
            }
            if let Some(phone) = data["phone"].as_str() {
                bind_vals.push(phone.to_string());
                set_clauses.push(format!("phone = ?{}", bind_vals.len()));
            }
            if let Some(address) = data["address"].as_str() {
                bind_vals.push(address.to_string());
                set_clauses.push(format!("address = ?{}", bind_vals.len()));
            }

            if !set_clauses.is_empty() {
                set_clauses.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')".to_string());
                bind_vals.push(user_id.clone());
                let sql = format!(
                    "UPDATE users SET {} WHERE id = ?{}",
                    set_clauses.join(", "),
                    bind_vals.len()
                );
                let mut query = sqlx::query(&sql);
                for v in &bind_vals {
                    query = query.bind(v);
                }
                query.execute(pool).await?;
            }
        }
    }

    Ok(())
}

async fn handle_member_delete(
    pool: &SqlitePool,
    approval: &Approval,
) -> Result<(), ApprovalServiceError> {
    if approval.action == "remove_sub_member" {
        sqlx::query("DELETE FROM sub_members WHERE id = ?1")
            .bind(&approval.entity_id)
            .execute(pool)
            .await?;
    } else {
        let member = members::find_by_id(pool, &approval.entity_id)
            .await?
            .ok_or_else(|| ApprovalServiceError::Internal("Member not found".into()))?;

        if let Some(ref user_id) = member.user_id {
            sqlx::query(
                "UPDATE users SET membership_status = 'SUSPENDED',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1",
            )
            .bind(user_id)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn handle_transaction_approve(
    pool: &SqlitePool,
    approval: &Approval,
    approved_by_id: &str,
) -> Result<String, ApprovalServiceError> {
    let tx = transactions::find_by_id(pool, &approval.entity_id)
        .await?
        .ok_or_else(|| ApprovalServiceError::Internal("Transaction not found".into()))?;

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    sqlx::query(
        "UPDATE transactions SET approval_status = 'APPROVED', approved_by_id = ?1, approved_at = ?2 WHERE id = ?3",
    )
    .bind(approved_by_id)
    .bind(&now)
    .bind(&approval.entity_id)
    .execute(pool)
    .await?;

    // Read membership flags from the persisted Transaction booleans
    let raw: Option<serde_json::Value> = approval
        .new_data
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let membership_details = TransactionMembershipDetails {
        membership_type: if tx.includes_subscription {
            raw.as_ref()
                .and_then(|r| r["membershipType"].as_str())
                .map(String::from)
        } else {
            None
        },
        fee_type: raw
            .as_ref()
            .and_then(|r| r["feeType"].as_str())
            .map(String::from),
        is_application_fee: tx.includes_application_fee,
        includes_subscription: tx.includes_subscription,
        includes_annual_fee: tx.includes_annual_fee,
        includes_application_fee: tx.includes_application_fee,
    };

    // If this is a membership transaction with a linked member, create
    // Membership record(s) and update the member's lifecycle status.
    if let Some(ref member_id) = tx.member_id {
        if tx.category == "MEMBERSHIP" {
            let _ = membership_service::apply_membership_status_from_transaction(
                pool,
                member_id,
                &tx.category,
                tx.amount,
                &membership_details,
            )
            .await;
        }
    }

    Ok(approval.entity_id.clone())
}

async fn handle_membership_approve(
    pool: &SqlitePool,
    approval: &Approval,
) -> Result<(), ApprovalServiceError> {
    let membership = memberships::find_by_id(pool, &approval.entity_id)
        .await?
        .ok_or_else(|| ApprovalServiceError::Internal("Membership not found".into()))?;

    memberships::update_status(pool, &approval.entity_id, "APPROVED").await?;

    // Update the member's User fields
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
                let mut extra = String::new();
                if membership.is_application_fee {
                    extra = ", application_fee_paid = 1".to_string();
                }
                let sql = format!(
                    "UPDATE users SET membership_status = 'ACTIVE',
                     membership_type = ?1, membership_start = ?2, membership_expiry = ?3,
                     total_paid = total_paid + ?4{extra},
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

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_approval_activity_action(entity_type: &str, action: &str, outcome: &str) -> String {
    match (entity_type, action) {
        ("TRANSACTION", _)                    => format!("transaction_{outcome}"),
        ("MEMBERSHIP",  _)                    => format!("membership_{outcome}"),
        ("MEMBER_ADD",  "add_sub_member")     => format!("sub_member_add_{outcome}"),
        ("MEMBER_ADD",  _)                    => format!("member_add_{outcome}"),
        ("MEMBER_EDIT", "edit_sub_member")    => format!("sub_member_edit_{outcome}"),
        ("MEMBER_EDIT", _)                    => format!("member_edit_{outcome}"),
        ("MEMBER_DELETE","remove_sub_member") => format!("sub_member_remove_{outcome}"),
        ("MEMBER_DELETE", _)                  => format!("member_delete_{outcome}"),
        _                                     => format!("approval_{outcome}"),
    }
}

fn transaction_to_snapshot_source(
    tx: &crate::db::models::Transaction,
) -> audit::TransactionSnapshotSource {
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
