//! Consolidated integration tests for authentication, role-based access control,
//! and cron endpoint security.
//!
//! Merges tests from:
//! - `auth_integration.rs` (login, session, logout, change-password flows)
//! - `security_integration.rs` (unauthenticated + per-role access control)
//! - `cron_integration.rs` (cron auth variants and response shape)
//!
//! Tests are grouped into 3 large functions that share `test_app()` instances
//! to reduce setup overhead while keeping logical grouping clear.

mod common;

use axum::http::{HeaderName, HeaderValue, StatusCode};
use serde_json::json;
use uuid::Uuid;

/// Convenience macro for attaching a session cookie to a request builder.
macro_rules! with_cookie {
    ($req:expr, $cookie:expr) => {
        $req.add_header(
            HeaderName::from_static("cookie"),
            $cookie.parse::<HeaderValue>().unwrap(),
        )
    };
}

// ---------------------------------------------------------------------------
// 1. Login, session, and password lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn login_session_and_password_lifecycle() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // ── Login with valid credentials ────────────────────────────────────
    let res = server
        .post("/api/auth/login")
        .json(&json!({
            "username": admin.email,
            "password": admin.password_plain,
        }))
        .await;

    assert_eq!(res.status_code(), StatusCode::OK, "valid login should return 200");

    let body: serde_json::Value = res.json();
    assert_eq!(body["role"], "ADMIN", "login response should include role=ADMIN");
    assert!(body["id"].as_str().is_some(), "login response should include user id");

    let set_cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("login should set a cookie");
    assert!(
        set_cookie.contains("bsds_session="),
        "set-cookie should contain bsds_session token"
    );
    assert!(
        set_cookie.contains("HttpOnly"),
        "session cookie should be HttpOnly"
    );
    assert!(
        set_cookie.contains("SameSite=Strict"),
        "session cookie should be SameSite=Strict"
    );

    // ── Login response includes mustChangePassword flag ─────────────────
    assert_eq!(
        body["mustChangePassword"], false,
        "normal user should have mustChangePassword=false"
    );

    // ── Login response includes memberId ────────────────────────────────
    assert!(
        body.get("memberId").is_some(),
        "login response should include memberId field"
    );

    // ── Login – username is case-insensitive ────────────────────────────
    let res = server
        .post("/api/auth/login")
        .json(&json!({
            "username": admin.email.to_uppercase(),
            "password": admin.password_plain,
        }))
        .await;

    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "login should be case-insensitive for email"
    );

    // ── Login with wrong password ───────────────────────────────────────
    let res = server
        .post("/api/auth/login")
        .json(&json!({
            "username": admin.email,
            "password": "absolutely-wrong",
        }))
        .await;

    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "wrong password should return 401"
    );
    let body: serde_json::Value = res.json();
    assert!(
        body.get("error").is_some(),
        "401 response should include an error field"
    );

    // ── Login with unknown email ────────────────────────────────────────
    let res = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "nobody@test.local",
            "password": "irrelevant",
        }))
        .await;

    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "unknown email should return 401"
    );

    // ── Login with empty username ───────────────────────────────────────
    let res = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "",
            "password": "something",
        }))
        .await;

    assert_eq!(
        res.status_code(),
        StatusCode::BAD_REQUEST,
        "empty username should return 400"
    );

    // ── Login with empty password ───────────────────────────────────────
    let res = server
        .post("/api/auth/login")
        .json(&json!({
            "username": admin.email,
            "password": "",
        }))
        .await;

    assert_eq!(
        res.status_code(),
        StatusCode::BAD_REQUEST,
        "empty password should return 400"
    );

    // ── GET /api/auth/me with valid cookie ──────────────────────────────
    let res = with_cookie!(server.get("/api/auth/me"), &cookie).await;

    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "GET /me with valid session should return 200"
    );
    let body: serde_json::Value = res.json();
    assert_eq!(body["role"], "ADMIN", "/me should return role=ADMIN");
    assert_eq!(
        body["id"].as_str().unwrap(),
        admin.id,
        "/me id should match seeded admin"
    );

    // ── GET /api/auth/me without cookie ─────────────────────────────────
    let res = server.get("/api/auth/me").await;

    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "GET /me without cookie should return 401"
    );

    // ── GET /api/auth/me with tampered cookie ───────────────────────────
    let res = with_cookie!(
        server.get("/api/auth/me"),
        "bsds_session=this.is.not.valid"
    )
    .await;

    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "GET /me with tampered cookie should return 401"
    );

    // ── Logout ──────────────────────────────────────────────────────────
    let res = with_cookie!(server.post("/api/auth/logout"), &cookie).await;

    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "logout should return 200"
    );
    let body: serde_json::Value = res.json();
    assert_eq!(body["ok"], true, "logout body should have ok=true");
    let set_cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("logout should set a cookie to clear it");
    assert!(
        set_cookie.contains("Max-Age=0"),
        "logout set-cookie should contain Max-Age=0 to expire the session"
    );

    // ── Change password – without session ───────────────────────────────
    let res = server
        .post("/api/auth/change-password")
        .json(&json!({
            "currentPassword": admin.password_plain,
            "newPassword": "brand-new-pass-123",
        }))
        .await;

    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "change-password without session should return 401"
    );

    // ── Change password – wrong current password ────────────────────────
    let res = with_cookie!(
        server.post("/api/auth/change-password"),
        &cookie
    )
    .json(&json!({
        "currentPassword": "not-the-right-one",
        "newPassword": "brand-new-pass-123",
    }))
    .await;

    assert_eq!(
        res.status_code(),
        StatusCode::BAD_REQUEST,
        "change-password with wrong current should return 400"
    );
    let body: serde_json::Value = res.json();
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("incorrect"),
        "error message should mention 'incorrect'"
    );

    // ── Change password – empty new password ────────────────────────────
    let res = with_cookie!(
        server.post("/api/auth/change-password"),
        &cookie
    )
    .json(&json!({
        "currentPassword": admin.password_plain,
        "newPassword": "",
    }))
    .await;

    assert_eq!(
        res.status_code(),
        StatusCode::BAD_REQUEST,
        "change-password with empty new password should return 400"
    );

    // ── Change password – success ───────────────────────────────────────
    let res = with_cookie!(
        server.post("/api/auth/change-password"),
        &cookie
    )
    .json(&json!({
        "currentPassword": admin.password_plain,
        "newPassword": "brand-new-pass-123",
    }))
    .await;

    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "change-password with correct current should return 200"
    );
    let body: serde_json::Value = res.json();
    assert_eq!(
        body["success"], true,
        "change-password response should have success=true"
    );

    // ── Change password – clears temp password flag in DB ───────────────
    // Insert a separate user with is_temp_password = 1, then change their
    // password and verify the DB flag is cleared.
    use bsds_backend::auth::{hash_password, create_session_token, SessionClaims};

    let temp_user_id = Uuid::new_v4().to_string();
    let temp_email = "temp-user@test.local";
    let temp_plain = "temporary-pass-999";
    let temp_hash = hash_password(temp_plain).expect("hash_password should succeed");

    sqlx::query(
        "INSERT INTO users (id, member_id, name, email, phone, address, password, is_temp_password, role, membership_status, application_fee_paid) \
         VALUES (?1, 'BSDS-2026-8888-00', 'Temp User', ?2, '+910000000000', 'Addr', ?3, 1, 'ADMIN', 'ACTIVE', 1)",
    )
    .bind(&temp_user_id)
    .bind(temp_email)
    .bind(&temp_hash)
    .execute(&pool)
    .await
    .expect("failed to insert temp user");

    let claims = SessionClaims {
        user_id: temp_user_id.clone(),
        username: temp_email.to_string(),
        role: "ADMIN".to_string(),
        member_id: Some("BSDS-2026-8888-00".to_string()),
        must_change_password: true,
    };
    let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
    let token = create_session_token(&claims, &secret);
    let temp_cookie = format!("bsds_session={token}");

    let res = with_cookie!(
        server.post("/api/auth/change-password"),
        &temp_cookie
    )
    .json(&json!({
        "currentPassword": temp_plain,
        "newPassword": "permanent-pass-888",
    }))
    .await;

    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "temp-user change-password should return 200"
    );

    let is_temp: bool =
        sqlx::query_scalar("SELECT is_temp_password FROM users WHERE id = ?1")
            .bind(&temp_user_id)
            .fetch_one(&pool)
            .await
            .expect("failed to query temp user");

    assert!(
        !is_temp,
        "is_temp_password flag should be cleared after password change"
    );
}

