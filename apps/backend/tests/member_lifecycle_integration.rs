//! # Member Lifecycle Integration Tests
//!
//! Covers: the full member state machine from submission through approval or
//!         rejection. Tests the OPERATOR → pending_approval path and ADMIN
//!         → direct path. Verifies that admin approval of a MEMBER_ADD creates
//!         a linked user record, that rejection leaves no user record, that
//!         activity logs are written, that OPERATOR updates queue an approval
//!         rather than applying directly, and that ADMIN deletes suspend
//!         immediately while OPERATOR deletes queue an approval.
//!
//! Does NOT cover: payment flow after approval (PENDING_PAYMENT → ACTIVE),
//!                 sub-member lifecycle (covered in members_integration.rs),
//!                 approval side-effects for TRANSACTION type (covered in
//!                 transaction_membership_link_integration.rs).
//!
//! Protects: apps/backend/src/services/member_service.rs,
//!           apps/backend/src/services/approval_service.rs,
//!           apps/backend/src/routes/members.rs,
//!           apps/backend/src/routes/approvals.rs

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
// Operator submission path
// ---------------------------------------------------------------------------

#[tokio::test]
async fn new_operator_created_member_starts_as_pending_approval_in_approval_queue() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let admin = common::seed_admin_user(&pool).await;
    let op_cookie = common::auth_cookie(&operator);
    let admin_cookie = common::auth_cookie(&admin);

    // Operator creates a member
    let resp = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Lifecycle Member",
            "email": "lifecycle.member@test.local",
            "phone": "+919200000001",
            "address": "1 Lifecycle Lane"
        })),
        op_cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "pending_approval");
    let approval_id = body["approvalId"].as_str().expect("approvalId must be set").to_string();

    // Approval must now appear in the pending list for admin
    let list_resp = with_cookie!(server.get("/api/approvals"), admin_cookie).await;
    list_resp.assert_status_ok();
    let list_text = list_resp.text();
    assert!(list_text.contains(&approval_id), "approval must appear in pending list");
}

#[tokio::test]
async fn admin_approving_member_add_creates_user_and_member_records() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let admin = common::seed_admin_user(&pool).await;
    let op_cookie = common::auth_cookie(&operator);
    let admin_cookie = common::auth_cookie(&admin);

    // Step 1: Operator submits member
    let create_resp = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Approval Candidate",
            "email": "approval.candidate@test.local",
            "phone": "+919200000002",
            "address": "2 Approval Ave"
        })),
        op_cookie
    )
    .await;

    let body = create_resp.json::<serde_json::Value>();
    let approval_id = body["approvalId"].as_str().unwrap().to_string();

    // Step 2: Admin approves
    let approve_resp = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/approve", approval_id))
            .json(&json!({})),
        admin_cookie
    )
    .await;

    approve_resp.assert_status_ok();
    let approve_body = approve_resp.json::<serde_json::Value>();
    assert_eq!(approve_body["ok"], true);

    // Step 3: Approval record must now be APPROVED in the DB
    let status: String = sqlx::query_scalar("SELECT status FROM approvals WHERE id = ?1")
        .bind(&approval_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "APPROVED", "approval must be APPROVED after admin action");

    // Step 4: A user record must have been created (email matches)
    let user_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = 'approval.candidate@test.local'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(user_count, 1, "user record must be created when MEMBER_ADD is approved");
}

#[tokio::test]
async fn admin_rejecting_member_add_leaves_no_user_in_db() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let admin = common::seed_admin_user(&pool).await;
    let op_cookie = common::auth_cookie(&operator);
    let admin_cookie = common::auth_cookie(&admin);

    // Operator submits member
    let create_resp = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Rejected Candidate",
            "email": "rejected.candidate@test.local",
            "phone": "+919200000003",
            "address": "3 Rejection Rd"
        })),
        op_cookie
    )
    .await;

    let body = create_resp.json::<serde_json::Value>();
    let approval_id = body["approvalId"].as_str().unwrap().to_string();

    // Admin rejects
    let reject_resp = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/reject", approval_id))
            .json(&json!({ "notes": "Not eligible" })),
        admin_cookie
    )
    .await;

    reject_resp.assert_status_ok();

    // No user record must exist for this email
    let user_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = 'rejected.candidate@test.local'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        user_count, 0,
        "no user record must exist after MEMBER_ADD rejection"
    );
}

