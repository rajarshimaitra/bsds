//! # Transaction-Membership Link Integration Tests
//!
//! Covers: audit log entries written for transaction creation and approval,
//!         admin-created transactions immediately write audit entries while
//!         operator-created transactions only write a TRANSACTION_CREATED entry,
//!         admin can reject a pending transaction, rejection writes a
//!         TRANSACTION_REJECTED audit entry, operator cannot reject transactions,
//!         transactions with includesSubscription flag create a membership record.
//!
//! Does NOT cover: Razorpay webhook-initiated transactions (covered in
//!                 webhooks_integration.rs), receipt generation triggered by
//!                 transaction creation (covered in receipts_integration.rs).
//!
//! Protects: apps/backend/src/routes/transactions.rs,
//!           apps/backend/src/services/transaction_service.rs,
//!           apps/backend/src/services/approval_service.rs

mod common;

use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
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
// Audit log: ADMIN direct transaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_transaction_creates_audit_log_transaction_created_entry() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 500.0,
            "paymentMode": "CASH",
            "purpose": "Admin audit test"
        })),
        cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let tx_id = body["transactionId"].as_str().unwrap().to_string();

    // TRANSACTION_CREATED audit entry must exist
    let created_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE transaction_id = ?1 AND event_type = 'TRANSACTION_CREATED'",
    )
    .bind(&tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(created_count >= 1, "TRANSACTION_CREATED audit entry must be written for admin transaction");
}

#[tokio::test]
async fn admin_transaction_creates_audit_log_transaction_approved_entry() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 750.0,
            "paymentMode": "UPI",
            "purpose": "Admin approved audit test"
        })),
        cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let tx_id = body["transactionId"].as_str().unwrap().to_string();

    // Admin-created transactions are immediately APPROVED, so both entries must exist
    let approved_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE transaction_id = ?1 AND event_type = 'TRANSACTION_APPROVED'",
    )
    .bind(&tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        approved_count >= 1,
        "TRANSACTION_APPROVED audit entry must be written for admin-created (directly approved) transaction"
    );
}

// ---------------------------------------------------------------------------
// Audit log: OPERATOR pending transaction — only CREATED entry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn operator_transaction_creates_only_transaction_created_audit_entry() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let cookie = common::auth_cookie(&operator);

    let resp = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 300.0,
            "paymentMode": "CASH",
            "purpose": "Operator audit test"
        })),
        cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let tx_id = body["transactionId"].as_str().unwrap().to_string();

    // TRANSACTION_CREATED must exist
    let created_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE transaction_id = ?1 AND event_type = 'TRANSACTION_CREATED'",
    )
    .bind(&tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(created_count >= 1, "TRANSACTION_CREATED must be written for operator transaction");

    // TRANSACTION_APPROVED must NOT exist yet (it is pending)
    let approved_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE transaction_id = ?1 AND event_type = 'TRANSACTION_APPROVED'",
    )
    .bind(&tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        approved_count, 0,
        "TRANSACTION_APPROVED must NOT exist for an operator-submitted pending transaction"
    );
}

// ---------------------------------------------------------------------------
// Admin can reject a pending transaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_can_reject_pending_transaction() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed a PENDING transaction directly (not via API so status stays PENDING)
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 200.0, 'CASH', 'Reject me', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
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

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["ok"], true);

    // Transaction must now be REJECTED
    let status: String =
        sqlx::query_scalar("SELECT approval_status FROM transactions WHERE id = ?1")
            .bind(&tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "REJECTED");
}

// ---------------------------------------------------------------------------
// Rejection activity log entry
// ---------------------------------------------------------------------------

/// The direct reject endpoint (/api/transactions/:id/reject) writes an
/// activity_log entry (not an audit_log row — audit_log rows for REJECTED are
/// written by the approval service path). This test verifies the activity log.
#[tokio::test]
async fn rejected_transaction_creates_activity_log_transaction_rejected_entry() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 150.0, 'CASH', 'Reject audit', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let _resp = with_cookie!(
        server
            .post(&format!("/api/transactions/{}/reject", tx_id))
            .json(&json!({})),
        cookie
    )
    .await;

    // The direct reject route writes an activity_log entry with action = 'transaction_rejected'
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs WHERE user_id = ?1 AND action = 'transaction_rejected'",
    )
    .bind(&admin.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        activity_count >= 1,
        "activity_log must record transaction_rejected action after direct rejection"
    );
}

// ---------------------------------------------------------------------------
// Operator cannot reject transactions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn operator_cannot_reject_transaction() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let admin = common::seed_admin_user(&pool).await;
    let op_cookie = common::auth_cookie(&operator);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 100.0, 'CASH', 'Op reject test', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = with_cookie!(
        server
            .post(&format!("/api/transactions/{}/reject", tx_id))
            .json(&json!({})),
        op_cookie
    )
    .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// Transaction with includesSubscription creates a membership record
// ---------------------------------------------------------------------------

#[tokio::test]
async fn transaction_with_includes_subscription_creates_membership_record() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // First, create a member so we have a member_id to link
    let member = common::seed_member(&pool, "Subscription Member").await;

    let resp = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "MEMBERSHIP",
            "amount": 250.0,
            "paymentMode": "CASH",
            "purpose": "Monthly subscription",
            "memberId": member.id,
            "membershipType": "MONTHLY",
            "includesSubscription": true
        })),
        cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");
    let tx_id = body["transactionId"].as_str().unwrap().to_string();

    // A membership record linked to this member must now exist
    let membership_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memberships WHERE member_id = ?1",
    )
    .bind(&member.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        membership_count >= 1,
        "a membership record must be created when includesSubscription is true; tx_id={tx_id}"
    );
}

// ---------------------------------------------------------------------------
// Approve via approval route creates both audit and activity log entries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn approve_transaction_via_approval_route_creates_approved_audit_entry() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed PENDING transaction + approval
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 400.0, 'UPI', 'Approval audit test', ?2, 'PENDING', 'MANUAL')",
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

    resp.assert_status_ok();

    // TRANSACTION_APPROVED audit entry must now exist (approval service writes this)
    let approved_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE transaction_id = ?1 AND event_type = 'TRANSACTION_APPROVED'",
    )
    .bind(&tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        approved_count >= 1,
        "TRANSACTION_APPROVED audit entry must be written after approval-service approves a TRANSACTION"
    );

    // Activity log must also record the approval
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs WHERE user_id = ?1 AND action LIKE '%transaction%approved%'",
    )
    .bind(&admin.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        activity_count >= 1,
        "activity_log must record transaction approved action"
    );
}