// ---------------------------------------------------------------------------
// 2. Unauthenticated and role-based access control
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unauthenticated_and_role_based_access_control() {
    let (server, pool, _dir) = common::test_app().await;

    // ── Unauthenticated requests ────────────────────────────────────────
    // Run these BEFORE seeding so DB-empty edge cases are covered.

    let res = server.get("/api/auth/me").await;
    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "unauthenticated GET /api/auth/me should be 401"
    );

    let res = server.get("/api/members").await;
    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "unauthenticated GET /api/members should be 401"
    );

    let res = server.get("/api/approvals").await;
    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "unauthenticated GET /api/approvals should be 401"
    );

    let res = server.get("/api/transactions").await;
    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "unauthenticated GET /api/transactions should be 401"
    );

    let res = server.post("/api/cron").await;
    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "unauthenticated POST /api/cron should be 401"
    );

    // ── Seed users for role-based tests ─────────────────────────────────
    let admin = common::seed_admin_user(&pool).await;
    let operator = common::seed_operator_user(&pool).await;
    let organiser = common::seed_organiser_user(&pool).await;
    let member = common::seed_member_user(&pool).await;

    let admin_cookie = common::auth_cookie(&admin);
    let operator_cookie = common::auth_cookie(&operator);
    let organiser_cookie = common::auth_cookie(&organiser);
    let member_cookie = common::auth_cookie(&member);

    // Seed a member record for ORGANISER sub-member test
    let seeded_member = common::seed_member(&pool, "Test Member").await;

    // Seed a pending transaction for OPERATOR reject test
    let txn_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source) \
         VALUES (?1, 'CASH_IN', 'OTHER', 100.0, 'CASH', 'Op reject test', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&txn_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .expect("failed to seed pending transaction");

    // Seed an approval + transaction for ADMIN approve test
    let approval_txn_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source) \
         VALUES (?1, 'CASH_IN', 'OTHER', 50.0, 'CASH', 'Admin approval test', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&approval_txn_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .expect("failed to seed approval transaction");

    let approval_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO approvals (id, entity_type, entity_id, action, requested_by_id, status) \
         VALUES (?1, 'TRANSACTION', ?2, 'create_transaction', ?3, 'PENDING')",
    )
    .bind(&approval_id)
    .bind(&approval_txn_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .expect("failed to seed approval");

    let fake_id = Uuid::new_v4().to_string();

    // ── MEMBER role ─────────────────────────────────────────────────────

    let res = with_cookie!(server.get("/api/members"), &member_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "MEMBER GET /api/members should be 403"
    );

    let res = with_cookie!(server.post("/api/members"), &member_cookie)
        .json(&json!({"name": "Nope"}))
        .await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "MEMBER POST /api/members should be 403"
    );

    let res = with_cookie!(server.get("/api/approvals"), &member_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "MEMBER GET /api/approvals should be 403"
    );

    let res = with_cookie!(
        server.post(&format!("/api/approvals/{fake_id}/approve")).json(&json!({})),
        &member_cookie
    )
    .await;
    let status = res.status_code();
    assert!(
        status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN,
        "MEMBER POST /api/approvals/:id/approve should be 401 or 403, got {status}"
    );

    let res = with_cookie!(server.get("/api/audit-log"), &member_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "MEMBER GET /api/audit-log should be 403"
    );

    // ── ORGANISER role ──────────────────────────────────────────────────

    let res = with_cookie!(server.get("/api/members"), &organiser_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "ORGANISER GET /api/members should be 200"
    );

    let res = with_cookie!(server.post("/api/members"), &organiser_cookie)
        .json(&json!({"name": "Should Fail"}))
        .await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "ORGANISER POST /api/members should be 403"
    );

    let res = with_cookie!(server.get("/api/transactions"), &organiser_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "ORGANISER GET /api/transactions should be 200"
    );

    let res = with_cookie!(server.post("/api/transactions"), &organiser_cookie)
        .json(&json!({"memberId": seeded_member.id, "amount": 10}))
        .await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "ORGANISER POST /api/transactions should be 403"
    );

    let res = with_cookie!(
        server.post(&format!("/api/members/{}/sub-members", seeded_member.id)),
        &organiser_cookie
    )
    .json(&json!({"name": "Sub Nope"}))
    .await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "ORGANISER POST /api/members/:id/sub-members should be 403"
    );

    // ── OPERATOR role ───────────────────────────────────────────────────

    let res = with_cookie!(server.post("/api/members"), &operator_cookie)
        .json(&json!({"name": "Operator Created"}))
        .await;
    let status = res.status_code();
    assert!(
        status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
        "OPERATOR POST /api/members should be 200 or 400 (not forbidden), got {status}"
    );

    let res = with_cookie!(server.get("/api/approvals"), &operator_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "OPERATOR GET /api/approvals should be 403"
    );

    let res = with_cookie!(server.post("/api/cron"), &operator_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "OPERATOR POST /api/cron should be 403"
    );

    let res = with_cookie!(
        server.post(&format!("/api/approvals/{fake_id}/approve")).json(&json!({})),
        &operator_cookie
    )
    .await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "OPERATOR POST /api/approvals/:id/approve should be 403"
    );

    let res = with_cookie!(
        server.post(&format!("/api/transactions/{txn_id}/reject")).json(&json!({})),
        &operator_cookie
    )
    .await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "OPERATOR POST /api/transactions/:id/reject should be 403"
    );

    // ── ADMIN role ──────────────────────────────────────────────────────

    let res = with_cookie!(server.get("/api/approvals"), &admin_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "ADMIN GET /api/approvals should be 200"
    );

    let res = with_cookie!(server.post("/api/cron"), &admin_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "ADMIN POST /api/cron should be 200"
    );

    let res = with_cookie!(
        server.post(&format!("/api/approvals/{approval_id}/approve")).json(&json!({})),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code() != StatusCode::FORBIDDEN,
        "ADMIN POST /api/approvals/:id/approve should NOT be 403, got {}",
        res.status_code()
    );
}

