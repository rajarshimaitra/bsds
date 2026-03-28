//! Member Service — business logic for member CRUD and sub-member management.
//!
//! Approval gating rules:
//!   ADMIN    -> direct DB write for all operations
//!   OPERATOR -> creates an Approval record instead of writing directly
//!              (change is only applied when an admin approves)

use sqlx::SqlitePool;

use crate::db::models::{Member, SubMember};
use crate::repositories::{activity_logs, approvals, members, users};
use crate::support::{approval_labels, member_id};
use crate::support::validation;
use crate::auth::temp_password::generate_temp_password_default;
use crate::auth::hash_password;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum MemberServiceError {
    #[error("Member not found")]
    NotFound,
    #[error("Parent member not found")]
    ParentNotFound,
    #[error("Parent member has no linked user account")]
    ParentNoUser,
    #[error("Maximum of 3 sub-members allowed per primary member")]
    MaxSubMembers,
    #[error("{0}")]
    Validation(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Password hashing error: {0}")]
    PasswordHash(String),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Caller identity for permission-gated operations.
#[derive(Debug, Clone)]
pub struct RequestedBy {
    pub id: String,
    pub role: String,
    pub name: String,
}

/// Result of a create/update/delete that may be gated by approval.
#[derive(Debug, Clone)]
pub struct MutationResult {
    /// `"direct"` if the change was applied immediately, `"pending_approval"` if queued.
    pub action: String,
    pub member_id: Option<String>,
    pub approval_id: Option<String>,
}

/// Struct-based input for add_sub_member (used by route handlers).
#[derive(Debug, Clone)]
pub struct AddSubMemberInput {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub relation: String,
}

/// Result of a successful add_sub_member (admin direct path).
#[derive(Debug, Clone)]
pub struct AddSubMemberResult {
    /// UUID of the new sub_members row.
    pub sub_member_id: String,
    /// BSDS-YYYY-NNNN-SS formatted member ID.
    pub member_id: String,
}

/// Struct-based input for update_sub_member (used by route handlers).
#[derive(Debug, Clone, Default)]
pub struct UpdateSubMemberInput {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub relation: Option<String>,
    pub can_login: Option<bool>,
}

// ---------------------------------------------------------------------------
// List members
// ---------------------------------------------------------------------------

/// List members with optional search and status filter, paginated.
pub async fn list_members(
    pool: &SqlitePool,
    filters: &members::MemberListFilters,
) -> Result<(Vec<Member>, i64), MemberServiceError> {
    let (members_list, total) = members::list(pool, filters).await?;
    Ok((members_list, total))
}

// ---------------------------------------------------------------------------
// Get single member
// ---------------------------------------------------------------------------

/// Retrieve a single member by primary key.
pub async fn get_member(
    pool: &SqlitePool,
    id: &str,
) -> Result<Member, MemberServiceError> {
    members::find_by_id(pool, id)
        .await?
        .ok_or(MemberServiceError::NotFound)
}

// ---------------------------------------------------------------------------
// Create member
// ---------------------------------------------------------------------------

/// Create a new member.
///
/// Admin: creates User + Member directly, status = PENDING_PAYMENT.
/// Operator: creates placeholder Member + Approval record.
pub async fn create_member(
    pool: &SqlitePool,
    data: &members::CreateMemberData,
    requested_by: &RequestedBy,
) -> Result<MutationResult, MemberServiceError> {
    // Validate inputs
    validation::validate_name(&data.name)
        .map_err(MemberServiceError::Validation)?;
    validation::validate_phone(&data.phone)
        .map_err(MemberServiceError::Validation)?;
    validation::validate_email(&data.email)
        .map_err(MemberServiceError::Validation)?;
    validation::validate_required_string(&data.address, "address", 500)
        .map_err(MemberServiceError::Validation)?;

    let email = data.email.to_lowercase().trim().to_string();

    if requested_by.role == "OPERATOR" {
        // Operator path: create placeholder Member (no User yet)
        let placeholder = members::create(
            pool,
            &members::CreateMemberData {
                user_id: None,
                name: data.name.clone(),
                phone: data.phone.clone(),
                email: email.clone(),
                address: data.address.clone(),
                parent_member_id: None,
            },
        )
        .await?;

        let new_data = serde_json::json!({
            "name": data.name,
            "email": email,
            "phone": data.phone,
            "address": data.address,
        });

        let approval = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "MEMBER_ADD".to_string(),
                entity_id: placeholder.id.clone(),
                action: "add_member".to_string(),
                previous_data: None,
                new_data: Some(serde_json::to_string(&new_data).unwrap_or_default()),
                requested_by_id: requested_by.id.clone(),
                status: "PENDING".to_string(),
            },
        )
        .await?;

        log_activity(
            pool,
            &requested_by.id,
            "member_add_requested",
            &format!(
                "Operator {} submitted new member request for {}",
                requested_by.name, data.name
            ),
            Some(serde_json::json!({
                "approvalId": approval.id,
                "memberEmail": email,
                "memberId": placeholder.id,
                "approvalType": approval_labels::MEMBERSHIP_APPROVAL,
            })),
        )
        .await;

        return Ok(MutationResult {
            action: "pending_approval".to_string(),
            member_id: None,
            approval_id: Some(approval.id),
        });
    }

    // Admin path: direct create
    let temp_password = generate_temp_password_default();
    let hashed_password = hash_password(&temp_password)
        .map_err(|e| MemberServiceError::PasswordHash(e.to_string()))?;

    let year = chrono::Utc::now().format("%Y").to_string().parse::<u32>().unwrap_or(2026);
    let max_seq = members::get_max_sequence_for_year(pool, year).await? as u32;
    let generated_member_id = member_id::generate_member_id(year, if max_seq == 0 { None } else { Some(max_seq) });

    // Create User record
    let user_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (id, member_id, name, email, phone, address, password, is_temp_password, role, membership_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 'MEMBER', 'PENDING_PAYMENT')",
    )
    .bind(&user_id)
    .bind(&generated_member_id)
    .bind(&data.name)
    .bind(&email)
    .bind(&data.phone)
    .bind(&data.address)
    .bind(&hashed_password)
    .execute(pool)
    .await?;

    // Create Member record linked to User
    let member = members::create(
        pool,
        &members::CreateMemberData {
            user_id: Some(user_id.clone()),
            name: data.name.clone(),
            phone: data.phone.clone(),
            email: email.clone(),
            address: data.address.clone(),
            parent_member_id: None,
        },
    )
    .await?;

    log_activity(
        pool,
        &requested_by.id,
        "member_created",
        &format!(
            "Admin {} created member {} ({})",
            requested_by.name, data.name, generated_member_id
        ),
        Some(serde_json::json!({
            "memberId": generated_member_id,
            "memberEmail": email,
            "memberRecordId": member.id,
            "approvalType": approval_labels::MEMBERSHIP_APPROVAL,
        })),
    )
    .await;

    Ok(MutationResult {
        action: "direct".to_string(),
        member_id: Some(generated_member_id),
        approval_id: None,
    })
}

