use axum::{
    extract::State,
    http::{header::SET_COOKIE, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::{create_session_token, make_cookie, AuthSession, SessionClaims};
use crate::support::member_id::{generate_member_id, generate_sub_member_id, parse_sequence_number};

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/profile", post(complete_profile))
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct SubMemberInput {
    name: String,
    email: String,
    phone: String,
    relation: String,
}

#[derive(serde::Deserialize)]
struct CompleteProfileRequest {
    name: String,
    phone: String,
    address: String,
    #[serde(rename = "subMembers", default)]
    sub_members: Vec<SubMemberInput>,
}

// ---------------------------------------------------------------------------
// POST /api/onboarding/profile
// ---------------------------------------------------------------------------

async fn complete_profile(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Json(body): Json<CompleteProfileRequest>,
) -> impl IntoResponse {
    // Validate required fields
    if body.name.trim().is_empty() || body.phone.trim().is_empty() || body.address.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name, phone, and address are required" })),
        )
            .into_response();
    }

    // Check user exists and get email
    let user_row = sqlx::query_as::<_, (String, String)>(
        "SELECT id, email FROM users WHERE id = ?",
    )
    .bind(&claims.user_id)
    .fetch_optional(&pool)
    .await;

    let (_user_id, user_email) = match user_row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "user not found" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("onboarding user lookup failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    // Check if a member record already exists for this user
    let existing = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM members WHERE user_id = ?",
    )
    .bind(&claims.user_id)
    .fetch_optional(&pool)
    .await;

    if let Ok(Some(_)) = existing {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "profile already completed" })),
        )
            .into_response();
    }

    // Determine next member sequence for the current year
    let year = chrono::Utc::now().format("%Y").to_string().parse::<u32>().unwrap_or(2026);
    let prefix = format!("BSDS-{year}-");

    let max_seq: Option<String> = sqlx::query_scalar(
        "SELECT member_id FROM members WHERE member_id LIKE ? ORDER BY member_id DESC LIMIT 1",
    )
    .bind(format!("{prefix}%"))
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let current_max = max_seq.as_deref().and_then(parse_sequence_number);
    let new_member_id = generate_member_id(year, current_max);

    // Insert member record
    let member_record_id = Uuid::new_v4().to_string();
    let insert_member = sqlx::query(
        "INSERT INTO members (id, user_id, name, phone, email, address, joined_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'), datetime('now'))",
    )
    .bind(&member_record_id)
    .bind(&claims.user_id)
    .bind(body.name.trim())
    .bind(body.phone.trim())
    .bind(&user_email)
    .bind(body.address.trim())
    .execute(&pool)
    .await;

    if let Err(e) = insert_member {
        tracing::error!("onboarding member insert failed: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create member record" })),
        )
            .into_response();
    }

    // Update user record with member_id, name, phone, address
    let _ = sqlx::query(
        "UPDATE users SET member_id = ?, name = ?, phone = ?, address = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&new_member_id)
    .bind(body.name.trim())
    .bind(body.phone.trim())
    .bind(body.address.trim())
    .bind(&claims.user_id)
    .execute(&pool)
    .await;

    // Insert sub-members (up to 3)
    for (i, sm) in body.sub_members.iter().take(3).enumerate() {
        if sm.name.trim().is_empty() || sm.email.trim().is_empty() {
            continue;
        }
        let sub_id = Uuid::new_v4().to_string();
        let sub_member_id = generate_sub_member_id(&new_member_id, (i + 1) as u32)
            .unwrap_or_else(|_| format!("{}-0{}", new_member_id, i + 1));

        // sub_members need a password; generate a temp one
        let temp_pw = crate::auth::temp_password::generate_temp_password_default();
        let hashed = match crate::auth::hash_password(&temp_pw) {
            Ok(h) => h,
            Err(_) => continue,
        };

        let _ = sqlx::query(
            "INSERT INTO sub_members (id, member_id, parent_user_id, name, email, phone, password, is_temp_password, relation, can_login, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, 0, datetime('now'))",
        )
        .bind(&sub_id)
        .bind(&sub_member_id)
        .bind(&claims.user_id)
        .bind(sm.name.trim())
        .bind(sm.email.trim())
        .bind(sm.phone.trim())
        .bind(&hashed)
        .bind(sm.relation.trim())
        .execute(&pool)
        .await;
    }

    // Activity log
    let log_id = Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT INTO activity_logs (id, user_id, action, description, metadata, created_at)
         VALUES (?, ?, 'member_registered', ?, NULL, datetime('now'))",
    )
    .bind(&log_id)
    .bind(&claims.user_id)
    .bind(format!("Member {} ({}) completed their profile", body.name.trim(), new_member_id))
    .execute(&pool)
    .await;

    // Re-issue session cookie with updated member_id
    let new_claims = SessionClaims {
        user_id: claims.user_id.clone(),
        username: claims.username.clone(),
        role: claims.role.clone(),
        member_id: Some(new_member_id.clone()),
        must_change_password: false,
    };
    let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
    let token = create_session_token(&new_claims, &secret);

    (
        StatusCode::OK,
        [(SET_COOKIE, make_cookie(&token))],
        Json(serde_json::json!({
            "success": true,
            "memberId": new_member_id,
            "memberRecordId": member_record_id,
        })),
    )
        .into_response()
}
