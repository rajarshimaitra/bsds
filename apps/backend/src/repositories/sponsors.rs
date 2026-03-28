//! Repository functions for the `sponsors` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::Sponsor;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input data for creating a sponsor.
#[derive(Debug, Clone)]
pub struct CreateSponsorData {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub company: Option<String>,
    pub created_by_id: String,
}

/// Input data for updating a sponsor.
#[derive(Debug, Clone, Default)]
pub struct UpdateSponsorData {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company: Option<Option<String>>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new sponsor record.
pub async fn create(
    pool: &SqlitePool,
    data: &CreateSponsorData,
) -> Result<Sponsor, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sponsors (id, name, phone, email, company, created_by_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&id)
    .bind(&data.name)
    .bind(&data.phone)
    .bind(&data.email)
    .bind(&data.company)
    .bind(&data.created_by_id)
    .execute(pool)
    .await?;

    find_by_id(pool, &id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Find a sponsor by primary key.
pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Sponsor>, sqlx::Error> {
    sqlx::query_as::<_, Sponsor>("SELECT * FROM sponsors WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// List all sponsors ordered by creation date descending.
pub async fn list(pool: &SqlitePool) -> Result<Vec<Sponsor>, sqlx::Error> {
    sqlx::query_as::<_, Sponsor>("SELECT * FROM sponsors ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

/// Update sponsor fields.
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    data: &UpdateSponsorData,
) -> Result<Sponsor, sqlx::Error> {
    let existing = find_by_id(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;

    let name = data.name.as_deref().unwrap_or(&existing.name);
    let phone = data.phone.as_deref().unwrap_or(&existing.phone);
    let email = data.email.as_deref().unwrap_or(&existing.email);
    let company = match &data.company {
        Some(c) => c.as_deref(),
        None => existing.company.as_deref(),
    };

    sqlx::query("UPDATE sponsors SET name = ?1, phone = ?2, email = ?3, company = ?4 WHERE id = ?5")
        .bind(name)
        .bind(phone)
        .bind(email)
        .bind(company)
        .bind(id)
        .execute(pool)
        .await?;

    find_by_id(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Delete a sponsor record.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sponsors WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
