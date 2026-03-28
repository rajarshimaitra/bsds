//! # Memberships Integration Tests
//!
//! Covers: list memberships by member_id, get membership by id,
//!         create membership as ADMIN (direct), create membership as OPERATOR
//!         (pending approval), approve membership, reject membership,
//!         authentication guard on all routes.
//!
//! Does NOT cover: automatic expiry transitions triggered by cron (tested in
//!                 cron_integration.rs), Razorpay-driven membership creation
//!                 (tested in webhooks_integration.rs).
//!
//! Protects: apps/backend/src/routes/memberships.rs,
//!           apps/backend/src/services/membership_service.rs

mod common;

use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_memberships_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/memberships").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_membership_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server
        .post("/api/memberships")
        .json(&json!({ "memberId": "x", "type": "MONTHLY", "amount": 250 }))
        .await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// List memberships
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_memberships_without_member_id_param_returns_empty_array() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/memberships")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    // The list endpoint now returns a paginated object; check that no data is present
    let is_empty = body == json!([])
        || body.get("data").and_then(|d| d.as_array()).map_or(false, |a| a.is_empty())
        || body.as_array().map_or(false, |a| a.is_empty());
    assert!(is_empty, "expected empty memberships on fresh DB, got: {}", body);
}

#[tokio::test]
async fn list_memberships_by_member_id_returns_existing_memberships() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Kate Membership").await;
    let cookie = common::auth_cookie(&admin);

    // Insert a membership directly
    let mem_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO memberships (id, member_id, type, fee_type, amount, start_date, end_date, is_application_fee, status)
         VALUES (?1, ?2, 'MONTHLY', 'SUBSCRIPTION', 250.0, '2026-01-01', '2026-01-31', 0, 'APPROVED')",
    )
    .bind(&mem_id)
    .bind(&member.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get("/api/memberships")
        .add_query_params([("member_id", member.id.as_str())])
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let arr = body.as_array().expect("should be array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], mem_id.as_str());
}

// ---------------------------------------------------------------------------
// Get single membership
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_membership_by_id_returns_correct_membership() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Leo GetMembership").await;
    let cookie = common::auth_cookie(&admin);

    let mem_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO memberships (id, member_id, type, fee_type, amount, start_date, end_date, is_application_fee, status)
         VALUES (?1, ?2, 'ANNUAL', 'SUBSCRIPTION', 3000.0, '2026-01-01', '2026-12-31', 0, 'PENDING')",
    )
    .bind(&mem_id)
    .bind(&member.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get(&format!("/api/memberships/{}", mem_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["id"], mem_id.as_str());
    assert_eq!(body["type"], "ANNUAL");
}

// ---------------------------------------------------------------------------
// Create membership — ADMIN
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_create_monthly_membership_returns_direct_action() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Mia Monthly").await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/memberships")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "memberId": member.id,
            "type": "MONTHLY",
            "amount": 250.0,
            "feeType": "SUBSCRIPTION",
            "isApplicationFee": false
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");
}

#[tokio::test]
async fn admin_create_membership_with_wrong_amount_returns_400() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Nina WrongAmt").await;
    let cookie = common::auth_cookie(&admin);

    // MONTHLY is 250, sending 999 should fail validation
    let resp = server
        .post("/api/memberships")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "memberId": member.id,
            "type": "MONTHLY",
            "amount": 999.0,
            "feeType": "SUBSCRIPTION",
            "isApplicationFee": false
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// Create membership — OPERATOR
// ---------------------------------------------------------------------------

#[tokio::test]
async fn operator_create_membership_returns_pending_approval() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let member = common::seed_member(&pool, "Oscar Operator").await;
    let cookie = common::auth_cookie(&operator);

    let resp = server
        .post("/api/memberships")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "memberId": member.id,
            "type": "MONTHLY",
            "amount": 250.0,
            "feeType": "SUBSCRIPTION"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "pending_approval");
    assert!(body["approvalId"].as_str().is_some());
}

// ---------------------------------------------------------------------------
// Approve / reject membership
// ---------------------------------------------------------------------------

#[tokio::test]
async fn patch_membership_approve_returns_ok() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Penny Approve").await;
    let cookie = common::auth_cookie(&admin);

    let mem_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO memberships (id, member_id, type, fee_type, amount, start_date, end_date, is_application_fee, status)
         VALUES (?1, ?2, 'MONTHLY', 'SUBSCRIPTION', 250.0, '2026-01-01', '2026-01-31', 0, 'PENDING')",
    )
    .bind(&mem_id)
    .bind(&member.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .patch(&format!("/api/memberships/{}", mem_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({ "action": "approve" }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn patch_membership_reject_returns_ok() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Quinn Reject").await;
    let cookie = common::auth_cookie(&admin);

    let mem_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO memberships (id, member_id, type, fee_type, amount, start_date, end_date, is_application_fee, status)
         VALUES (?1, ?2, 'MONTHLY', 'SUBSCRIPTION', 250.0, '2026-01-01', '2026-01-31', 0, 'PENDING')",
    )
    .bind(&mem_id)
    .bind(&member.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .patch(&format!("/api/memberships/{}", mem_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({ "action": "reject", "notes": "Test rejection" }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["ok"], true);
}
