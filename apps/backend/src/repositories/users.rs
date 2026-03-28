//! Repository functions for the `users` table.

use sqlx::SqlitePool;

use crate::db::models::User;

/// Explicit column list for SELECT on users — casts total_paid to REAL.
const USER_COLS: &str =
    "id, member_id, name, email, phone, address, password, is_temp_password, role, \
     membership_status, membership_type, membership_start, membership_expiry, \
     CAST(total_paid AS REAL) as total_paid, application_fee_paid, \
     annual_fee_start, annual_fee_expiry, annual_fee_paid, created_at, updated_at";

/// Find a user by email address.
pub async fn find_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(&format!("SELECT {USER_COLS} FROM users WHERE email = ?1"))
        .bind(email)
        .fetch_optional(pool)
        .await
}

/// Find a user by primary key.
pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(&format!("SELECT {USER_COLS} FROM users WHERE id = ?1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Update the password hash for a user.
pub async fn update_password(
    pool: &SqlitePool,
    id: &str,
    hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET password = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?2",
    )
    .bind(hash)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update the `is_temp_password` flag for a user.
pub async fn update_must_change_password(
    pool: &SqlitePool,
    id: &str,
    value: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET is_temp_password = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?2",
    )
    .bind(value)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
