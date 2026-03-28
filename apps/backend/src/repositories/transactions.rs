//! Repository functions for the `transactions` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::Transaction;

/// Explicit column list for SELECT queries on the transactions table.
///
/// SQLite NUMERIC affinity stores whole-number floats as INTEGER, which
/// causes sqlx's f64 decoder to fail.  Wrapping `amount` in CAST(… AS REAL)
/// forces it back to a floating-point type before sqlx decodes it.
const TX_COLS: &str =
    "id, \"type\", category, CAST(amount AS REAL) as amount, payment_mode, purpose, remark, \
     sponsor_purpose, member_id, sponsor_id, entered_by_id, approval_status, approval_source, \
     approved_by_id, approved_at, razorpay_payment_id, razorpay_order_id, \
     sender_name, sender_phone, sender_upi_id, sender_bank_account, sender_bank_name, \
     sponsor_sender_name, sponsor_sender_contact, \
     receipt_number, includes_subscription, includes_annual_fee, includes_application_fee, \
     created_at";

// ---------------------------------------------------------------------------
// Filter / input types
// ---------------------------------------------------------------------------

/// Filters for listing transactions.
#[derive(Debug, Clone, Default)]
pub struct TransactionListFilters {
    pub r#type: Option<String>,
    pub category: Option<String>,
    pub payment_mode: Option<String>,
    pub status: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub page: u32,
    pub limit: u32,
}

/// Input data for creating a transaction.
#[derive(Debug, Clone)]
pub struct CreateTransactionData {
    pub r#type: String,
    pub category: String,
    pub amount: f64,
    pub payment_mode: String,
    pub purpose: String,
    pub remark: Option<String>,
    pub sponsor_purpose: Option<String>,
    pub member_id: Option<String>,
    pub sponsor_id: Option<String>,
    pub entered_by_id: String,
    pub approval_status: String,
    pub approval_source: String,
    pub approved_by_id: Option<String>,
    pub approved_at: Option<String>,
    pub razorpay_payment_id: Option<String>,
    pub razorpay_order_id: Option<String>,
    pub sender_name: Option<String>,
    pub sender_phone: Option<String>,
    pub sender_upi_id: Option<String>,
    pub sender_bank_account: Option<String>,
    pub sender_bank_name: Option<String>,
    pub sponsor_sender_name: Option<String>,
    pub sponsor_sender_contact: Option<String>,
    pub receipt_number: Option<String>,
    pub includes_subscription: bool,
    pub includes_annual_fee: bool,
    pub includes_application_fee: bool,
}

/// Input data for updating a transaction.
#[derive(Debug, Clone, Default)]
pub struct UpdateTransactionData {
    pub approval_status: Option<String>,
    pub approved_by_id: Option<String>,
    pub approved_at: Option<String>,
    pub receipt_number: Option<String>,
}

