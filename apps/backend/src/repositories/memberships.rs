//! Repository functions for the `memberships` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::Membership;

/// Explicit column list for SELECT on memberships — casts amount to REAL so
/// sqlx f64 decoding works even when SQLite NUMERIC affinity stores it as INTEGER.
const MEM_COLS: &str =
    "id, member_id, \"type\", fee_type, CAST(amount AS REAL) as amount, \
     start_date, end_date, is_application_fee, status, created_at";

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input data for creating a membership record.
#[derive(Debug, Clone)]
pub struct CreateMembershipData {
    pub member_id: String,
    pub r#type: String,
    pub fee_type: String,
    pub amount: f64,
    pub start_date: String,
    pub end_date: String,
    pub is_application_fee: bool,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new membership record.
pub async fn create(
    pool: &SqlitePool,
    data: &CreateMembershipData,
) -> Result<Membership, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO memberships (id, member_id, type, fee_type, amount, start_date, end_date, is_application_fee, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(&id)
    .bind(&data.member_id)
    .bind(&data.r#type)
    .bind(&data.fee_type)
    .bind(data.amount)
    .bind(&data.start_date)
    .bind(&data.end_date)
    .bind(data.is_application_fee)
    .bind(&data.status)
    .execute(pool)
    .await?;

    find_by_id(pool, &id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Find a membership by primary key.
pub async fn find_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<Membership>, sqlx::Error> {
    sqlx::query_as::<_, Membership>(&format!("SELECT {MEM_COLS} FROM memberships WHERE id = ?1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Find the active (most recent APPROVED, not-yet-expired) membership for a member.
pub async fn find_active_by_member_id(
    pool: &SqlitePool,
    member_id: &str,
) -> Result<Option<Membership>, sqlx::Error> {
    sqlx::query_as::<_, Membership>(&format!(
        "SELECT {MEM_COLS} FROM memberships
         WHERE member_id = ?1 AND status = 'APPROVED'
           AND end_date >= date('now')
         ORDER BY end_date DESC
         LIMIT 1"
    ))
    .bind(member_id)
    .fetch_optional(pool)
    .await
}

/// List all memberships for a given member, ordered by created_at descending.
pub async fn list_by_member_id(
    pool: &SqlitePool,
    member_id: &str,
) -> Result<Vec<Membership>, sqlx::Error> {
    sqlx::query_as::<_, Membership>(&format!(
        "SELECT {MEM_COLS} FROM memberships WHERE member_id = ?1 ORDER BY created_at DESC"
    ))
    .bind(member_id)
    .fetch_all(pool)
    .await
}

/// Update the status of a membership record.
pub async fn update_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE memberships SET status = ?1 WHERE id = ?2")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// List memberships expiring within the next N days.
/// Only returns APPROVED memberships whose end_date is between today and
/// today + days_ahead.
pub async fn list_expiring_soon(
    pool: &SqlitePool,
    days_ahead: i64,
) -> Result<Vec<Membership>, sqlx::Error> {
    sqlx::query_as::<_, Membership>(&format!(
        "SELECT {MEM_COLS} FROM memberships
         WHERE status = 'APPROVED'
           AND end_date >= date('now')
           AND end_date <= date('now', '+' || ?1 || ' days')
         ORDER BY end_date ASC"
    ))
    .bind(days_ahead)
    .fetch_all(pool)
    .await
}

/// List memberships that have expired (end_date < today, still APPROVED).
pub async fn list_expired(pool: &SqlitePool) -> Result<Vec<Membership>, sqlx::Error> {
    sqlx::query_as::<_, Membership>(&format!(
        "SELECT {MEM_COLS} FROM memberships
         WHERE status = 'APPROVED'
           AND end_date < date('now')
         ORDER BY end_date ASC"
    ))
    .fetch_all(pool)
    .await
}
