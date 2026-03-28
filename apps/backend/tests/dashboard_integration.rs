//! # Dashboard Integration Tests
//!
//! Covers: stats endpoint returns all required fields, values reflect seeded
//!         data (member counts, pending approvals, financial totals),
//!         authentication guard.
//!
//! Does NOT cover: real-time updates triggered by cron (covered in
//!                 cron_integration.rs).
//!
//! Protects: apps/backend/src/routes/dashboard.rs

mod common;

use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stats_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/dashboard/stats").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Stats response shape
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stats_returns_all_required_fields_on_empty_db() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/dashboard/stats")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.get("totalMembers").is_some(), "totalMembers field missing");
    assert!(body.get("activeMembers").is_some(), "activeMembers field missing");
    assert!(body.get("pendingApprovals").is_some(), "pendingApprovals field missing");
    assert!(body.get("totalCashIn").is_some(), "totalCashIn field missing");
    assert!(body.get("totalCashOut").is_some(), "totalCashOut field missing");
    assert!(body.get("netBalance").is_some(), "netBalance field missing");
}

// ---------------------------------------------------------------------------
// Stats values reflect seeded data
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stats_total_members_counts_only_member_role_users() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let _member = common::seed_member_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/dashboard/stats")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    // admin user should NOT be counted, only MEMBER role users
    let total_members = body["totalMembers"].as_i64().unwrap_or(-1);
    assert_eq!(
        total_members, 1,
        "totalMembers should be 1 (only the MEMBER-role user)"
    );
}

#[tokio::test]
async fn stats_active_members_counts_only_active_status() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // The seeded member_user has membership_status = ACTIVE (seed_user default)
    let resp = server
        .get("/api/dashboard/stats")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let active = body["activeMembers"].as_i64().unwrap_or(-1);
    // member has ACTIVE status
    assert!(active >= 1, "activeMembers should be at least 1 (seeded member has ACTIVE status)");
    let _ = member;
}

#[tokio::test]
async fn stats_pending_approvals_increments_after_operator_creates_member() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie_admin = common::auth_cookie(&admin);
    let operator = common::seed_operator_user(&pool).await;
    let cookie_op = common::auth_cookie(&operator);

    // Create a member as operator (creates a pending approval)
    let _ = server
        .post("/api/members")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie_op.parse().unwrap())
        .json(&serde_json::json!({
            "name": "Pending Person",
            "email": "pending.stats@test.local",
            "phone": "+919000000099",
            "address": "Stats Road"
        }))
        .await;

    let resp = server
        .get("/api/dashboard/stats")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie_admin.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let pending = body["pendingApprovals"].as_i64().unwrap_or(0);
    assert!(pending >= 1, "pendingApprovals should reflect the operator-created member approval");
}

#[tokio::test]
async fn stats_net_balance_reflects_approved_cash_in_and_cash_out() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed approved CASH_IN 5000 and CASH_OUT 2000
    for (tx_type, amount) in [("CASH_IN", 5000.0f64), ("CASH_OUT", 2000.0f64)] {
        let tx_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
             VALUES (?1, ?2, 'OTHER', ?3, 'CASH', 'Balance test', ?4, 'APPROVED', 'MANUAL')",
        )
        .bind(&tx_id)
        .bind(tx_type)
        .bind(amount)
        .bind(&admin.id)
        .execute(&pool)
        .await
        .unwrap();
    }

    let resp = server
        .get("/api/dashboard/stats")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let cash_in = body["totalCashIn"].as_f64().unwrap_or(0.0);
    let cash_out = body["totalCashOut"].as_f64().unwrap_or(0.0);
    let net = body["netBalance"].as_f64().unwrap_or(0.0);

    assert!(cash_in >= 5000.0, "totalCashIn should be >= 5000");
    assert!(cash_out >= 2000.0, "totalCashOut should be >= 2000");
    assert!(
        (net - (cash_in - cash_out)).abs() < 0.01,
        "netBalance should equal totalCashIn - totalCashOut"
    );
}
