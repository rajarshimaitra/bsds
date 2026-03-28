//! Repository functions for the `approvals` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::Approval;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input data for creating an approval record.
#[derive(Debug, Clone)]
pub struct CreateApprovalData {
    pub entity_type: String,
    pub entity_id: String,
    pub action: String,
    pub previous_data: Option<String>,
    pub new_data: Option<String>,
    pub requested_by_id: String,
    pub status: String,
}

/// Filters for listing approvals.
#[derive(Debug, Clone, Default)]
pub struct ApprovalListFilters {
    pub entity_type: Option<String>,
    pub status: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub page: u32,
    pub limit: u32,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new approval record.
pub async fn create(
    pool: &SqlitePool,
    data: &CreateApprovalData,
) -> Result<Approval, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO approvals (id, entity_type, entity_id, action, previous_data, new_data, requested_by_id, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(&id)
    .bind(&data.entity_type)
    .bind(&data.entity_id)
    .bind(&data.action)
    .bind(&data.previous_data)
    .bind(&data.new_data)
    .bind(&data.requested_by_id)
    .bind(&data.status)
    .execute(pool)
    .await?;

    find_by_id(pool, &id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Find an approval by primary key.
pub async fn find_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<Approval>, sqlx::Error> {
    sqlx::query_as::<_, Approval>("SELECT * FROM approvals WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// List all pending approvals.
pub async fn list_pending(pool: &SqlitePool) -> Result<Vec<Approval>, sqlx::Error> {
    sqlx::query_as::<_, Approval>(
        "SELECT * FROM approvals WHERE status = 'PENDING' ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
}

/// List approvals with filters and pagination.
///
/// If `status` is `None`, defaults to showing only PENDING.
/// If `status` is `"ALL"`, shows all statuses.
pub async fn list_all(
    pool: &SqlitePool,
    filters: &ApprovalListFilters,
) -> Result<(Vec<Approval>, i64, i64), sqlx::Error> {
    let offset = (filters.page.saturating_sub(1)) * filters.limit;

    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref et) = filters.entity_type {
        let idx = bind_values.len() + 1;
        conditions.push(format!("entity_type = ?{idx}"));
        bind_values.push(et.clone());
    }

    match filters.status.as_deref() {
        None => {
            conditions.push("status = 'PENDING'".to_string());
        }
        Some("ALL") => {
            // No status filter
        }
        Some(s) => {
            let idx = bind_values.len() + 1;
            conditions.push(format!("status = ?{idx}"));
            bind_values.push(s.to_string());
        }
    }

    if let Some(ref df) = filters.date_from {
        let idx = bind_values.len() + 1;
        conditions.push(format!("created_at >= ?{idx}"));
        bind_values.push(df.clone());
    }
    if let Some(ref dt) = filters.date_to {
        let idx = bind_values.len() + 1;
        conditions.push(format!("created_at <= ?{idx}"));
        bind_values.push(format!("{dt}T23:59:59Z"));
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    let count_sql = format!("SELECT COUNT(*) FROM approvals WHERE {where_clause}");
    let list_sql = format!(
        "SELECT * FROM approvals WHERE {where_clause} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for v in &bind_values {
        count_query = count_query.bind(v);
    }
    let total = count_query.fetch_one(pool).await.unwrap_or(0);

    let mut list_query = sqlx::query_as::<_, Approval>(&list_sql);
    for v in &bind_values {
        list_query = list_query.bind(v);
    }
    list_query = list_query.bind(filters.limit as i64).bind(offset as i64);
    let approvals = list_query.fetch_all(pool).await?;

    // Always fetch the total pending count for the badge
    let pending_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM approvals WHERE status = 'PENDING'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    Ok((approvals, total, pending_count))
}

/// Update the status of an approval record.
pub async fn update_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    reviewer_id: &str,
    notes: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE approvals SET status = ?1, reviewed_by_id = ?2,
         reviewed_at = strftime('%Y-%m-%dT%H:%M:%SZ','now'), notes = ?3
         WHERE id = ?4",
    )
    .bind(status)
    .bind(reviewer_id)
    .bind(notes)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
