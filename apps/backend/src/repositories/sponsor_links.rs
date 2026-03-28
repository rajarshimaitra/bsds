//! Repository functions for the `sponsor_links` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::SponsorLink;

/// Explicit column list for SELECT on sponsor_links — casts amount to REAL.
const SL_COLS: &str =
    "id, sponsor_id, token, CAST(amount AS REAL) as amount, upi_id, \
     bank_details, is_active, created_by_id, created_at, expires_at";

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input data for creating a sponsor link.
#[derive(Debug, Clone)]
pub struct CreateSponsorLinkData {
    pub sponsor_id: Option<String>,
    pub token: String,
    pub amount: Option<f64>,
    pub upi_id: String,
    pub bank_details: Option<String>,
    pub is_active: bool,
    pub created_by_id: String,
    pub expires_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new sponsor link record.
pub async fn create(
    pool: &SqlitePool,
    data: &CreateSponsorLinkData,
) -> Result<SponsorLink, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sponsor_links (id, sponsor_id, token, amount, upi_id, bank_details, is_active, created_by_id, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(&id)
    .bind(&data.sponsor_id)
    .bind(&data.token)
    .bind(data.amount)
    .bind(&data.upi_id)
    .bind(&data.bank_details)
    .bind(data.is_active)
    .bind(&data.created_by_id)
    .bind(&data.expires_at)
    .execute(pool)
    .await?;

    find_by_id(pool, &id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Find a sponsor link by its unique token.
pub async fn find_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<SponsorLink>, sqlx::Error> {
    sqlx::query_as::<_, SponsorLink>(&format!("SELECT {SL_COLS} FROM sponsor_links WHERE token = ?1"))
        .bind(token)
        .fetch_optional(pool)
        .await
}

/// Find a sponsor link by primary key.
pub async fn find_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<SponsorLink>, sqlx::Error> {
    sqlx::query_as::<_, SponsorLink>(&format!("SELECT {SL_COLS} FROM sponsor_links WHERE id = ?1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// List all sponsor links for a given sponsor.
pub async fn list_by_sponsor_id(
    pool: &SqlitePool,
    sponsor_id: &str,
) -> Result<Vec<SponsorLink>, sqlx::Error> {
    sqlx::query_as::<_, SponsorLink>(&format!(
        "SELECT {SL_COLS} FROM sponsor_links WHERE sponsor_id = ?1 ORDER BY created_at DESC"
    ))
    .bind(sponsor_id)
    .fetch_all(pool)
    .await
}

/// Update the payment status / activity of a sponsor link.
///
/// This sets `is_active` to false after a payment is received and stores
/// the razorpay payment ID in the `bank_details` JSON for audit purposes.
pub async fn update_payment_status(
    pool: &SqlitePool,
    id: &str,
    is_active: bool,
    razorpay_payment_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    // If a razorpay_payment_id is provided, append it to the bank_details JSON.
    // Otherwise just update is_active.
    if let Some(pay_id) = razorpay_payment_id {
        // Read existing bank_details to append the payment ID
        let existing = find_by_id(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;
        let mut details: serde_json::Value = existing
            .bank_details
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::json!({}));

        if let Some(obj) = details.as_object_mut() {
            obj.insert(
                "razorpayPaymentId".to_string(),
                serde_json::Value::String(pay_id.to_string()),
            );
        }

        let details_str = serde_json::to_string(&details).unwrap_or_default();
        sqlx::query("UPDATE sponsor_links SET is_active = ?1, bank_details = ?2 WHERE id = ?3")
            .bind(is_active)
            .bind(&details_str)
            .bind(id)
            .execute(pool)
            .await?;
    } else {
        sqlx::query("UPDATE sponsor_links SET is_active = ?1 WHERE id = ?2")
            .bind(is_active)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}