// ---------------------------------------------------------------------------
// Update member
// ---------------------------------------------------------------------------

/// Update member fields.
///
/// Admin: applies update directly to Member (and User if linked).
/// Operator: creates an Approval record (MEMBER_EDIT).
pub async fn update_member(
    pool: &SqlitePool,
    id: &str,
    data: &members::UpdateMemberData,
    requested_by: &RequestedBy,
) -> Result<MutationResult, MemberServiceError> {
    let existing = members::find_by_id(pool, id)
        .await?
        .ok_or(MemberServiceError::NotFound)?;

    if requested_by.role == "OPERATOR" {
        let previous_data = serde_json::json!({
            "name": existing.name,
            "email": existing.email,
            "phone": existing.phone,
            "address": existing.address,
        });
        let new_data = serde_json::json!({
            "name": data.name,
            "email": data.email,
            "phone": data.phone,
            "address": data.address,
        });

        let approval = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "MEMBER_EDIT".to_string(),
                entity_id: id.to_string(),
                action: "edit_member".to_string(),
                previous_data: Some(serde_json::to_string(&previous_data).unwrap_or_default()),
                new_data: Some(serde_json::to_string(&new_data).unwrap_or_default()),
                requested_by_id: requested_by.id.clone(),
                status: "PENDING".to_string(),
            },
        )
        .await?;

        log_activity(
            pool,
            &requested_by.id,
            "member_edit_requested",
            &format!(
                "Operator {} submitted edit request for member {}",
                requested_by.name, existing.name
            ),
            Some(serde_json::json!({
                "approvalId": approval.id,
                "memberId": id,
            })),
        )
        .await;

        return Ok(MutationResult {
            action: "pending_approval".to_string(),
            member_id: None,
            approval_id: Some(approval.id),
        });
    }

    // Admin path: direct update
    let email_lower = data.email.as_ref().map(|e| e.to_lowercase().trim().to_string());
    let update_data = members::UpdateMemberData {
        name: data.name.clone(),
        phone: data.phone.clone(),
        email: email_lower.clone(),
        address: data.address.clone(),
    };
    members::update(pool, id, &update_data).await?;

    // Mirror changes to the linked User record
    if let Some(ref user_id) = existing.user_id {
        let mut set_clauses = Vec::new();
        let mut bind_vals: Vec<String> = Vec::new();

        if let Some(ref name) = data.name {
            bind_vals.push(name.clone());
            set_clauses.push(format!("name = ?{}", bind_vals.len()));
        }
        if let Some(ref email) = email_lower {
            bind_vals.push(email.clone());
            set_clauses.push(format!("email = ?{}", bind_vals.len()));
        }
        if let Some(ref phone) = data.phone {
            bind_vals.push(phone.clone());
            set_clauses.push(format!("phone = ?{}", bind_vals.len()));
        }
        if let Some(ref address) = data.address {
            bind_vals.push(address.clone());
            set_clauses.push(format!("address = ?{}", bind_vals.len()));
        }

        if !set_clauses.is_empty() {
            set_clauses.push(format!(
                "updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')"
            ));
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

    log_activity(
        pool,
        &requested_by.id,
        "member_updated",
        &format!(
            "Admin {} updated member {}",
            requested_by.name, existing.name
        ),
        Some(serde_json::json!({
            "memberId": id,
            "previousData": {
                "name": existing.name,
                "email": existing.email,
                "phone": existing.phone,
                "address": existing.address,
            },
            "newData": {
                "name": data.name.as_deref().unwrap_or(&existing.name),
                "email": email_lower.as_deref().unwrap_or(&existing.email),
                "phone": data.phone.as_deref().unwrap_or(&existing.phone),
                "address": data.address.as_deref().unwrap_or(&existing.address),
            },
        })),
    )
    .await;

    Ok(MutationResult {
        action: "direct".to_string(),
        member_id: None,
        approval_id: None,
    })
}

// ---------------------------------------------------------------------------
// Delete member (soft-delete)
// ---------------------------------------------------------------------------

/// Soft-delete a member by setting their User status to SUSPENDED.
///
/// Admin: applies directly.
/// Operator: creates an Approval record (MEMBER_DELETE).
pub async fn delete_member(
    pool: &SqlitePool,
    id: &str,
    requested_by: &RequestedBy,
) -> Result<MutationResult, MemberServiceError> {
    let existing = members::find_by_id(pool, id)
        .await?
        .ok_or(MemberServiceError::NotFound)?;

    if requested_by.role == "OPERATOR" {
        let previous_data = serde_json::json!({
            "id": existing.id,
            "name": existing.name,
            "email": existing.email,
        });
        let new_data = serde_json::json!({ "membershipStatus": "SUSPENDED" });

        let approval = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "MEMBER_DELETE".to_string(),
                entity_id: id.to_string(),
                action: "delete_member".to_string(),
                previous_data: Some(serde_json::to_string(&previous_data).unwrap_or_default()),
                new_data: Some(serde_json::to_string(&new_data).unwrap_or_default()),
                requested_by_id: requested_by.id.clone(),
                status: "PENDING".to_string(),
            },
        )
        .await?;

        log_activity(
            pool,
            &requested_by.id,
            "member_delete_requested",
            &format!(
                "Operator {} submitted delete request for member {}",
                requested_by.name, existing.name
            ),
            Some(serde_json::json!({
                "approvalId": approval.id,
                "memberId": id,
            })),
        )
        .await;

        return Ok(MutationResult {
            action: "pending_approval".to_string(),
            member_id: None,
            approval_id: Some(approval.id),
        });
    }

    // Admin path: soft-delete (set User status to SUSPENDED)
    if let Some(ref user_id) = existing.user_id {
        sqlx::query("UPDATE users SET membership_status = 'SUSPENDED', updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1")
            .bind(user_id)
            .execute(pool)
            .await?;
    }

    log_activity(
        pool,
        &requested_by.id,
        "member_deleted",
        &format!(
            "Admin {} suspended (soft-deleted) member {}",
            requested_by.name, existing.name
        ),
        Some(serde_json::json!({ "memberId": id })),
    )
    .await;

    Ok(MutationResult {
        action: "direct".to_string(),
        member_id: None,
        approval_id: None,
    })
}

