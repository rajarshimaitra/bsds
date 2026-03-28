//! Repository functions for the `activity_logs` table.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::models::ActivityLog;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Input data for creating an activity log entry.
#[derive(Debug, Clone)]
pub struct CreateActivityLogData {
    pub user_id: String,
    pub action: String,
    pub description: String,
    pub metadata: Option<String>,
}

/// Filters for listing activity logs.
#[derive(Debug, Clone, Default)]
pub struct ActivityLogListFilters {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub page: u32,
    pub limit: u32,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Create a new activity log entry.
pub async fn create(
    pool: &SqlitePool,
    data: &CreateActivityLogData,
) -> Result<ActivityLog, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO activity_logs (id, user_id, action, description, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&id)
    .bind(&data.user_id)
    .bind(&data.action)
    .bind(&data.description)
    .bind(&data.metadata)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, ActivityLog>("SELECT * FROM activity_logs WHERE id = ?1")
        .bind(&id)
        .fetch_one(pool)
        .await
}

/// List activity logs with filters and pagination.
pub async fn list(
    pool: &SqlitePool,
    filters: &ActivityLogListFilters,
) -> Result<(Vec<ActivityLog>, i64), sqlx::Error> {
    let offset = (filters.page.saturating_sub(1)) * filters.limit;

    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref uid) = filters.user_id {
        let idx = bind_values.len() + 1;
        conditions.push(format!("user_id = ?{idx}"));
        bind_values.push(uid.clone());
    }
    if let Some(ref action) = filters.action {
        let idx = bind_values.len() + 1;
        conditions.push(format!("action = ?{idx}"));
        bind_values.push(action.clone());
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    let count_sql = format!("SELECT COUNT(*) FROM activity_logs WHERE {where_clause}");
    let list_sql = format!(
        "SELECT * FROM activity_logs WHERE {where_clause} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2,
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for v in &bind_values {
        count_query = count_query.bind(v);
    }
    let total = count_query.fetch_one(pool).await.unwrap_or(0);

    let mut list_query = sqlx::query_as::<_, ActivityLog>(&list_sql);
    for v in &bind_values {
        list_query = list_query.bind(v);
    }
    list_query = list_query.bind(filters.limit as i64).bind(offset as i64);
    let logs = list_query.fetch_all(pool).await?;

    Ok((logs, total))
}