// ---------------------------------------------------------------------------
// 3. Cron auth and response
// ---------------------------------------------------------------------------

const TEST_CRON_SECRET: &str = "bsds-test-cron-secret-constant";

fn setup_cron_secret() -> &'static str {
    unsafe {
        std::env::set_var("CRON_SECRET", TEST_CRON_SECRET);
    }
    TEST_CRON_SECRET
}

#[tokio::test]
async fn cron_auth_and_response() {
    let secret = setup_cron_secret();
    let (server, pool, _dir) = common::test_app().await;

    // ── Unauthenticated, no secret header ───────────────────────────────
    let res = server.post("/api/cron").await;
    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "POST /api/cron without auth or secret should be 401"
    );

    // ── Wrong secret header ─────────────────────────────────────────────
    let res = server
        .post("/api/cron")
        .add_header(
            HeaderName::from_static("x-cron-secret"),
            "wrong-secret".parse::<HeaderValue>().unwrap(),
        )
        .await;
    assert_eq!(
        res.status_code(),
        StatusCode::UNAUTHORIZED,
        "POST /api/cron with wrong secret should be 401"
    );

    // ── Correct secret header on empty DB -> all zero counts ────────────
    let res = server
        .post("/api/cron")
        .add_header(
            HeaderName::from_static("x-cron-secret"),
            secret.parse::<HeaderValue>().unwrap(),
        )
        .await;
    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "POST /api/cron with correct secret should be 200"
    );
    let body: serde_json::Value = res.json();
    assert!(
        body.get("processed").is_some(),
        "cron response should have 'processed' field"
    );
    assert!(
        body.get("reminded").is_some(),
        "cron response should have 'reminded' field"
    );
    assert!(
        body.get("expired").is_some(),
        "cron response should have 'expired' field"
    );
    // On an empty DB all counts should be zero
    assert_eq!(
        body["processed"].as_i64().unwrap_or(-1),
        0,
        "empty DB: processed should be 0"
    );
    assert_eq!(
        body["reminded"].as_i64().unwrap_or(-1),
        0,
        "empty DB: reminded should be 0"
    );
    assert_eq!(
        body["expired"].as_i64().unwrap_or(-1),
        0,
        "empty DB: expired should be 0"
    );

    // ── Seed role users for auth checks ─────────────────────────────────
    let admin = common::seed_admin_user(&pool).await;
    let operator = common::seed_operator_user(&pool).await;
    let member = common::seed_member_user(&pool).await;

    let admin_cookie = common::auth_cookie(&admin);
    let operator_cookie = common::auth_cookie(&operator);
    let member_cookie = common::auth_cookie(&member);

    // ── MEMBER -> 403 ───────────────────────────────────────────────────
    let res = with_cookie!(server.post("/api/cron"), &member_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "MEMBER POST /api/cron should be 403"
    );

    // ── OPERATOR -> 403 ─────────────────────────────────────────────────
    let res = with_cookie!(server.post("/api/cron"), &operator_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::FORBIDDEN,
        "OPERATOR POST /api/cron should be 403"
    );

    // ── ADMIN -> 200 with response shape ────────────────────────────────
    let res = with_cookie!(server.post("/api/cron"), &admin_cookie).await;
    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "ADMIN POST /api/cron should be 200"
    );
    let body: serde_json::Value = res.json();
    assert!(
        body.get("processed").is_some(),
        "ADMIN cron response should have 'processed'"
    );
    assert!(
        body.get("reminded").is_some(),
        "ADMIN cron response should have 'reminded'"
    );
    assert!(
        body.get("expired").is_some(),
        "ADMIN cron response should have 'expired'"
    );

    // ── Secret header -> returns summary shape (repeat to confirm) ──────
    let res = server
        .post("/api/cron")
        .add_header(
            HeaderName::from_static("x-cron-secret"),
            secret.parse::<HeaderValue>().unwrap(),
        )
        .await;
    assert_eq!(
        res.status_code(),
        StatusCode::OK,
        "POST /api/cron via secret header should be 200"
    );
    let body: serde_json::Value = res.json();
    assert!(
        body["processed"].is_i64(),
        "cron summary 'processed' should be a number"
    );
    assert!(
        body["reminded"].is_i64(),
        "cron summary 'reminded' should be a number"
    );
    assert!(
        body["expired"].is_i64(),
        "cron summary 'expired' should be a number"
    );
}