// ---------------------------------------------------------------------------
// Sub-member operations
// ---------------------------------------------------------------------------

/// List all sub-members for a given parent member.
pub async fn list_sub_members(
    pool: &SqlitePool,
    parent_member_id: &str,
) -> Result<Vec<SubMember>, MemberServiceError> {
    let parent = members::find_by_id(pool, parent_member_id)
        .await?
        .ok_or(MemberServiceError::ParentNotFound)?;

    let user_id = match parent.user_id {
        Some(ref uid) => uid.clone(),
        None => return Ok(Vec::new()),
    };

    let subs = sqlx::query_as::<_, SubMember>(
        "SELECT * FROM sub_members WHERE parent_user_id = ?1 ORDER BY member_id ASC",
    )
    .bind(&user_id)
    .fetch_all(pool)
    .await?;

    Ok(subs)
}

/// Add a sub-member to a parent member.
/// Enforces max 3 sub-members cap.
pub async fn add_sub_member(
    pool: &SqlitePool,
    parent_member_id: &str,
    name: &str,
    email: &str,
    phone: &str,
    relation: &str,
    requested_by: &RequestedBy,
) -> Result<MutationResult, MemberServiceError> {
    let parent = members::find_by_id(pool, parent_member_id)
        .await?
        .ok_or(MemberServiceError::ParentNotFound)?;

    let user_id = parent
        .user_id
        .as_deref()
        .ok_or(MemberServiceError::ParentNoUser)?;

    let current_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sub_members WHERE parent_user_id = ?1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;

    if current_count >= 3 {
        return Err(MemberServiceError::MaxSubMembers);
    }

    let email_lower = email.to_lowercase().trim().to_string();

    if requested_by.role == "OPERATOR" {
        let new_data = serde_json::json!({
            "name": name,
            "email": email_lower,
            "phone": phone,
            "relation": relation,
            "parentMemberId": parent_member_id,
            "parentUserId": user_id,
        });

        let approval = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "MEMBER_ADD".to_string(),
                entity_id: parent_member_id.to_string(),
                action: "add_sub_member".to_string(),
                previous_data: None,
                new_data: Some(serde_json::to_string(&new_data).unwrap_or_default()),
                requested_by_id: requested_by.id.clone(),
                status: "PENDING".to_string(),
            },
        )
        .await?;

        log_activity(
            pool,
            &requested_by.id,
            "sub_member_add_requested",
            &format!(
                "Operator {} submitted sub-member add request for {} under member {}",
                requested_by.name, name, parent.name
            ),
            Some(serde_json::json!({
                "approvalId": approval.id,
                "parentMemberId": parent_member_id,
                "name": name,
                "email": email_lower,
                "phone": phone,
                "relation": relation,
            })),
        )
        .await;

        return Ok(MutationResult {
            action: "pending_approval".to_string(),
            member_id: None,
            approval_id: Some(approval.id),
        });
    }

    // Admin path: direct create
    // Get parent user's member_id for sub-member ID generation
    let parent_user = users::find_by_id(pool, user_id)
        .await?
        .ok_or(MemberServiceError::ParentNoUser)?;

    // Find next available sub-member index (1-3)
    let existing_subs: Vec<SubMember> = sqlx::query_as(
        "SELECT * FROM sub_members WHERE parent_user_id = ?1",
    )
    .bind(user_id)
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
        .ok_or(MemberServiceError::MaxSubMembers)?;

    let sub_member_id = member_id::generate_sub_member_id(&parent_user.member_id, index)
        .map_err(|e| MemberServiceError::Validation(e))?;

    let temp_password = generate_temp_password_default();
    let hashed_password = hash_password(&temp_password)
        .map_err(|e| MemberServiceError::PasswordHash(e.to_string()))?;

    let sm_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sub_members (id, member_id, parent_user_id, name, email, phone, password, is_temp_password, relation, can_login)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, 1)",
    )
    .bind(&sm_id)
    .bind(&sub_member_id)
    .bind(user_id)
    .bind(name)
    .bind(&email_lower)
    .bind(phone)
    .bind(&hashed_password)
    .bind(relation)
    .execute(pool)
    .await?;

    log_activity(
        pool,
        &requested_by.id,
        "sub_member_created",
        &format!(
            "Admin {} added sub-member {} ({}) to member {}",
            requested_by.name, name, sub_member_id, parent.name
        ),
        Some(serde_json::json!({
            "subMemberId": sm_id,
            "subMemberMemberId": sub_member_id,
            "parentMemberId": parent_member_id,
            "name": name,
            "email": email_lower,
            "phone": phone,
            "relation": relation,
        })),
    )
    .await;

    Ok(MutationResult {
        action: "direct".to_string(),
        member_id: Some(sub_member_id),
        approval_id: None,
    })
}

