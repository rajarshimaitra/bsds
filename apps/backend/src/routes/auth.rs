use axum::{
    extract::State,
    http::{header::SET_COOKIE, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use sqlx::SqlitePool;

use crate::auth::{
    clear_cookie, create_session_token, hash_password, make_cookie, verify_password,
    AuthSession, SessionClaims,
};

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/change-password", post(change_password))
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(serde::Serialize)]
struct UserResponse {
    id: String,
    username: String,
    role: String,
    #[serde(rename = "memberId")]
    member_id: Option<String>,
    #[serde(rename = "mustChangePassword")]
    must_change_password: bool,
}

#[derive(serde::Deserialize)]
struct ChangePasswordRequest {
    #[serde(rename = "currentPassword")]
    current_password: String,
    #[serde(rename = "newPassword")]
    new_password: String,
}

// ---------------------------------------------------------------------------
// POST /login
// ---------------------------------------------------------------------------

async fn login(
    State(pool): State<SqlitePool>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let username = body.username.trim().to_lowercase();

    if username.is_empty() || body.password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "username and password are required" })),
        )
            .into_response();
    }

    // Look up user by email (the legacy system uses email as the login field)
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, member_id, email, name, password, role, is_temp_password FROM users WHERE email = ?",
    )
    .bind(&username)
    .fetch_optional(&pool)
    .await;

    let user = match user {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "invalid credentials" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("login query failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    let password_ok = match verify_password(&body.password, &user.password) {
        Ok(ok) => ok,
        Err(e) => {
            tracing::error!("bcrypt verify failed for user {}: {e}", user.id);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    if !password_ok {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid credentials" })),
        )
            .into_response();
    }

    let claims = SessionClaims {
        user_id: user.id.clone(),
        username: user.email.clone(),
        role: user.role.clone(),
        member_id: Some(user.member_id.clone()),
        must_change_password: user.is_temp_password,
    };

    let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
    let token = create_session_token(&claims, &secret);

    let response_body = UserResponse {
        id: user.id,
        username: user.email,
        role: user.role,
        member_id: Some(user.member_id),
        must_change_password: user.is_temp_password,
    };

    (
        StatusCode::OK,
        [(SET_COOKIE, make_cookie(&token))],
        Json(serde_json::to_value(response_body).unwrap()),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// POST /logout
// ---------------------------------------------------------------------------

async fn logout() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(SET_COOKIE, clear_cookie())],
        Json(serde_json::json!({ "ok": true })),
    )
}

// ---------------------------------------------------------------------------
// GET /me
// ---------------------------------------------------------------------------

async fn me(AuthSession(claims): AuthSession) -> impl IntoResponse {
    let body = UserResponse {
        id: claims.user_id,
        username: claims.username,
        role: claims.role,
        member_id: claims.member_id,
        must_change_password: claims.must_change_password,
    };
    (StatusCode::OK, Json(serde_json::to_value(body).unwrap()))
}

// ---------------------------------------------------------------------------
// POST /change-password
// ---------------------------------------------------------------------------

async fn change_password(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Json(body): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    if body.new_password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "new password must not be empty" })),
        )
            .into_response();
    }

    // Fetch current password hash from DB
    let row = sqlx::query_as::<_, PasswordRow>(
        "SELECT id, password FROM users WHERE id = ?",
    )
    .bind(&claims.user_id)
    .fetch_optional(&pool)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "account not found" })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("change-password query failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    // Verify current password
    let current_ok = match verify_password(&body.current_password, &row.password) {
        Ok(ok) => ok,
        Err(e) => {
            tracing::error!("bcrypt verify failed for user {}: {e}", row.id);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    if !current_ok {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "current password is incorrect" })),
        )
            .into_response();
    }

    // Hash new password and update DB
    let new_hash = match hash_password(&body.new_password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("bcrypt hash failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    let result = sqlx::query(
        "UPDATE users SET password = ?, is_temp_password = false, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&new_hash)
    .bind(&claims.user_id)
    .execute(&pool)
    .await;

    if let Err(e) = result {
        tracing::error!("change-password update failed: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "internal server error" })),
        )
            .into_response();
    }

    // Log the password change event
    let log_id = uuid::Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT INTO activity_logs (id, user_id, action, description, metadata, created_at)
         VALUES (?, ?, 'password_changed', ?, NULL, datetime('now'))",
    )
    .bind(&log_id)
    .bind(&claims.user_id)
    .bind(format!("User {} changed their password", claims.username))
    .execute(&pool)
    .await;

    // Re-issue session cookie with must_change_password = false
    let new_claims = SessionClaims {
        user_id: claims.user_id.clone(),
        username: claims.username.clone(),
        role: claims.role.clone(),
        member_id: claims.member_id.clone(),
        must_change_password: false,
    };
    let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
    let token = create_session_token(&new_claims, &secret);

    (
        StatusCode::OK,
        [(SET_COOKIE, make_cookie(&token))],
        Json(serde_json::json!({ "success": true })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Internal row types for sqlx queries
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    member_id: String,
    email: String,
    #[allow(dead_code)]
    name: String,
    password: String,
    role: String,
    is_temp_password: bool,
}

#[derive(sqlx::FromRow)]
struct PasswordRow {
    id: String,
    password: String,
}
