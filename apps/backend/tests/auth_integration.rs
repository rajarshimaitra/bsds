//! # Auth Integration Tests
//!
//! Covers: login flow, session lifecycle (me endpoint), logout cookie clearing,
//!         change-password happy path and error paths, temp-password flag
//!         propagation, empty credential rejection.
//!
//! Does NOT cover: password reset via email (not implemented), OAuth,
//!                 rate-limiting (separate middleware concern).
//!
//! Protects: apps/backend/src/routes/auth.rs, apps/backend/src/auth/mod.rs

mod common;

use axum_test::TestServer;
use serde_json::json;

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

#[tokio::test]
async fn login_with_valid_credentials_returns_200_and_sets_session_cookie() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": user.email, "password": user.password_plain }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["role"], "ADMIN");
    assert!(body["id"].as_str().is_some());

    // Session cookie must be present in response headers
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("set-cookie header missing");
    assert!(set_cookie.contains("bsds_session="), "cookie name missing");
    assert!(set_cookie.contains("HttpOnly"), "HttpOnly flag missing");
    assert!(set_cookie.contains("SameSite=Strict"), "SameSite flag missing");
}

#[tokio::test]
async fn login_with_wrong_password_returns_401() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": user.email, "password": "totally-wrong" }))
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
    let body = resp.json::<serde_json::Value>();
    assert!(body["error"].as_str().is_some());
}

#[tokio::test]
async fn login_with_unknown_email_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": "nobody@test.local", "password": "pass" }))
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_with_empty_username_returns_400() {
    let (server, _pool, _dir) = common::test_app().await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": "", "password": "pass" }))
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_with_empty_password_returns_400() {
    let (server, _pool, _dir) = common::test_app().await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": "user@test.local", "password": "" }))
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_username_is_case_insensitive() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;

    // Use uppercase email
    let resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": user.email.to_uppercase(), "password": user.password_plain }))
        .await;

    resp.assert_status_ok();
}

#[tokio::test]
async fn login_response_includes_must_change_password_flag() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;
    // Admin seeded with is_temp_password = 0, so mustChangePassword should be false
    let resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": user.email, "password": user.password_plain }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["mustChangePassword"], false);
}

#[tokio::test]
async fn login_response_returns_member_id() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": user.email, "password": user.password_plain }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body["memberId"].as_str().is_some(), "memberId missing from response");
}

// ---------------------------------------------------------------------------
// Logout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn logout_returns_200_and_clears_cookie() {
    let (server, _pool, _dir) = common::test_app().await;

    let resp = server.post("/api/auth/logout").await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["ok"], true);

    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("set-cookie header missing on logout");
    assert!(
        set_cookie.contains("Max-Age=0"),
        "cookie not cleared on logout: {set_cookie}"
    );
}

// ---------------------------------------------------------------------------
// Me endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn me_without_session_cookie_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;

    let resp = server.get("/api/auth/me").await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_with_valid_session_cookie_returns_user_info() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&user);

    let resp = server
        .get("/api/auth/me")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["role"], "ADMIN");
    assert_eq!(body["id"], user.id.as_str());
}

#[tokio::test]
async fn me_with_tampered_session_cookie_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;

    let resp = server
        .get("/api/auth/me")
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            "bsds_session=this.is.not.valid".parse().unwrap(),
        )
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Change password
// ---------------------------------------------------------------------------

#[tokio::test]
async fn change_password_with_correct_current_password_succeeds() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&user);

    let resp = server
        .post("/api/auth/change-password")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "currentPassword": user.password_plain,
            "newPassword": "NewSecurePassword2!"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn change_password_with_wrong_current_password_returns_400() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&user);

    let resp = server
        .post("/api/auth/change-password")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "currentPassword": "definitely-wrong",
            "newPassword": "NewSecurePassword2!"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let body = resp.json::<serde_json::Value>();
    assert!(body["error"].as_str().unwrap().contains("incorrect"));
}

#[tokio::test]
async fn change_password_with_empty_new_password_returns_400() {
    let (server, pool, _dir) = common::test_app().await;
    let user = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&user);

    let resp = server
        .post("/api/auth/change-password")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "currentPassword": user.password_plain,
            "newPassword": ""
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn change_password_without_session_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;

    let resp = server
        .post("/api/auth/change-password")
        .json(&json!({
            "currentPassword": "old",
            "newPassword": "new"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn change_password_clears_temp_password_flag_in_db() {
    let (server, pool, _dir) = common::test_app().await;

    // Insert user with is_temp_password = 1
    let user_id = uuid::Uuid::new_v4().to_string();
    let hashed = bsds_backend::auth::hash_password("TempPass1!").unwrap();
    sqlx::query(
        "INSERT INTO users (id, member_id, name, email, phone, address, password, is_temp_password, role, membership_status, application_fee_paid)
         VALUES (?1, 'BSDS-2026-8888-00', 'Temp User', 'temp@test.local', '+910000000000', 'Addr', ?2, 1, 'ADMIN', 'ACTIVE', 1)",
    )
    .bind(&user_id)
    .bind(&hashed)
    .execute(&pool)
    .await
    .unwrap();

    let claims = bsds_backend::auth::SessionClaims {
        user_id: user_id.clone(),
        username: "temp@test.local".to_string(),
        role: "ADMIN".to_string(),
        member_id: Some("BSDS-2026-8888-00".to_string()),
        must_change_password: true,
    };
    let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
    let token = bsds_backend::auth::create_session_token(&claims, &secret);
    let cookie = format!("bsds_session={token}");

    let resp = server
        .post("/api/auth/change-password")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({ "currentPassword": "TempPass1!", "newPassword": "RealPass2!" }))
        .await;

    resp.assert_status_ok();

    // Verify DB flag cleared
    let is_temp: bool = sqlx::query_scalar("SELECT is_temp_password FROM users WHERE id = ?1")
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!is_temp, "is_temp_password should be false after password change");
}
