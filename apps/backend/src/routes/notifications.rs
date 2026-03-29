use axum::{Router, routing::post, extract::State, Json};
use sqlx::SqlitePool;

use crate::auth::AuthSession;
use crate::auth::permissions::Role;
use crate::db::models::{Approval, Transaction, User};
use crate::integrations::whatsapp::WhatsappClient;
use crate::routes::AppError;
use crate::services::notification_service;

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/whatsapp", post(trigger_whatsapp))
}

/// POST /api/notifications/whatsapp
///
/// Manually re-trigger a WhatsApp notification for a given entity.
/// Admin only.
///
/// Body: { "type": "approval" | "payment" | "new_member" | "membership_approved"
///                | "expiry_reminder" | "membership_expired" | "sponsor_payment"
///                | "rejection",
///         "entityId": "<uuid>",
///         "tempPassword": "<str>" (optional, for membership_approved) }
async fn trigger_whatsapp(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = Role::from_str(&claims.role).unwrap_or(Role::Member);
    if role != Role::Admin {
        return Err(AppError::Forbidden);
    }

    let notification_type = body
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("`type` is required".into()))?
        .to_string();

    let entity_id = body
        .get("entityId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("`entityId` is required".into()))?
        .to_string();

    let temp_password = body
        .get("tempPassword")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Build WhatsApp client from environment (None if not configured)
    let client = WhatsappClient::from_env();
    let client_ref = client.as_ref();

    let result = match notification_type.as_str() {
        "approval" => {
            let row = sqlx::query_as::<_, Approval>("SELECT * FROM approvals WHERE id = ?1")
                .bind(&entity_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or(AppError::NotFound)?;

            notification_service::notify_new_approval_request(
                &pool,
                client_ref,
                &row,
                &claims.username,
            )
            .await
        }

        "payment" => {
            let tx = sqlx::query_as::<_, Transaction>("SELECT * FROM transactions WHERE id = ?1")
                .bind(&entity_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or(AppError::NotFound)?;

            // Look up member name if linked
            let member_name: Option<String> = if let Some(mid) = &tx.member_id {
                sqlx::query_scalar::<_, String>("SELECT name FROM members WHERE id = ?1")
                    .bind(mid)
                    .fetch_optional(&pool)
                    .await
                    .unwrap_or(None)
            } else {
                None
            };

            notification_service::notify_payment_received(
                &pool,
                client_ref,
                &tx,
                member_name.as_deref(),
            )
            .await
        }

        "new_member" => {
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?1")
                .bind(&entity_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or(AppError::NotFound)?;

            notification_service::notify_new_member_registration(&pool, client_ref, &user).await
        }

        "membership_approved" => {
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?1")
                .bind(&entity_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or(AppError::NotFound)?;

            let password = temp_password.unwrap_or_else(|| "(check with admin)".into());
            let login_url =
                std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3001".into())
                    + "/login";

            notification_service::notify_membership_approved(
                &pool,
                client_ref,
                &user,
                &password,
                &login_url,
            )
            .await
        }

        "expiry_reminder" => {
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?1")
                .bind(&entity_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or(AppError::NotFound)?;

            let days_left: i64 = user
                .membership_expiry
                .as_deref()
                .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .map(|exp| {
                    let today = chrono::Local::now().date_naive();
                    (exp - today).num_days()
                })
                .unwrap_or(0)
                .max(0);

            notification_service::notify_membership_expiry_reminder(
                &pool,
                client_ref,
                &user,
                days_left,
            )
            .await
        }

        "membership_expired" => {
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?1")
                .bind(&entity_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or(AppError::NotFound)?;

            notification_service::notify_membership_expired(&pool, client_ref, &user).await
        }

        other => {
            return Err(AppError::BadRequest(format!(
                "Unknown notification type: {other}"
            )));
        }
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "sent": result.sent,
        "failed": result.failed,
    })))
}
