//! Repository functions for the `members` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::Member;

// ---------------------------------------------------------------------------
// Filter / input types
// ---------------------------------------------------------------------------

/// Filters for the member list query.
#[derive(Debug, Clone, Default)]
pub struct MemberListFilters {
    /// Filter by membership status on the linked user.
    /// `"PENDING_APPROVAL"` is special: matches members with no linked user.
    pub status: Option<String>,
    /// Free-text search across name, email, and member ID.
    pub search: Option<String>,
    pub page: u32,
    pub limit: u32,
}

/// Input data for creating a member.
#[derive(Debug, Clone)]
pub struct CreateMemberData {
    pub user_id: Option<String>,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub parent_member_id: Option<String>,
}

/// Input data for updating a member.
#[derive(Debug, Clone, Default)]
pub struct UpdateMemberData {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new member record.
pub async fn create(pool: &SqlitePool, data: &CreateMemberData) -> Result<Member, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO members (id, user_id, name, phone, email, address, parent_member_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&id)
    .bind(&data.user_id)
    .bind(&data.name)
    .bind(&data.phone)
    .bind(&data.email)
    .bind(&data.address)
    .bind(&data.parent_member_id)
    .execute(pool)
    .await?;

    // Fetch the created row (includes server-side defaults like created_at).
    find_by_id(pool, &id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

/// Find a member by primary key.
pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>("SELECT * FROM members WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Find a member by their `user_id` foreign key.
pub async fn find_by_member_id(
    pool: &SqlitePool,
    member_id: &str,
) -> Result<Option<Member>, sqlx::Error> {
    // `member_id` here refers to the user's member_id string (BSDS-YYYY-NNNN-SS).
    // We join through users to resolve this.
    sqlx::query_as::<_, Member>(
        "SELECT m.* FROM members m
         INNER JOIN users u ON u.id = m.user_id
         WHERE u.member_id = ?1",
    )
    .bind(member_id)
    .fetch_optional(pool)
    .await
}

/// List members with optional filters and pagination.
///
/// Only returns top-level members (parent_member_id IS NULL) unless searching
/// by status. The search filter matches on member name, email, or the linked
/// user's member_id.
pub async fn list(
    pool: &SqlitePool,
    filters: &MemberListFilters,
) -> Result<(Vec<Member>, i64), sqlx::Error> {
    let offset = (filters.page.saturating_sub(1)) * filters.limit;

    // Build dynamic SQL. We always filter to top-level members.
    let mut conditions = vec!["m.parent_member_id IS NULL".to_string()];
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref status) = filters.status {
        if status == "PENDING_APPROVAL" {
            conditions.push("m.user_id IS NULL".to_string());
        } else {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM users u WHERE u.id = m.user_id AND u.membership_status = ?{})",
                bind_values.len() + 1
            ));
            bind_values.push(status.clone());
        }
    }

    if let Some(ref search) = filters.search {
        let like_val = format!("%{search}%");
        let idx = bind_values.len() + 1;
        conditions.push(format!(
            "(m.name LIKE ?{idx} COLLATE NOCASE OR m.email LIKE ?{idx} COLLATE NOCASE OR EXISTS (SELECT 1 FROM users u WHERE u.id = m.user_id AND u.member_id LIKE ?{idx} COLLATE NOCASE))"
        ));
        bind_values.push(like_val);
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) as count FROM members m WHERE {where_clause}");
    let list_sql = format!(
        "SELECT m.* FROM members m WHERE {where_clause} ORDER BY m.created_at DESC LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2
    );

    // Count query
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for v in &bind_values {
        count_query = count_query.bind(v);
    }
    let total: i64 = count_query.fetch_one(pool).await.unwrap_or(0);

    // List query
    let mut list_query = sqlx::query_as::<_, Member>(&list_sql);
    for v in &bind_values {
        list_query = list_query.bind(v);
    }
    list_query = list_query.bind(filters.limit as i64).bind(offset as i64);
    let members = list_query.fetch_all(pool).await?;

    Ok((members, total))
}

/// Update an existing member.
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    data: &UpdateMemberData,
) -> Result<Member, sqlx::Error> {
    let existing = find_by_id(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;

    let name = data.name.as_deref().unwrap_or(&existing.name);
    let phone = data.phone.as_deref().unwrap_or(&existing.phone);
    let email = data.email.as_deref().unwrap_or(&existing.email);
    let address = data.address.as_deref().unwrap_or(&existing.address);

    sqlx::query(
        "UPDATE members SET name = ?1, phone = ?2, email = ?3, address = ?4,
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?5",
    )
    .bind(name)
    .bind(phone)
    .bind(email)
    .bind(address)
    .bind(id)
    .execute(pool)
    .await?;

    find_by_id(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Hard-delete a member record.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM members WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// List child members (sub-members) for a given parent member ID.
pub async fn list_sub_members(
    pool: &SqlitePool,
    parent_member_id: &str,
) -> Result<Vec<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>(
        "SELECT * FROM members WHERE parent_member_id = ?1 ORDER BY created_at ASC",
    )
    .bind(parent_member_id)
    .fetch_all(pool)
    .await
}

/// Count child members for a given parent member ID.
pub async fn count_sub_members(
    pool: &SqlitePool,
    parent_member_id: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM members WHERE parent_member_id = ?1",
    )
    .bind(parent_member_id)
    .fetch_one(pool)
    .await
}

/// Get the maximum sequence number for member IDs created in a given year.
///
/// Looks at the `users.member_id` column for IDs matching the pattern
/// `BSDS-{year}-NNNN-SS` and returns the max NNNN value.
pub async fn get_max_sequence_for_year(
    pool: &SqlitePool,
    year: u32,
) -> Result<i64, sqlx::Error> {
    let prefix = crate::support::member_id::member_id_prefix(year);
    let like_pattern = format!("{prefix}%");

    // Extract the NNNN segment (characters after the prefix, before the last dash).
    // member_id format: BSDS-2026-0042-00
    // prefix is "BSDS-2026-" (10 chars), NNNN is the next 4 chars.
    let prefix_len = prefix.len() as i64;
    let result = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MAX(CAST(SUBSTR(member_id, ?1 + 1, 4) AS INTEGER))
         FROM users WHERE member_id LIKE ?2",
    )
    .bind(prefix_len)
    .bind(&like_pattern)
    .fetch_one(pool)
    .await?;

    Ok(result.unwrap_or(0))
}
