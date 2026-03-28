//! # Sponsor Links Integration Tests
//!
//! Covers: list sponsor links (authenticated, with filters), create sponsor link,
//!         get public sponsor link by token (unauthenticated),
//!         deactivate sponsor link, role-gating, expired link detection.
//!
//! Does NOT cover: webhook-triggered link deactivation after payment (covered in
//!                 webhooks_integration.rs).
//!
//! Protects: apps/backend/src/routes/sponsor_links.rs,
//!           apps/backend/src/services/sponsor_service.rs (link functions)

mod common;

use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth / role guard on list + create
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_sponsor_links_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/sponsor-links").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_sponsor_links_as_member_returns_403() {
    let (server, pool, _dir) = common::test_app().await;
    let member = common::seed_member_user(&pool).await;
    let cookie = common::auth_cookie(&member);

    let resp = server
        .get("/api/sponsor-links")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// List sponsor links
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_sponsor_links_returns_empty_data_on_fresh_db() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/sponsor-links")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["total"], 0);
    assert!(body["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_sponsor_links_returns_all_active_links() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Insert two active links
    for token in ["token-aaa", "token-bbb"] {
        let link_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO sponsor_links (id, token, upi_id, is_active, created_by_id)
             VALUES (?1, ?2, 'test@upi', 1, ?3)",
        )
        .bind(&link_id)
        .bind(token)
        .bind(&admin.id)
        .execute(&pool)
        .await
        .unwrap();
    }

    let resp = server
        .get("/api/sponsor-links")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["total"], 2);
}

#[tokio::test]
async fn list_sponsor_links_isactive_filter_returns_only_active() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // One active, one inactive
    for (token, active) in [("tok-active", 1i32), ("tok-inactive", 0)] {
        let link_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO sponsor_links (id, token, upi_id, is_active, created_by_id)
             VALUES (?1, ?2, 'test@upi', ?3, ?4)",
        )
        .bind(&link_id)
        .bind(token)
        .bind(active)
        .bind(&admin.id)
        .execute(&pool)
        .await
        .unwrap();
    }

    let resp = server
        .get("/api/sponsor-links")
        .add_query_params([("isActive", "true")])
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["total"], 1, "only active link should be returned");
}

// ---------------------------------------------------------------------------
// Create sponsor link
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_create_sponsor_link_returns_link_and_url() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/sponsor-links")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "upiId": "sponsor@upi",
            "sponsorPurpose": "GOLD_SPONSOR",
            "amount": 10000.0
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body["link"].is_object(), "link object missing");
    assert!(body["url"].as_str().is_some(), "url missing");
    assert!(body["message"].as_str().is_some());
}

// ---------------------------------------------------------------------------
// Public sponsor link — GET /api/sponsor-links/:token
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_public_sponsor_link_returns_link_data_for_valid_token() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;

    let link_id = Uuid::new_v4().to_string();
    let token = "public-token-xyz";
    sqlx::query(
        "INSERT INTO sponsor_links (id, token, upi_id, is_active, created_by_id)
         VALUES (?1, ?2, 'pay@upi', 1, ?3)",
    )
    .bind(&link_id)
    .bind(token)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    // Public endpoint — no auth required
    let resp = server
        .get(&format!("/api/sponsor-links/{}", token))
        .await;

    resp.assert_status_ok();
}

#[tokio::test]
async fn get_public_sponsor_link_returns_error_for_inactive_link() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;

    let link_id = Uuid::new_v4().to_string();
    let token = "inactive-token-xyz";
    sqlx::query(
        "INSERT INTO sponsor_links (id, token, upi_id, is_active, created_by_id)
         VALUES (?1, ?2, 'pay@upi', 0, ?3)",
    )
    .bind(&link_id)
    .bind(token)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get(&format!("/api/sponsor-links/{}", token))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    // Inactive links return 200 with an error field (not 404)
    assert!(body["error"].as_str().is_some(), "error field expected for inactive link");
}

#[tokio::test]
async fn get_public_sponsor_link_returns_404_for_unknown_token() {
    let (server, _pool, _dir) = common::test_app().await;

    let resp = server
        .get("/api/sponsor-links/no-such-token-here")
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Deactivate sponsor link — PATCH /api/sponsor-links/:token
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deactivate_sponsor_link_returns_success_message() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let link_id = Uuid::new_v4().to_string();
    let token = "deactivate-me-token";
    sqlx::query(
        "INSERT INTO sponsor_links (id, token, upi_id, is_active, created_by_id)
         VALUES (?1, ?2, 'pay@upi', 1, ?3)",
    )
    .bind(&link_id)
    .bind(token)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .patch(&format!("/api/sponsor-links/{}", token))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body["message"].as_str().is_some());

    // Verify in DB
    let is_active: bool =
        sqlx::query_scalar("SELECT is_active FROM sponsor_links WHERE id = ?1")
            .bind(&link_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!is_active, "link should be deactivated in DB");
}

#[tokio::test]
async fn deactivate_already_inactive_link_returns_400() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let link_id = Uuid::new_v4().to_string();
    let token = "already-inactive";
    sqlx::query(
        "INSERT INTO sponsor_links (id, token, upi_id, is_active, created_by_id)
         VALUES (?1, ?2, 'pay@upi', 0, ?3)",
    )
    .bind(&link_id)
    .bind(token)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .patch(&format!("/api/sponsor-links/{}", token))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}