/// Update a sub-member belonging to a parent member.
///
/// Admin: applies update directly.
/// Operator: creates an approval request for later review.
pub async fn update_sub_member(
    pool: &SqlitePool,
    parent_member_id: &str,
    sub_member_id: &str,
    name: Option<&str>,
    email: Option<&str>,
    phone: Option<&str>,
    relation: Option<&str>,
    can_login: Option<bool>,
    requested_by: &RequestedBy,
) -> Result<MutationResult, MemberServiceError> {
    let parent = members::find_by_id(pool, parent_member_id)
        .await?
        .ok_or(MemberServiceError::ParentNotFound)?;

    let user_id = parent
        .user_id
        .as_deref()
        .ok_or(MemberServiceError::ParentNoUser)?;

    let existing = sqlx::query_as::<_, SubMember>(
        "SELECT * FROM sub_members WHERE id = ?1 AND parent_user_id = ?2",
    )
    .bind(sub_member_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(MemberServiceError::NotFound)?;

    let email_lower = email.map(|value| value.to_lowercase().trim().to_string());

    if requested_by.role == "OPERATOR" {
        let previous_data = serde_json::json!({
            "name": existing.name,
            "email": existing.email,
            "phone": existing.phone,
            "relation": existing.relation,
            "canLogin": existing.can_login,
        });

        let new_data = serde_json::json!({
            "name": name,
            "email": email_lower,
            "phone": phone,
            "relation": relation,
            "canLogin": can_login,
            "parentMemberId": parent_member_id,
        });

        let approval = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "MEMBER_EDIT".to_string(),
                entity_id: sub_member_id.to_string(),
                action: "edit_sub_member".to_string(),
                previous_data: Some(serde_json::to_string(&previous_data).unwrap_or_default()),
                new_data: Some(serde_json::to_string(&new_data).unwrap_or_default()),
                requested_by_id: requested_by.id.clone(),
                status: "PENDING".to_string(),
            },
        )
        .await?;

        log_activity(
            pool,
            &requested_by.id,
            "sub_member_edit_requested",
            &format!(
                "Operator {} submitted edit request for sub-member {}",
                requested_by.name, existing.name
            ),
            Some(serde_json::json!({
                "approvalId": approval.id,
                "subMemberId": sub_member_id,
                "parentMemberId": parent_member_id,
                "name": existing.name,
                "email": existing.email,
                "phone": existing.phone,
                "relation": existing.relation,
            })),
        )
        .await;

        return Ok(MutationResult {
            action: "pending_approval".to_string(),
            member_id: None,
            approval_id: Some(approval.id),
        });
    }

    sqlx::query(
        "UPDATE sub_members
         SET name = COALESCE(?1, name),
             email = COALESCE(?2, email),
             phone = COALESCE(?3, phone),
             relation = COALESCE(?4, relation),
             can_login = COALESCE(?5, can_login)
         WHERE id = ?6 AND parent_user_id = ?7",
    )
    .bind(name)
    .bind(email_lower.as_deref())
    .bind(phone)
    .bind(relation)
    .bind(can_login)
    .bind(sub_member_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    log_activity(
        pool,
        &requested_by.id,
        "sub_member_updated",
        &format!(
            "Admin {} updated sub-member {}",
            requested_by.name, existing.name
        ),
        Some(serde_json::json!({
            "subMemberId": sub_member_id,
            "parentMemberId": parent_member_id,
            "previousData": {
                "name": existing.name,
                "email": existing.email,
                "phone": existing.phone,
                "relation": existing.relation,
                "canLogin": existing.can_login,
            },
            "newData": {
                "name": name.unwrap_or(&existing.name),
                "email": email_lower.as_deref().unwrap_or(&existing.email),
                "phone": phone.unwrap_or(&existing.phone),
                "relation": relation.unwrap_or(&existing.relation),
                "canLogin": can_login.unwrap_or(existing.can_login),
            },
        })),
    )
    .await;

    Ok(MutationResult {
        action: "direct".to_string(),
        member_id: None,
        approval_id: None,
    })
}

