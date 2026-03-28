//! Repository functions for the `audit_logs` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::AuditLog;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input data for creating an audit log entry.
#[derive(Debug, Clone)]
pub struct CreateAuditLogData {
    pub transaction_id: String,
    pub event_type: String,
    pub transaction_snapshot: String,
    pub performed_by_id: String,
}

/// Filters for listing audit logs.
#[derive(Debug, Clone, Default)]
pub struct AuditLogListFilters {
    pub transaction_id: Option<String>,
    pub event_type: Option<String>,
    pub performed_by_id: Option<String>,
    pub page: u32,
    pub limit: u32,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new audit log entry.
pub async fn create(
    pool: &SqlitePool,
    data: &CreateAuditLogData,
) -> Result<AuditLog, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO audit_logs (id, transaction_id, event_type, transaction_snapshot, performed_by_id)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&id)
    .bind(&data.transaction_id)
    .bind(&data.event_type)
    .bind(&data.transaction_snapshot)
    .bind(&data.performed_by_id)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, AuditLog>("SELECT * FROM audit_logs WHERE id = ?1")
        .bind(&id)
        .fetch_one(pool)
        .await
}

/// List audit logs with filters and pagination.
pub async fn list(
    pool: &SqlitePool,
    filters: &AuditLogListFilters,
) -> Result<(Vec<AuditLog>, i64), sqlx::Error> {
    let offset = (filters.page.saturating_sub(1)) * filters.limit;

    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref tid) = filters.transaction_id {
        let idx = bind_values.len() + 1;
        conditions.push(format!("transaction_id = ?{idx}"));
        bind_values.push(tid.clone());
    }
    if let Some(ref et) = filters.event_type {
        let idx = bind_values.len() + 1;
        conditions.push(format!("event_type = ?{idx}"));
        bind_values.push(et.clone());
    }
    if let Some(ref pid) = filters.performed_by_id {
        let idx = bind_values.len() + 1;
        conditions.push(format!("performed_by_id = ?{idx}"));
        bind_values.push(pid.clone());
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    let count_sql = format!("SELECT COUNT(*) FROM audit_logs WHERE {where_clause}");
    let list_sql = format!(
        "SELECT * FROM audit_logs WHERE {where_clause} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2,
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for v in &bind_values {
        count_query = count_query.bind(v);
    }
    let total = count_query.fetch_one(pool).await.unwrap_or(0);

    let mut list_query = sqlx::query_as::<_, AuditLog>(&list_sql);
    for v in &bind_values {
        list_query = list_query.bind(v);
    }
    list_query = list_query.bind(filters.limit as i64).bind(offset as i64);
    let logs = list_query.fetch_all(pool).await?;

    Ok((logs, total))
}
