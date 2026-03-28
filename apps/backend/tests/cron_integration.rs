//! # Cron Integration Tests
//!
//! Covers: cron endpoint auth (session-based ADMIN check, secret-header bypass),
//!         non-ADMIN session returns 403, cron endpoint returns MembershipCronSummary
//!         shape with processed/reminded/expired fields.
//!
//! Does NOT cover: actual scheduler execution timing (tokio-cron-scheduler is not
//!                 exercised in tests), WhatsApp notification dispatch side-effects
//!                 (require live WhatsApp client).
//!
//! Protects: apps/backend/src/routes/cron.rs,
//!           apps/backend/src/services/membership_service::run_daily_membership_cron

mod common;

/// Single constant cron secret used by all tests that need the secret-header
/// bypass.  All tests set the same value so concurrent set_var calls from
/// different threads don't cause mismatches.
const TEST_CRON_SECRET: &str = "bsds-test-cron-secret-constant";

fn setup_cron_secret() -> &'static str {
    std::env::set_var("CRON_SECRET", TEST_CRON_SECRET);
    TEST_CRON_SECRET
}

// ---------------------------------------------------------------------------
// Auth guard variants
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cron_without_auth_and_without_secret_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.post("/api/cron").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cron_as_member_role_returns_403() {
    let (server, pool, _dir) = common::test_app().await;
    let member = common::seed_member_user(&pool).await;
    let cookie = common::auth_cookie(&member);

    let resp = server
        .post("/api/cron")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cron_as_operator_role_returns_403() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let cookie = common::auth_cookie(&operator);

    let resp = server
        .post("/api/cron")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cron_via_secret_header_bypasses_session_auth() {
    let (server, _pool, _dir) = common::test_app().await;
    let secret = setup_cron_secret();

    let resp = server
        .post("/api/cron")
        .add_header(
            axum::http::HeaderName::from_static("x-cron-secret"),
            secret.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;

    resp.assert_status_ok();
}

#[tokio::test]
async fn cron_via_wrong_secret_returns_401_fallback() {
    let (server, _pool, _dir) = common::test_app().await;
    setup_cron_secret();

    let resp = server
        .post("/api/cron")
        .add_header(
            axum::http::HeaderName::from_static("x-cron-secret"),
            "definitely-wrong-secret".parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;

    // Wrong secret + no session → 401
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Cron response shape
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cron_as_admin_returns_summary_with_required_fields() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/cron")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.get("processed").is_some(), "processed field missing");
    assert!(body.get("reminded").is_some(), "reminded field missing");
    assert!(body.get("expired").is_some(), "expired field missing");
}

#[tokio::test]
async fn cron_on_empty_db_returns_all_zero_counts() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/cron")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["processed"], 0, "no members to process in fresh DB");
    assert_eq!(body["reminded"], 0, "no reminders in fresh DB");
    assert_eq!(body["expired"], 0, "no expirations in fresh DB");
}

#[tokio::test]
async fn cron_via_secret_header_returns_summary_shape() {
    let (server, _pool, _dir) = common::test_app().await;
    let secret = setup_cron_secret();

    let resp = server
        .post("/api/cron")
        .add_header(
            axum::http::HeaderName::from_static("x-cron-secret"),
            secret.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.get("processed").is_some());
    assert!(body.get("reminded").is_some());
    assert!(body.get("expired").is_some());
}