/// Remove a sub-member from a parent member.
///
/// Admin: deletes directly.
/// Operator: creates an approval request for later review.
pub async fn remove_sub_member(
    pool: &SqlitePool,
    parent_member_id: &str,
    sub_member_id: &str,
    requested_by: &RequestedBy,
) -> Result<MutationResult, MemberServiceError> {
    let parent = members::find_by_id(pool, parent_member_id)
        .await?
        .ok_or(MemberServiceError::ParentNotFound)?;

    let user_id = parent
        .user_id
        .as_deref()
        .ok_or(MemberServiceError::ParentNoUser)?;

    let existing = sqlx::query_as::<_, SubMember>(
        "SELECT * FROM sub_members WHERE id = ?1 AND parent_user_id = ?2",
    )
    .bind(sub_member_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(MemberServiceError::NotFound)?;

    if requested_by.role == "OPERATOR" {
        let previous_data = serde_json::json!({
            "id": existing.id,
            "memberId": existing.member_id,
            "name": existing.name,
            "email": existing.email,
        });

        let new_data = serde_json::json!({
            "deleted": true,
            "parentMemberId": parent_member_id,
        });

        let approval = approvals::create(
            pool,
            &approvals::CreateApprovalData {
                entity_type: "MEMBER_DELETE".to_string(),
                entity_id: sub_member_id.to_string(),
                action: "remove_sub_member".to_string(),
                previous_data: Some(serde_json::to_string(&previous_data).unwrap_or_default()),
                new_data: Some(serde_json::to_string(&new_data).unwrap_or_default()),
                requested_by_id: requested_by.id.clone(),
                status: "PENDING".to_string(),
            },
        )
        .await?;

        log_activity(
            pool,
            &requested_by.id,
            "sub_member_remove_requested",
            &format!(
                "Operator {} submitted remove request for sub-member {}",
                requested_by.name, existing.name
            ),
            Some(serde_json::json!({
                "approvalId": approval.id,
                "subMemberId": sub_member_id,
                "parentMemberId": parent_member_id,
                "name": existing.name,
                "email": existing.email,
                "phone": existing.phone,
                "relation": existing.relation,
            })),
        )
        .await;

        return Ok(MutationResult {
            action: "pending_approval".to_string(),
            member_id: None,
            approval_id: Some(approval.id),
        });
    }

    sqlx::query("DELETE FROM sub_members WHERE id = ?1 AND parent_user_id = ?2")
        .bind(sub_member_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    log_activity(
        pool,
        &requested_by.id,
        "sub_member_removed",
        &format!(
            "Admin {} removed sub-member {}",
            requested_by.name, existing.name
        ),
        Some(serde_json::json!({
            "subMemberId": sub_member_id,
            "parentMemberId": parent_member_id,
            "name": existing.name,
            "email": existing.email,
            "phone": existing.phone,
            "relation": existing.relation,
        })),
    )
    .await;

    Ok(MutationResult {
        action: "direct".to_string(),
        member_id: None,
        approval_id: None,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Log an activity entry. Errors are swallowed (best-effort logging).
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
