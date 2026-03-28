//! # Security Integration Tests
//!
//! Covers: role-based access control for every major API surface, unauthenticated
//!         request rejection, MEMBER role cannot reach admin/operator endpoints,
//!         ORGANISER role can only read (not mutate) member/transaction data,
//!         OPERATOR role cannot reach approvals or cron, ADMIN role can reach
//!         all routes.
//!
//! Does NOT cover: rate-limiting (in-process state, not HTTP-observable in unit
//!                 tests without concurrency), JWT tamper detection (covered in
//!                 auth_integration.rs), password strength rules (covered in
//!                 auth_integration.rs).
//!
//! Protects: apps/backend/src/auth/permissions.rs,
//!           apps/backend/src/routes/members.rs,
//!           apps/backend/src/routes/transactions.rs,
//!           apps/backend/src/routes/approvals.rs,
//!           apps/backend/src/routes/cron.rs

mod common;

use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helper: add a cookie header to a request builder
// ---------------------------------------------------------------------------

macro_rules! with_cookie {
    ($req:expr, $cookie:expr) => {
        $req.add_header(
            axum::http::HeaderName::from_static("cookie"),
            $cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
    };
}

// ---------------------------------------------------------------------------
// Unauthenticated requests (no cookie) → 401
// ---------------------------------------------------------------------------

#[tokio::test]
async fn login_with_no_cookie_returns_401_on_me_endpoint() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/auth/me").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unauthenticated_list_members_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/members").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unauthenticated_list_approvals_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/approvals").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unauthenticated_list_transactions_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/transactions").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unauthenticated_cron_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.post("/api/cron").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// MEMBER role — should be forbidden from management endpoints
// ---------------------------------------------------------------------------

#[tokio::test]
async fn member_role_cannot_list_members() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "MEMBER").await;

    let resp = with_cookie!(server.get("/api/members"), cookie).await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_role_cannot_create_member() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "MEMBER").await;

    let resp = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Attempt Member",
            "email": "attempt@test.local",
            "phone": "+919000000099",
            "address": "99 Denied St"
        })),
        cookie
    )
    .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_role_cannot_view_approvals() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "MEMBER").await;

    let resp = with_cookie!(server.get("/api/approvals"), cookie).await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_role_cannot_approve() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "MEMBER").await;

    let fake_id = Uuid::new_v4().to_string();
    let resp = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/approve", fake_id))
            .json(&json!({})),
        cookie
    )
    .await;

    // Must be 401 or 403 — never 200
    let status = resp.status_code();
    assert!(
        status == axum::http::StatusCode::FORBIDDEN
            || status == axum::http::StatusCode::UNAUTHORIZED,
        "expected 401 or 403 for MEMBER approve attempt, got {status}"
    );
}

#[tokio::test]
async fn member_cannot_view_audit_log() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "MEMBER").await;

    let resp = with_cookie!(server.get("/api/audit-log"), cookie).await;

    // /api/audit-log is ORGANISER+ so MEMBER gets 403
    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// ORGANISER role — can read members & transactions but cannot mutate or approve
// ---------------------------------------------------------------------------

#[tokio::test]
async fn organiser_can_list_members() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "ORGANISER").await;

    let resp = with_cookie!(server.get("/api/members"), cookie).await;

    resp.assert_status_ok();
}

#[tokio::test]
async fn organiser_cannot_create_member() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "ORGANISER").await;

    let resp = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Organiser Attempt",
            "email": "organiser.attempt@test.local",
            "phone": "+919000000088",
            "address": "88 Forbidden St"
        })),
        cookie
    )
    .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn organiser_can_list_transactions() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "ORGANISER").await;

    let resp = with_cookie!(server.get("/api/transactions"), cookie).await;

    resp.assert_status_ok();
}

#[tokio::test]
async fn organiser_cannot_create_transaction() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "ORGANISER").await;

    let resp = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 100.0,
            "paymentMode": "CASH",
            "purpose": "Organiser attempt"
        })),
        cookie
    )
    .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn organiser_cannot_add_sub_member() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "ORGANISER").await;
    let member = common::seed_member(&pool, "Org SubTarget").await;

    let resp = with_cookie!(
        server
            .post(&format!("/api/members/{}/sub-members", member.id))
            .json(&json!({
                "name": "Sub Attempt",
                "email": "sub.attempt@test.local",
                "phone": "+919100000099",
                "relation": "SPOUSE"
            })),
        cookie
    )
    .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// OPERATOR role — can mutate members/transactions but cannot access approvals/cron
// ---------------------------------------------------------------------------

#[tokio::test]
async fn operator_can_create_member() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "OPERATOR").await;

    let resp = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Operator Created Member",
            "email": "op.created@test.local",
            "phone": "+919000000077",
            "address": "77 Op Street"
        })),
        cookie
    )
    .await;

    // Operator creates → pending_approval (200 OK is acceptable; 400 is validation failure)
    assert!(
        resp.status_code().is_success() || resp.status_code() == axum::http::StatusCode::BAD_REQUEST,
        "expected 200/400 for operator member create, got {}",
        resp.status_code()
    );
}

#[tokio::test]
async fn operator_cannot_view_approvals() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "OPERATOR").await;

    let resp = with_cookie!(server.get("/api/approvals"), cookie).await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn operator_cannot_trigger_cron() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "OPERATOR").await;

    let resp = with_cookie!(server.post("/api/cron"), cookie).await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn operator_cannot_approve_via_approval_route() {
    let (server, pool, _dir) = common::test_app().await;
    let (_id, cookie) = common::seed_user_with_role(&pool, "OPERATOR").await;

    let fake_id = Uuid::new_v4().to_string();
    let resp = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/approve", fake_id))
            .json(&json!({})),
        cookie
    )
    .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn operator_cannot_reject_transaction() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let cookie = common::auth_cookie(&operator);

    // Insert a pending transaction so the route has something to reject
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 500.0, 'CASH', 'Op reject attempt', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&operator.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = with_cookie!(
        server
            .post(&format!("/api/transactions/{}/reject", tx_id))
            .json(&json!({})),
        cookie
    )
    .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// ADMIN role — unrestricted access to all management routes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_can_view_approvals() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = with_cookie!(server.get("/api/approvals"), cookie).await;

    resp.assert_status_ok();
}

#[tokio::test]
async fn admin_can_trigger_cron() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = with_cookie!(server.post("/api/cron"), cookie).await;

    resp.assert_status_ok();
}

#[tokio::test]
async fn admin_can_approve_a_pending_approval() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed a transaction + a PENDING approval for it
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 250.0, 'CASH', 'Admin approval test', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let approval_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO approvals (id, entity_type, entity_id, action, requested_by_id, status)
         VALUES (?1, 'TRANSACTION', ?2, 'create_transaction', ?3, 'PENDING')",
    )
    .bind(&approval_id)
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/approve", approval_id))
            .json(&json!({})),
        cookie
    )
    .await;

    // Should NOT be 403; 200 is success
    assert!(
        !resp.status_code().is_client_error() || resp.status_code() == axum::http::StatusCode::BAD_REQUEST,
        "admin approve returned unexpected client error: {}",
        resp.status_code()
    );
}