/// Aggregated transaction summary.
#[derive(Debug, Clone)]
pub struct TransactionSummary {
    pub total_income: f64,
    pub total_expenses: f64,
    pub pending_amount: f64,
    pub net_balance: f64,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new transaction record.
pub async fn create(
    pool: &SqlitePool,
    data: &CreateTransactionData,
) -> Result<Transaction, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (
            id, type, category, amount, payment_mode, purpose, remark,
            sponsor_purpose, member_id, sponsor_id, entered_by_id,
            approval_status, approval_source, approved_by_id, approved_at,
            razorpay_payment_id, razorpay_order_id,
            sender_name, sender_phone, sender_upi_id, sender_bank_account, sender_bank_name,
            sponsor_sender_name, sponsor_sender_contact,
            receipt_number, includes_subscription, includes_annual_fee, includes_application_fee
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28
         )",
    )
    .bind(&id)
    .bind(&data.r#type)
    .bind(&data.category)
    .bind(data.amount)
    .bind(&data.payment_mode)
    .bind(&data.purpose)
    .bind(&data.remark)
    .bind(&data.sponsor_purpose)
    .bind(&data.member_id)
    .bind(&data.sponsor_id)
    .bind(&data.entered_by_id)
    .bind(&data.approval_status)
    .bind(&data.approval_source)
    .bind(&data.approved_by_id)
    .bind(&data.approved_at)
    .bind(&data.razorpay_payment_id)
    .bind(&data.razorpay_order_id)
    .bind(&data.sender_name)
    .bind(&data.sender_phone)
    .bind(&data.sender_upi_id)
    .bind(&data.sender_bank_account)
    .bind(&data.sender_bank_name)
    .bind(&data.sponsor_sender_name)
    .bind(&data.sponsor_sender_contact)
    .bind(&data.receipt_number)
    .bind(data.includes_subscription)
    .bind(data.includes_annual_fee)
    .bind(data.includes_application_fee)
    .execute(pool)
    .await?;

    find_by_id(pool, &id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Find a transaction by primary key.
pub async fn find_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<Transaction>, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(&format!("SELECT {TX_COLS} FROM transactions WHERE id = ?1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// List transactions with filters and pagination.
pub async fn list(
    pool: &SqlitePool,
    filters: &TransactionListFilters,
) -> Result<(Vec<Transaction>, i64), sqlx::Error> {
    let offset = (filters.page.saturating_sub(1)) * filters.limit;

    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref t) = filters.r#type {
        let idx = bind_values.len() + 1;
        conditions.push(format!("type = ?{idx}"));
        bind_values.push(t.clone());
    }
    if let Some(ref cat) = filters.category {
        let idx = bind_values.len() + 1;
        conditions.push(format!("category = ?{idx}"));
        bind_values.push(cat.clone());
    }
    if let Some(ref pm) = filters.payment_mode {
        let idx = bind_values.len() + 1;
        conditions.push(format!("payment_mode = ?{idx}"));
        bind_values.push(pm.clone());
    }
    if let Some(ref status) = filters.status {
        let idx = bind_values.len() + 1;
        conditions.push(format!("approval_status = ?{idx}"));
        bind_values.push(status.clone());
    }
    if let Some(ref df) = filters.date_from {
        let idx = bind_values.len() + 1;
        conditions.push(format!("created_at >= ?{idx}"));
        bind_values.push(df.clone());
    }
    if let Some(ref dt) = filters.date_to {
        let idx = bind_values.len() + 1;
        // Include the full day
        conditions.push(format!("created_at <= ?{idx}"));
        bind_values.push(format!("{dt}T23:59:59Z"));
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    let count_sql = format!("SELECT COUNT(*) FROM transactions WHERE {where_clause}");
    let list_sql = format!(
        "SELECT {TX_COLS} FROM transactions WHERE {where_clause} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for v in &bind_values {
        count_query = count_query.bind(v);
    }
    let total = count_query.fetch_one(pool).await.unwrap_or(0);

    let mut list_query = sqlx::query_as::<_, Transaction>(&list_sql);
    for v in &bind_values {
        list_query = list_query.bind(v);
    }
    list_query = list_query.bind(filters.limit as i64).bind(offset as i64);
    let transactions = list_query.fetch_all(pool).await?;

    Ok((transactions, total))
}

/// Get an aggregated transaction summary: total income, expenses, pending, net balance.
pub async fn summary(pool: &SqlitePool) -> Result<TransactionSummary, sqlx::Error> {
    let income: Option<f64> = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(amount), 0.0) AS REAL) FROM transactions WHERE type = 'CASH_IN' AND approval_status = 'APPROVED'",
    )
    .fetch_one(pool)
    .await?;

    let expenses: Option<f64> = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(amount), 0.0) AS REAL) FROM transactions WHERE type = 'CASH_OUT' AND approval_status = 'APPROVED'",
    )
    .fetch_one(pool)
    .await?;

    let pending: Option<f64> = sqlx::query_scalar(
        "SELECT CAST(COALESCE(SUM(amount), 0.0) AS REAL) FROM transactions WHERE approval_status = 'PENDING'",
    )
    .fetch_one(pool)
    .await?;

    let total_income = income.unwrap_or(0.0);
    let total_expenses = expenses.unwrap_or(0.0);
    let pending_amount = pending.unwrap_or(0.0);

    Ok(TransactionSummary {
        total_income,
        total_expenses,
        pending_amount,
        net_balance: total_income - total_expenses,
    })
}

/// Update transaction fields (used for approval status changes).
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    data: &UpdateTransactionData,
) -> Result<Transaction, sqlx::Error> {
    let existing = find_by_id(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;

    let approval_status = data
        .approval_status
        .as_deref()
        .unwrap_or(&existing.approval_status);
    let approved_by_id = data.approved_by_id.as_deref().or(existing.approved_by_id.as_deref());
    let approved_at = data.approved_at.as_deref().or(existing.approved_at.as_deref());
    let receipt_number = data.receipt_number.as_deref().or(existing.receipt_number.as_deref());

    sqlx::query(
        "UPDATE transactions SET approval_status = ?1, approved_by_id = ?2, approved_at = ?3,
         receipt_number = ?4 WHERE id = ?5",
    )
    .bind(approval_status)
    .bind(approved_by_id)
    .bind(approved_at)
    .bind(receipt_number)
    .bind(id)
    .execute(pool)
    .await?;

    find_by_id(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Delete a transaction (returns error — transactions are immutable).
pub async fn delete(_pool: &SqlitePool, _id: &str) -> Result<(), sqlx::Error> {
    // Transactions are immutable per business rules. This is a no-op placeholder.
    Err(sqlx::Error::Protocol(
        "Transactions are immutable and cannot be deleted".to_string(),
    ))
}