#[tokio::test]
async fn member_add_approval_activity_log_recorded() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let op_cookie = common::auth_cookie(&operator);

    // Operator submits member — service writes an activity log entry
    let _resp = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Activity Log Member",
            "email": "activity.log.member@test.local",
            "phone": "+919200000004",
            "address": "4 Activity Blvd"
        })),
        op_cookie
    )
    .await;

    // At least one activity log entry must exist for the operator's action
    let log_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM activity_logs WHERE user_id = ?1")
            .bind(&operator.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(log_count >= 1, "activity log must record the operator submission");
}

// ---------------------------------------------------------------------------
// Operator update → pending approval
// ---------------------------------------------------------------------------

#[tokio::test]
async fn operator_update_member_queues_approval_not_direct() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let member = common::seed_member(&pool, "Update Target Member").await;
    let op_cookie = common::auth_cookie(&operator);

    let resp = with_cookie!(
        server
            .patch(&format!("/api/members/{}", member.id))
            .json(&json!({ "name": "Update Target Updated" })),
        op_cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(
        body["action"], "pending_approval",
        "OPERATOR update must queue an approval, not apply directly"
    );
    assert!(
        body["approvalId"].as_str().is_some(),
        "approvalId must be set for OPERATOR update"
    );
}

// ---------------------------------------------------------------------------
// Admin update → direct
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_update_member_is_direct_no_approval() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Admin Direct Update Target").await;
    let admin_cookie = common::auth_cookie(&admin);

    let resp = with_cookie!(
        server
            .patch(&format!("/api/members/{}", member.id))
            .json(&json!({ "name": "Admin Direct Updated" })),
        admin_cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(
        body["action"], "direct",
        "ADMIN update must apply directly without queuing an approval"
    );
    assert!(
        body.get("approvalId").map_or(true, |v| v.is_null()),
        "approvalId must be null for ADMIN direct update"
    );
}

// ---------------------------------------------------------------------------
// Operator delete → queues approval
// ---------------------------------------------------------------------------

#[tokio::test]
async fn operator_delete_member_queues_approval() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let member = common::seed_member(&pool, "Delete Queued Member").await;
    let op_cookie = common::auth_cookie(&operator);

    let resp = with_cookie!(
        server.delete(&format!("/api/members/{}", member.id)),
        op_cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    // The delete endpoint always returns { "ok": true }; we verify the
    // approval was queued by checking the approvals table in the DB.
    assert_eq!(body["ok"], true);

    // A MEMBER_DELETE approval must have been queued for this member
    let approval_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM approvals WHERE entity_id = ?1 AND entity_type = 'MEMBER_DELETE' AND status = 'PENDING'",
    )
    .bind(&member.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        approval_count >= 1,
        "a MEMBER_DELETE approval must be queued when OPERATOR deletes a member"
    );

    // User must NOT be suspended yet — the approval is only queued
    let status: String =
        sqlx::query_scalar("SELECT membership_status FROM users WHERE id = ?1")
            .bind(&member.user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(
        status, "SUSPENDED",
        "user must not be suspended until admin approves the delete"
    );
}

// ---------------------------------------------------------------------------
// Admin delete → suspends immediately
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_delete_member_suspends_directly() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Admin Delete Target").await;
    let admin_cookie = common::auth_cookie(&admin);

    let resp = with_cookie!(
        server.delete(&format!("/api/members/{}", member.id)),
        admin_cookie
    )
    .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["ok"], true);

    // User must be SUSPENDED immediately
    let status: String =
        sqlx::query_scalar("SELECT membership_status FROM users WHERE id = ?1")
            .bind(&member.user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "SUSPENDED", "user must be suspended immediately by admin delete");
}

// ---------------------------------------------------------------------------
// Approval notes are persisted
// ---------------------------------------------------------------------------

#[tokio::test]
async fn approval_notes_are_saved_in_db_on_rejection() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let admin_cookie = common::auth_cookie(&admin);

    // Seed a PENDING approval directly
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 100.0, 'CASH', 'Notes test', ?2, 'PENDING', 'MANUAL')",
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

    let _resp = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/reject", approval_id))
            .json(&json!({ "notes": "Not a valid expense" })),
        admin_cookie
    )
    .await;

    let notes: Option<String> =
        sqlx::query_scalar("SELECT notes FROM approvals WHERE id = ?1")
            .bind(&approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        notes.as_deref(),
        Some("Not a valid expense"),
        "rejection notes must be persisted to the approvals row"
    );
}
