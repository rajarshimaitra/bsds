//! Repository functions for the `receipts` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::Receipt;

/// Explicit column list for SELECT on receipts — casts amount to REAL.
const RECEIPT_COLS: &str =
    "id, transaction_id, receipt_number, issued_by_id, issued_at, status, \
     \"type\", member_name, member_code, membership_start, membership_end, \
     sponsor_name, sponsor_company, sponsor_purpose, CAST(amount AS REAL) as amount, \
     payment_mode, category, purpose, breakdown, remark, received_by, \
     club_name, club_address, created_at, updated_at";

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input data for creating a receipt record.
#[derive(Debug, Clone)]
pub struct CreateReceiptData {
    pub transaction_id: String,
    pub receipt_number: String,
    pub issued_by_id: String,
    pub status: String,
    pub r#type: String,
    pub member_name: Option<String>,
    pub member_code: Option<String>,
    pub membership_start: Option<String>,
    pub membership_end: Option<String>,
    pub sponsor_name: Option<String>,
    pub sponsor_company: Option<String>,
    pub sponsor_purpose: Option<String>,
    pub amount: f64,
    pub payment_mode: String,
    pub category: String,
    pub purpose: String,
    pub breakdown: Option<String>,
    pub remark: Option<String>,
    pub received_by: String,
    pub club_name: String,
    pub club_address: String,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new receipt record.
pub async fn create(
    pool: &SqlitePool,
    data: &CreateReceiptData,
) -> Result<Receipt, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO receipts (
            id, transaction_id, receipt_number, issued_by_id, status, type,
            member_name, member_code, membership_start, membership_end,
            sponsor_name, sponsor_company, sponsor_purpose,
            amount, payment_mode, category, purpose, breakdown, remark,
            received_by, club_name, club_address
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
         )",
    )
    .bind(&id)
    .bind(&data.transaction_id)
    .bind(&data.receipt_number)
    .bind(&data.issued_by_id)
    .bind(&data.status)
    .bind(&data.r#type)
    .bind(&data.member_name)
    .bind(&data.member_code)
    .bind(&data.membership_start)
    .bind(&data.membership_end)
    .bind(&data.sponsor_name)
    .bind(&data.sponsor_company)
    .bind(&data.sponsor_purpose)
    .bind(data.amount)
    .bind(&data.payment_mode)
    .bind(&data.category)
    .bind(&data.purpose)
    .bind(&data.breakdown)
    .bind(&data.remark)
    .bind(&data.received_by)
    .bind(&data.club_name)
    .bind(&data.club_address)
    .execute(pool)
    .await?;

    find_by_id(pool, &id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Find a receipt by primary key.
pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Receipt>, sqlx::Error> {
    sqlx::query_as::<_, Receipt>(&format!("SELECT {RECEIPT_COLS} FROM receipts WHERE id = ?1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Find a receipt by its associated transaction ID.
pub async fn find_by_transaction_id(
    pool: &SqlitePool,
    transaction_id: &str,
) -> Result<Option<Receipt>, sqlx::Error> {
    sqlx::query_as::<_, Receipt>(&format!("SELECT {RECEIPT_COLS} FROM receipts WHERE transaction_id = ?1"))
        .bind(transaction_id)
        .fetch_optional(pool)
        .await
}

/// Find a receipt by membership ID (matches via member_code field).
pub async fn find_by_membership_id(
    pool: &SqlitePool,
    membership_id: &str,
) -> Result<Option<Receipt>, sqlx::Error> {
    sqlx::query_as::<_, Receipt>(&format!("SELECT {RECEIPT_COLS} FROM receipts WHERE member_code = ?1"))
        .bind(membership_id)
        .fetch_optional(pool)
        .await
}
