//! # Sponsors Integration Tests
//!
//! Covers: list sponsors (with pagination, search), create sponsor, get sponsor
//!         (returns linked transactions + links), update sponsor, delete sponsor,
//!         role-gating (MEMBER role rejected with 403), temp-password gate.
//!
//! Does NOT cover: sponsor link generation (tested in sponsor_links_integration.rs),
//!                 webhook-driven sponsor payment processing (tested in webhooks_integration.rs).
//!
//! Protects: apps/backend/src/routes/sponsors.rs,
//!           apps/backend/src/services/sponsor_service.rs

mod common;

use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth / role guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_sponsors_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/sponsors").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_sponsors_as_member_role_returns_403() {
    let (server, pool, _dir) = common::test_app().await;
    let member = common::seed_member_user(&pool).await;
    let cookie = common::auth_cookie(&member);

    let resp = server
        .get("/api/sponsors")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn create_sponsor_as_member_role_returns_403() {
    let (server, pool, _dir) = common::test_app().await;
    let member = common::seed_member_user(&pool).await;
    let cookie = common::auth_cookie(&member);

    let resp = server
        .post("/api/sponsors")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({ "name": "Test", "phone": "+91", "email": "s@test.local" }))
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// Temp-password gate
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_sponsors_with_must_change_password_returns_403() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;

    // Build a session with must_change_password = true
    let claims = bsds_backend::auth::SessionClaims {
        user_id: admin.id.clone(),
        username: admin.email.clone(),
        role: "ADMIN".to_string(),
        member_id: Some(admin.member_id.clone()),
        must_change_password: true,
    };
    let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
    let token = bsds_backend::auth::create_session_token(&claims, &secret);
    let cookie = format!("bsds_session={token}");

    let resp = server
        .get("/api/sponsors")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// List sponsors
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_sponsors_returns_empty_data_on_fresh_db() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/sponsors")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let data = body["data"].as_array().expect("data array missing");
    assert_eq!(data.len(), 0);
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn list_sponsors_returns_created_sponsor() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed a sponsor
    let sp_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sponsors (id, name, phone, email, created_by_id)
         VALUES (?1, 'ACME Corp', '+919000000002', 'acme@corp.test', ?2)",
    )
    .bind(&sp_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get("/api/sponsors")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["total"], 1);
    let data = body["data"].as_array().unwrap();
    assert_eq!(data[0]["name"], "ACME Corp");
}

#[tokio::test]
async fn list_sponsors_search_filters_by_name() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    for (name, email) in [("ACME Corp", "acme@test.local"), ("Beta Ltd", "beta@test.local")] {
        let sp_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO sponsors (id, name, phone, email, created_by_id) VALUES (?1, ?2, '+91', ?3, ?4)")
            .bind(&sp_id)
            .bind(name)
            .bind(email)
            .bind(&admin.id)
            .execute(&pool)
            .await
            .unwrap();
    }

    let resp = server
        .get("/api/sponsors")
        .add_query_params([("search", "acme")])
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["name"], "ACME Corp");
}

// ---------------------------------------------------------------------------
// Create sponsor
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_create_sponsor_returns_sponsor_with_id() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/sponsors")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "name": "New Sponsor",
            "phone": "+919000000003",
            "email": "newsponsor@test.local",
            "company": "New Corp"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body["id"].as_str().is_some(), "sponsor id missing");
    assert_eq!(body["name"], "New Sponsor");
    assert_eq!(body["company"], "New Corp");
}

// ---------------------------------------------------------------------------
// Get sponsor
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_sponsor_returns_sponsor_with_links_and_transactions() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let sp_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sponsors (id, name, phone, email, created_by_id) VALUES (?1, 'Detail Corp', '+91', 'detail@test.local', ?2)")
        .bind(&sp_id)
        .bind(&admin.id)
        .execute(&pool)
        .await
        .unwrap();

    let resp = server
        .get(&format!("/api/sponsors/{}", sp_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["sponsor"]["id"], sp_id.as_str());
    assert!(body["links"].is_array(), "links should be array");
    assert!(body["transactions"].is_array(), "transactions should be array");
}

#[tokio::test]
async fn get_nonexistent_sponsor_returns_404() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/sponsors/00000000-0000-0000-0000-000000000000")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Update sponsor
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_update_sponsor_returns_success_message() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let sp_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sponsors (id, name, phone, email, created_by_id) VALUES (?1, 'Old Name', '+91', 'old@test.local', ?2)")
        .bind(&sp_id)
        .bind(&admin.id)
        .execute(&pool)
        .await
        .unwrap();

    let resp = server
        .put(&format!("/api/sponsors/{}", sp_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({ "name": "New Name" }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body["message"].as_str().is_some());
}

// ---------------------------------------------------------------------------
// Delete sponsor
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_delete_sponsor_with_no_transactions_returns_success() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let sp_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sponsors (id, name, phone, email, created_by_id) VALUES (?1, 'Del Sponsor', '+91', 'del@test.local', ?2)")
        .bind(&sp_id)
        .bind(&admin.id)
        .execute(&pool)
        .await
        .unwrap();

    let resp = server
        .delete(&format!("/api/sponsors/{}", sp_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body["message"].as_str().is_some());
}

#[tokio::test]
async fn admin_delete_sponsor_with_transactions_returns_400() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let sp_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sponsors (id, name, phone, email, created_by_id) VALUES (?1, 'Has Txns', '+91', 'hastxn@test.local', ?2)")
        .bind(&sp_id)
        .bind(&admin.id)
        .execute(&pool)
        .await
        .unwrap();

    // Attach a transaction to this sponsor
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, sponsor_id, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'SPONSORSHIP', 5000.0, 'UPI', 'Sponsor payment', ?2, ?3, 'APPROVED', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&sp_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .delete(&format!("/api/sponsors/{}", sp_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}
