use axum::{Router, routing::get, extract::{State, Path}, Json};
use sqlx::SqlitePool;

use crate::auth::AuthSession;
use crate::routes::AppError;
use crate::services::receipt_service;

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/:transaction_id", get(get_receipt_by_transaction))
}

/// GET /api/receipts/:transaction_id
///
/// Returns the stored receipt snapshot for the given transaction as a
/// camelCase `ReceiptData` JSON object, ready for the frontend ReceiptView.
async fn get_receipt_by_transaction(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(transaction_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = claims;
    let receipt = receipt_service::receipt_json_by_transaction(&pool, &transaction_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;
    Ok(Json(receipt))
}
