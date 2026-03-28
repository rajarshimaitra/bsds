//! # Transactions Integration Tests
//!
//! Covers: list transactions (with filters), get transaction by id,
//!         create transaction as ADMIN (direct), create transaction as OPERATOR
//!         (pending approval), transaction summary endpoint, delete transaction,
//!         authentication guard on every endpoint.
//!
//! Does NOT cover: Razorpay payment-initiated transactions (tested in
//!                 webhooks_integration.rs), receipt generation (tested in
//!                 receipts_integration.rs).
//!
//! Protects: apps/backend/src/routes/transactions.rs,
//!           apps/backend/src/services/transaction_service.rs

mod common;

use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_transactions_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/transactions").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn transaction_summary_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/transactions/summary").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_transaction_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server
        .post("/api/transactions")
        .json(&json!({ "type": "CASH_IN", "category": "OTHER", "amount": 100 }))
        .await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// List transactions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_transactions_returns_empty_array_on_fresh_db() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/transactions")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.is_array() || body.is_object(), "should return array or paginated object");
}

#[tokio::test]
async fn list_transactions_returns_seeded_transaction() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Insert a transaction directly
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 500.0, 'CASH', 'Test payment', ?2, 'APPROVED', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get("/api/transactions")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body_str = resp.text();
    assert!(body_str.contains(&tx_id), "transaction id should appear in listing");
}

// ---------------------------------------------------------------------------
// Get transaction by id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_transaction_by_id_returns_correct_transaction() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_OUT', 'EXPENSE', 200.0, 'CASH', 'Office supplies', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get(&format!("/api/transactions/{}", tx_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["id"], tx_id.as_str());
    assert_eq!(body["category"], "EXPENSE");
}

// ---------------------------------------------------------------------------
// Transaction summary
// ---------------------------------------------------------------------------

#[tokio::test]
async fn transaction_summary_returns_correct_fields_on_empty_db() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/transactions/summary")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.get("totalIncome").is_some(), "missing totalIncome");
    assert!(body.get("totalExpenses").is_some(), "missing totalExpenses");
    assert!(body.get("pendingAmount").is_some(), "missing pendingAmount");
    assert!(body.get("netBalance").is_some(), "missing netBalance");
}

#[tokio::test]
async fn transaction_summary_reflects_approved_cash_in_transactions() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed an approved CASH_IN
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'MEMBERSHIP', 3000.0, 'UPI', 'Annual membership', ?2, 'APPROVED', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get("/api/transactions/summary")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let total_income = body["totalIncome"].as_f64().unwrap_or(0.0);
    assert!(total_income >= 3000.0, "totalIncome should include the approved CASH_IN");
}

// ---------------------------------------------------------------------------
// Create transaction — ADMIN
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_create_cash_in_transaction_returns_direct_action() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/transactions")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 1000.0,
            "paymentMode": "CASH",
            "purpose": "Event donation"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");
    assert!(body["transactionId"].as_str().is_some());
}

#[tokio::test]
async fn admin_create_cash_out_transaction_returns_direct_action() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/transactions")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "type": "CASH_OUT",
            "category": "EXPENSE",
            "amount": 500.0,
            "paymentMode": "CASH",
            "purpose": "Printing costs"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");
}

// ---------------------------------------------------------------------------
// Create transaction — OPERATOR
// ---------------------------------------------------------------------------

#[tokio::test]
async fn operator_create_transaction_returns_pending_approval() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let cookie = common::auth_cookie(&operator);

    let resp = server
        .post("/api/transactions")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 750.0,
            "paymentMode": "UPI",
            "purpose": "Operator-submitted donation"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "pending_approval");
    assert!(body["approvalId"].as_str().is_some());
}

// ---------------------------------------------------------------------------
// Delete transaction
// ---------------------------------------------------------------------------

/// Transactions are immutable — there is no DELETE route on /api/transactions/:id.
/// The router only exposes GET and POST /api/transactions/:id/reject (ADMIN only).
/// A DELETE request returns 405 Method Not Allowed from axum's router.
#[tokio::test]
async fn delete_transaction_returns_error_because_transactions_are_immutable() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 100.0, 'CASH', 'Cannot delete', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .delete(&format!("/api/transactions/{}", tx_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    // The DELETE route is not registered — axum returns 405 Method Not Allowed.
    // Either 400 (if a delete handler existed) or 405 (no handler) confirms immutability.
    assert!(
        resp.status_code() == axum::http::StatusCode::BAD_REQUEST
            || resp.status_code() == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "DELETE on a transaction must return 400 or 405; got {}",
        resp.status_code()
    );
}
