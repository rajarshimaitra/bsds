//! Consolidated integration tests for BSDS Dashboard business logic.
//!
//! This file merges tests from 7 individual integration test files into 5 large
//! test functions that each share a single `test_app()` instance. This reduces
//! setup overhead and makes the test suite faster while still covering the full
//! surface area.
//!
//! Consolidated from:
//!   - members_integration.rs (22 tests)
//!   - memberships_integration.rs (10 tests)
//!   - transactions_integration.rs (12 tests)
//!   - approvals_integration.rs (15 tests)
//!   - member_lifecycle_integration.rs (9 tests)
//!   - transaction_membership_link_integration.rs (8 tests)
//!   - dashboard_integration.rs (6 tests)
//!
//! Test functions:
//!   1. member_and_submember_crud
//!   2. approval_workflow
//!   3. transaction_and_membership_lifecycle
//!   4. member_lifecycle_state_machine
//!   5. dashboard_stats_aggregation

mod common;

use serde_json::{json, Value};

/// Convenience macro for adding a cookie header to a test request.
macro_rules! with_cookie {
    ($req:expr, $cookie:expr) => {
        $req.add_header(
            axum::http::HeaderName::from_static("cookie"),
            $cookie
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
    };
}

// ---------------------------------------------------------------------------
// 1. Member and sub-member CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn member_and_submember_crud() {
    let (server, pool, _tmp) = common::test_app().await;

    // ── Seed users ──────────────────────────────────────────────────────
    let admin = common::seed_admin_user(&pool).await;
    let operator = common::seed_operator_user(&pool).await;
    let organiser = common::seed_organiser_user(&pool).await;
    let member_user = common::seed_member_user(&pool).await;

    let admin_cookie = common::auth_cookie(&admin);
    let operator_cookie = common::auth_cookie(&operator);
    let organiser_cookie = common::auth_cookie(&organiser);
    let member_cookie = common::auth_cookie(&member_user);

    // ── Auth guards: no cookie -> 401 ───────────────────────────────────
    let res = server.get("/api/members").await;
    assert!(
        res.status_code().is_client_error(),
        "GET /api/members without auth should return 401"
    );

    let res = server
        .get("/api/members/00000000-0000-0000-0000-000000000000")
        .await;
    assert!(
        res.status_code().is_client_error(),
        "GET /api/members/:id without auth should return 401"
    );

    let res = server
        .post("/api/members")
        .json(&json!({
            "name": "Unauth User",
            "email": "unauth@example.com"
        }))
        .await;
    assert!(
        res.status_code().is_client_error(),
        "POST /api/members without auth should return 401"
    );

    // ── List members on fresh DB -> empty ───────────────────────────────
    let res = with_cookie!(server.get("/api/members"), &admin_cookie).await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to list members"
    );
    let body: Value = res.json();
    // Could be an empty array or a paginated object with empty data
    let is_empty = body.is_array() && body.as_array().unwrap().is_empty()
        || body.get("data").map_or(false, |d| {
            d.is_array() && d.as_array().unwrap().is_empty()
        });
    assert!(is_empty, "member list should be empty on fresh DB");

    // ── Seed a member and list again ────────────────────────────────────
    let alice = common::seed_member(&pool, "Alice Smith").await;

    let res = with_cookie!(server.get("/api/members"), &admin_cookie).await;
    let body_text = res.text();
    assert!(
        body_text.contains("Alice Smith"),
        "member list should contain 'Alice Smith' after seeding"
    );

    // ── Get member by id ────────────────────────────────────────────────
    let res = with_cookie!(
        server.get(&format!("/api/members/{}", alice.id)),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "GET /api/members/:id should succeed for admin"
    );
    let body: Value = res.json();
    let body_str = serde_json::to_string(&body).unwrap();
    assert!(
        body_str.contains("Alice Smith"),
        "get member should return correct name"
    );
    assert!(
        body_str.contains(&alice.id),
        "get member should return correct id"
    );

    // ── Get nonexistent member ──────────────────────────────────────────
    let res = with_cookie!(
        server.get("/api/members/00000000-0000-0000-0000-000000000000"),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_server_error() || res.status_code().is_client_error(),
        "GET nonexistent member should return error"
    );

    // ── Admin create member ─────────────────────────────────────────────
    let res = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Bob Jones",
            "email": "bob@example.com",
            "phone": "1234567890"
        })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to create a member"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "direct",
        "admin create member should be direct (no approval needed)"
    );
    assert!(
        body["memberId"].is_string() || body.get("memberId").is_some(),
        "admin create should return a memberId"
    );
    assert!(
        body["approvalId"].is_null(),
        "admin create should not return an approvalId"
    );

    // ── Member ID follows BSDS format ───────────────────────────────────
    if let Some(member_id_val) = body.get("memberId") {
        if let Some(mid) = member_id_val.as_str() {
            assert!(
                mid.starts_with("BSDS-"),
                "member ID '{}' should start with 'BSDS-'",
                mid
            );
            assert!(
                mid.ends_with("-00"),
                "member ID '{}' should end with '-00'",
                mid
            );
        }
    }

    // ── Operator create member -> pending approval ──────────────────────
    let res = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Charlie Operator",
            "email": "charlie@example.com",
            "phone": "9876543210"
        })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should be able to create a member"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "pending_approval",
        "operator create member should require approval"
    );
    assert!(
        body.get("approvalId").is_some() && !body["approvalId"].is_null(),
        "operator create should return an approvalId"
    );
    assert!(
        body["memberId"].is_null(),
        "operator create should not return a memberId until approved"
    );

    // ── Admin update member -> direct ───────────────────────────────────
    let res = with_cookie!(
        server
            .patch(&format!("/api/members/{}", alice.id))
            .json(&json!({ "name": "Alice Updated" })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to update a member"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "direct",
        "admin update should be direct"
    );

    // ── Operator update member -> pending approval ──────────────────────
    let res = with_cookie!(
        server
            .patch(&format!("/api/members/{}", alice.id))
            .json(&json!({ "name": "Alice Op Update" })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should be able to update a member"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "pending_approval",
        "operator update should require approval"
    );

    // ── Admin delete member ─────────────────────────────────────────────
    let delete_target = common::seed_member(&pool, "Delete Target").await;
    let res = with_cookie!(
        server.delete(&format!("/api/members/{}", delete_target.id)),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to delete a member"
    );
    let body: Value = res.json();
    assert_eq!(body["ok"], true, "admin delete should return ok==true");

    // Verify user membership_status in DB == "SUSPENDED"
    let row = sqlx::query_scalar::<_, String>(
        "SELECT membership_status FROM users WHERE id = ?",
    )
    .bind(&delete_target.user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        row, "SUSPENDED",
        "user membership_status should be SUSPENDED after admin delete"
    );

    // ── Sub-members: list -> empty ──────────────────────────────────────
    let res = with_cookie!(
        server.get(&format!("/api/members/{}/sub-members", alice.id)),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "should be able to list sub-members"
    );
    let body: Value = res.json();
    let sub_list = if body.is_array() {
        body.as_array().unwrap().clone()
    } else {
        body.get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default()
    };
    assert!(
        sub_list.is_empty(),
        "sub-member list should be empty initially"
    );

    // ── Admin add sub-member -> direct ──────────────────────────────────
    let res = with_cookie!(
        server
            .post(&format!("/api/members/{}/sub-members", alice.id))
            .json(&json!({
                "name": "Sub One",
                "phone": "1111111111",
                "relation": "SPOUSE"
            })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to add a sub-member"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "direct",
        "admin add sub-member should be direct"
    );

    // DB count == 1
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sub_members WHERE member_id = ?")
            .bind(&alice.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1, "should have 1 sub-member in DB after add");

    // ── Add up to 3 sub-members, then 4th should fail ───────────────────
    let res = with_cookie!(
        server
            .post(&format!("/api/members/{}/sub-members", alice.id))
            .json(&json!({
                "name": "Sub Two",
                "phone": "2222222222",
                "relation": "CHILD"
            })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "second sub-member add should succeed"
    );

    let res = with_cookie!(
        server
            .post(&format!("/api/members/{}/sub-members", alice.id))
            .json(&json!({
                "name": "Sub Three",
                "phone": "3333333333",
                "relation": "CHILD"
            })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "third sub-member add should succeed"
    );

    let res = with_cookie!(
        server
            .post(&format!("/api/members/{}/sub-members", alice.id))
            .json(&json!({
                "name": "Sub Four",
                "phone": "4444444444",
                "relation": "PARENT"
            })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_client_error(),
        "fourth sub-member add should fail (max 3)"
    );

    // ── Get the sub-member id for update/remove tests ───────────────────
    let sub_row = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM sub_members WHERE member_id = ? AND name = 'Sub One'",
    )
    .bind(&alice.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let sub_one_id = sub_row.0;

    // ── Update sub-member -> name changed in DB ─────────────────────────
    let res = with_cookie!(
        server
            .put(&format!("/api/members/{}/sub-members", alice.id))
            .json(&json!({
                "subMemberId": sub_one_id,
                "name": "Sub One Updated",
                "phone": "1111111111",
                "relation": "SPOUSE"
            })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to update a sub-member"
    );

    let updated_name: String =
        sqlx::query_scalar("SELECT name FROM sub_members WHERE id = ?")
            .bind(&sub_one_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        updated_name, "Sub One Updated",
        "sub-member name should be updated in DB"
    );

    // ── Remove sub-member -> DB count decreases ─────────────────────────
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sub_members WHERE member_id = ?")
            .bind(&alice.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let res = with_cookie!(
        server
            .delete(&format!("/api/members/{}/sub-members", alice.id))
            .json(&json!({ "subMemberId": sub_one_id })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to remove a sub-member"
    );

    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sub_members WHERE member_id = ?")
            .bind(&alice.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count_after,
        count_before - 1,
        "sub-member count should decrease by 1 after removal"
    );

    // ── Organiser cannot add sub-member -> 403 ──────────────────────────
    let res = with_cookie!(
        server
            .post(&format!("/api/members/{}/sub-members", alice.id))
            .json(&json!({
                "name": "Org Sub",
                "phone": "5555555555",
                "relation": "SPOUSE"
            })),
        &organiser_cookie
    )
    .await;
    assert_eq!(
        res.status_code().as_u16(),
        403,
        "organiser should not be allowed to add sub-members"
    );

    // ── Operator can add sub-member -> pending_approval ─────────────────
    // First, remove one sub-member to make room (currently at 2 after removal)
    let res = with_cookie!(
        server
            .post(&format!("/api/members/{}/sub-members", alice.id))
            .json(&json!({
                "name": "Op Sub",
                "phone": "6666666666",
                "relation": "CHILD"
            })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should be able to add sub-member"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "pending_approval",
        "operator add sub-member should require approval"
    );
    assert!(
        body.get("approvalId").is_some() && !body["approvalId"].is_null(),
        "operator sub-member add should return approvalId"
    );

    // ── MEMBER role cannot list sub-members -> 403 ──────────────────────
    let res = with_cookie!(
        server.get(&format!("/api/members/{}/sub-members", alice.id)),
        &member_cookie
    )
    .await;
    assert_eq!(
        res.status_code().as_u16(),
        403,
        "MEMBER role should not be able to list sub-members"
    );
}

// ---------------------------------------------------------------------------
// 2. Approval workflow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn approval_workflow() {
    let (server, pool, _tmp) = common::test_app().await;

    let admin = common::seed_admin_user(&pool).await;
    let operator = common::seed_operator_user(&pool).await;

    let admin_cookie = common::auth_cookie(&admin);
    let operator_cookie = common::auth_cookie(&operator);

    // ── Auth guards ─────────────────────────────────────────────────────
    let res = server.get("/api/approvals").await;
    assert!(
        res.status_code().is_client_error(),
        "GET /api/approvals without auth should return 401"
    );

    let res = server
        .post("/api/approvals/00000000-0000-0000-0000-000000000000/approve")
        .json(&json!({}))
        .await;
    assert!(
        res.status_code().is_client_error(),
        "POST /api/approvals/:id/approve without auth should return 401"
    );

    // ── List approvals on empty DB -> empty ─────────────────────────────
    let res = with_cookie!(server.get("/api/approvals"), &admin_cookie).await;
    assert!(
        res.status_code().is_success(),
        "admin should list approvals"
    );
    let body: Value = res.json();
    let is_empty = body.is_array() && body.as_array().unwrap().is_empty()
        || body.get("data").map_or(false, |d| {
            d.is_array() && d.as_array().unwrap().is_empty()
        });
    assert!(is_empty, "approval list should be empty on fresh DB");

    // ── Seed a pending approval via operator creating a member ───────────
    let approval_id =
        common::create_pending_approval(&pool, &server, &operator_cookie).await;

    // ── List approvals -> includes the seeded one ───────────────────────
    let res = with_cookie!(server.get("/api/approvals"), &admin_cookie).await;
    let body_text = res.text();
    assert!(
        body_text.contains(&approval_id),
        "approval list should contain the seeded approval_id"
    );

    // ── Verify PENDING status in DB ─────────────────────────────────────
    let status: String =
        sqlx::query_scalar("SELECT status FROM approvals WHERE id = ?")
            .bind(&approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        status, "PENDING",
        "newly created approval should be PENDING in DB"
    );

    // ── Create a transaction approval to test approvalType/direction ────
    // Operator creates a CASH_IN transaction -> pending approval
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 1000.0,
            "paymentMode": "CASH",
            "purpose": "Test cash in for approval type check"
        })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should create a transaction"
    );
    let tx_body: Value = res.json();
    let tx_approval_id = tx_body["approvalId"]
        .as_str()
        .expect("operator tx should return approvalId");

    // ── Check approvalType and direction on the transaction approval ────
    let res = with_cookie!(server.get("/api/approvals"), &admin_cookie).await;
    let body_text = res.text();
    assert!(
        body_text.contains(tx_approval_id),
        "approval list should contain the tx approval"
    );
    // Parse body to find the tx approval and check fields
    let body: Value = serde_json::from_str(&body_text).unwrap();
    let approvals_list = if body.is_array() {
        body.as_array().unwrap().clone()
    } else {
        body.get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let tx_approval = approvals_list
        .iter()
        .find(|a| a["id"].as_str() == Some(tx_approval_id));
    if let Some(appr) = tx_approval {
        assert_eq!(
            appr["approvalType"], "MEMBERSHIP_PAYMENT_APPROVAL",
            "transaction approval should have correct approvalType"
        );
        assert_eq!(
            appr["direction"], "INCOMING",
            "CASH_IN transaction approval direction should be INCOMING"
        );
    }

    // ── Review: approve the transaction approval ────────────────────────
    // First, seed a direct transaction as admin so we have a tx to approve
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 2000.0,
            "paymentMode": "CASH",
            "purpose": "Admin direct tx"
        })),
        &operator_cookie
    )
    .await;
    let op_tx_body: Value = res.json();
    let op_tx_approval_id = op_tx_body["approvalId"]
        .as_str()
        .expect("operator tx should have approvalId");

    let res = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/approve", op_tx_approval_id))
            .json(&json!({})),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to approve a transaction approval"
    );
    let body: Value = res.json();
    assert_eq!(
        body["ok"], true,
        "approve should return ok==true"
    );

    // ── DB: approval status == APPROVED after approve ───────────────────
    let status: String =
        sqlx::query_scalar("SELECT status FROM approvals WHERE id = ?")
            .bind(op_tx_approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        status, "APPROVED",
        "approval status should be APPROVED after admin approves"
    );

    // ── DB: tx approval_status == APPROVED and audit log exists ─────────
    // Find the transaction associated with this approval
    let maybe_tx_id: Option<String> = sqlx::query_scalar(
        "SELECT entity_id FROM approvals WHERE id = ? AND entity_type = 'TRANSACTION'",
    )
    .bind(op_tx_approval_id)
    .fetch_optional(&pool)
    .await
    .unwrap();
    if let Some(tx_id) = maybe_tx_id {
        let tx_status: String =
            sqlx::query_scalar("SELECT approval_status FROM transactions WHERE id = ?")
                .bind(&tx_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            tx_status, "APPROVED",
            "transaction approval_status should be APPROVED after admin approves"
        );

        let audit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE entity_id = ? AND action = 'TRANSACTION_APPROVED'",
        )
        .bind(&tx_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            audit_count > 0,
            "audit_logs should contain TRANSACTION_APPROVED entry after approval"
        );
    }

    // ── Review: reject a transaction approval ───────────────────────────
    // Create another operator tx to reject
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 500.0,
            "paymentMode": "CASH",
            "purpose": "To be rejected"
        })),
        &operator_cookie
    )
    .await;
    let reject_tx_body: Value = res.json();
    let reject_approval_id = reject_tx_body["approvalId"]
        .as_str()
        .expect("should have approvalId for rejection");

    let res = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/reject", reject_approval_id))
            .json(&json!({ "notes": "Not valid" })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to reject an approval"
    );
    let body: Value = res.json();
    assert_eq!(body["ok"], true, "reject should return ok==true");

    // ── DB: tx approval_status == REJECTED after rejection ──────────────
    let maybe_tx_id: Option<String> = sqlx::query_scalar(
        "SELECT entity_id FROM approvals WHERE id = ? AND entity_type = 'TRANSACTION'",
    )
    .bind(reject_approval_id)
    .fetch_optional(&pool)
    .await
    .unwrap();
    if let Some(tx_id) = maybe_tx_id {
        let tx_status: String =
            sqlx::query_scalar("SELECT approval_status FROM transactions WHERE id = ?")
                .bind(&tx_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            tx_status, "REJECTED",
            "transaction approval_status should be REJECTED after admin rejects"
        );
    }

    // ── Notes saved in DB after rejection ───────────────────────────────
    let notes: Option<String> =
        sqlx::query_scalar("SELECT notes FROM approvals WHERE id = ?")
            .bind(reject_approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        notes.as_deref(),
        Some("Not valid"),
        "rejection notes should be saved in DB"
    );

    // ── Approve MEMBER_ADD -> user record created ───────────────────────
    // Operator submits a new member
    let res = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Approval Target",
            "email": "approval_target@example.com",
            "phone": "7777777777"
        })),
        &operator_cookie
    )
    .await;
    let member_add_body: Value = res.json();
    let member_add_approval_id = member_add_body["approvalId"]
        .as_str()
        .expect("operator member create should have approvalId");

    // Admin approves
    let res = with_cookie!(
        server
            .post(&format!(
                "/api/approvals/{}/approve",
                member_add_approval_id
            ))
            .json(&json!({})),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should approve member_add"
    );

    // Verify user record was created
    let user_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE email = 'approval_target@example.com'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        user_count > 0,
        "user record should be created after approving MEMBER_ADD"
    );

    // ── Reject MEMBER_ADD -> no user record ─────────────────────────────
    let res = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Reject Target",
            "email": "reject_target@example.com",
            "phone": "8888888888"
        })),
        &operator_cookie
    )
    .await;
    let reject_member_body: Value = res.json();
    let reject_member_approval_id = reject_member_body["approvalId"]
        .as_str()
        .expect("should have approvalId");

    let res = with_cookie!(
        server
            .post(&format!(
                "/api/approvals/{}/reject",
                reject_member_approval_id
            ))
            .json(&json!({ "notes": "Duplicate entry" })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should reject member_add"
    );

    let user_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE email = 'reject_target@example.com'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        user_count, 0,
        "no user record should exist after rejecting MEMBER_ADD"
    );

    // ── Rejection notes saved ───────────────────────────────────────────
    let notes: Option<String> =
        sqlx::query_scalar("SELECT notes FROM approvals WHERE id = ?")
            .bind(reject_member_approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        notes.as_deref(),
        Some("Duplicate entry"),
        "rejection notes should be saved in DB for member_add rejection"
    );

    // ── Operator cannot approve -> 403 ──────────────────────────────────
    // Create another pending approval for operator to try to approve
    let res = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Op Approve Attempt",
            "email": "op_approve@example.com",
            "phone": "9999999999"
        })),
        &operator_cookie
    )
    .await;
    let op_attempt_body: Value = res.json();
    let op_attempt_approval_id = op_attempt_body["approvalId"]
        .as_str()
        .expect("should have approvalId");

    let res = with_cookie!(
        server
            .post(&format!(
                "/api/approvals/{}/approve",
                op_attempt_approval_id
            ))
            .json(&json!({})),
        &operator_cookie
    )
    .await;
    assert_eq!(
        res.status_code().as_u16(),
        403,
        "operator should not be able to approve approvals"
    );
}

// ---------------------------------------------------------------------------
// 3. Transaction and membership lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn transaction_and_membership_lifecycle() {
    let (server, pool, _tmp) = common::test_app().await;

    let admin = common::seed_admin_user(&pool).await;
    let operator = common::seed_operator_user(&pool).await;

    let admin_cookie = common::auth_cookie(&admin);
    let operator_cookie = common::auth_cookie(&operator);

    // ═══════════════════════════════════════════════════════════════════
    // TRANSACTION AUTH GUARDS
    // ═══════════════════════════════════════════════════════════════════

    let res = server.get("/api/transactions").await;
    assert!(
        res.status_code().is_client_error(),
        "GET /api/transactions without auth should return 401"
    );

    let res = server.get("/api/transactions/summary").await;
    assert!(
        res.status_code().is_client_error(),
        "GET /api/transactions/summary without auth should return 401"
    );

    let res = server
        .post("/api/transactions")
        .json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 100.0,
            "paymentMode": "CASH",
            "purpose": "unauth"
        }))
        .await;
    assert!(
        res.status_code().is_client_error(),
        "POST /api/transactions without auth should return 401"
    );

    // ═══════════════════════════════════════════════════════════════════
    // TRANSACTION LIST (empty), SUMMARY (empty)
    // ═══════════════════════════════════════════════════════════════════

    let res = with_cookie!(server.get("/api/transactions"), &admin_cookie).await;
    assert!(
        res.status_code().is_success(),
        "admin should list transactions"
    );
    let body: Value = res.json();
    let is_empty = body.is_array() && body.as_array().unwrap().is_empty()
        || body.get("data").map_or(false, |d| {
            d.is_array() && d.as_array().unwrap().is_empty()
        });
    assert!(is_empty, "transaction list should be empty on fresh DB");

    let res = with_cookie!(
        server.get("/api/transactions/summary"),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should get transaction summary"
    );
    let summary: Value = res.json();
    assert!(
        summary.get("totalIncome").is_some(),
        "summary should have totalIncome"
    );
    assert!(
        summary.get("totalExpenses").is_some(),
        "summary should have totalExpenses"
    );
    assert!(
        summary.get("pendingAmount").is_some(),
        "summary should have pendingAmount"
    );
    assert!(
        summary.get("netBalance").is_some(),
        "summary should have netBalance"
    );

    // ═══════════════════════════════════════════════════════════════════
    // ADMIN CREATE CASH_IN TRANSACTION
    // ═══════════════════════════════════════════════════════════════════

    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 3000.0,
            "paymentMode": "CASH",
            "purpose": "Admin cash in"
        })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should create a CASH_IN transaction"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "direct",
        "admin transaction should be direct"
    );
    assert!(
        body.get("transactionId").is_some() && !body["transactionId"].is_null(),
        "admin tx should return transactionId"
    );
    let admin_tx_id = body["transactionId"]
        .as_str()
        .unwrap()
        .to_string();

    // ── Admin CASH_IN creates TRANSACTION_CREATED audit log ─────────────
    let audit_created: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE entity_id = ? AND action = 'TRANSACTION_CREATED'",
    )
    .bind(&admin_tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        audit_created > 0,
        "admin tx should create TRANSACTION_CREATED audit log"
    );

    // ── Admin tx creates TRANSACTION_APPROVED audit log (auto-approved) ─
    let audit_approved: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE entity_id = ? AND action = 'TRANSACTION_APPROVED'",
    )
    .bind(&admin_tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        audit_approved > 0,
        "admin tx should create TRANSACTION_APPROVED audit log (auto-approved)"
    );

    // ── ADMIN CREATE CASH_OUT TRANSACTION ───────────────────────────────
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_OUT",
            "category": "OTHER",
            "amount": 500.0,
            "paymentMode": "CASH",
            "purpose": "Admin cash out"
        })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should create a CASH_OUT transaction"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "direct",
        "admin CASH_OUT should be direct"
    );

    // ── LIST with seeded tx -> body contains tx_id ──────────────────────
    let res = with_cookie!(server.get("/api/transactions"), &admin_cookie).await;
    let body_text = res.text();
    assert!(
        body_text.contains(&admin_tx_id),
        "transaction list should contain seeded tx_id"
    );

    // ── GET transaction by id ───────────────────────────────────────────
    let res = with_cookie!(
        server.get(&format!("/api/transactions/{}", admin_tx_id)),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "should get transaction by id"
    );
    let body: Value = res.json();
    let body_str = serde_json::to_string(&body).unwrap();
    assert!(
        body_str.contains(&admin_tx_id),
        "get tx should return correct id"
    );
    assert!(
        body_str.contains("OTHER"),
        "get tx should return correct category"
    );

    // ── SUMMARY with seeded APPROVED CASH_IN 3000 ───────────────────────
    let res = with_cookie!(
        server.get("/api/transactions/summary"),
        &admin_cookie
    )
    .await;
    let summary: Value = res.json();
    let total_income = summary["totalIncome"]
        .as_f64()
        .unwrap_or(0.0);
    assert!(
        total_income >= 3000.0,
        "totalIncome should be >= 3000.0 after admin CASH_IN, got {}",
        total_income
    );

    // ── OPERATOR CREATE -> pending_approval ─────────────────────────────
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 1000.0,
            "paymentMode": "CASH",
            "purpose": "Operator tx"
        })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should create a transaction"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "pending_approval",
        "operator tx should require approval"
    );
    assert!(
        body.get("approvalId").is_some() && !body["approvalId"].is_null(),
        "operator tx should return approvalId"
    );
    let op_tx_approval_id = body["approvalId"].as_str().unwrap().to_string();

    // ── Operator tx creates ONLY TRANSACTION_CREATED (no APPROVED) ──────
    // We need the tx id; find it from the approval
    let op_tx_id: Option<String> = sqlx::query_scalar(
        "SELECT entity_id FROM approvals WHERE id = ?",
    )
    .bind(&op_tx_approval_id)
    .fetch_optional(&pool)
    .await
    .unwrap();
    if let Some(ref tx_id) = op_tx_id {
        let created_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE entity_id = ? AND action = 'TRANSACTION_CREATED'",
        )
        .bind(tx_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            created_count > 0,
            "operator tx should have TRANSACTION_CREATED audit log"
        );

        let approved_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE entity_id = ? AND action = 'TRANSACTION_APPROVED'",
        )
        .bind(tx_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            approved_count, 0,
            "operator tx should NOT have TRANSACTION_APPROVED audit log (still pending)"
        );
    }

    // ── DELETE transaction -> 400 or 405 (immutable) ────────────────────
    let res = with_cookie!(
        server.delete(&format!("/api/transactions/{}", admin_tx_id)),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().as_u16() == 400 || res.status_code().as_u16() == 405,
        "DELETE transaction should return 400 or 405 (transactions are immutable), got {}",
        res.status_code()
    );

    // ── Admin can reject pending tx -> status REJECTED ──────────────────
    // Create another operator tx to reject via the transaction route
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 200.0,
            "paymentMode": "CASH",
            "purpose": "To be rejected via tx route"
        })),
        &operator_cookie
    )
    .await;
    let reject_body: Value = res.json();
    let reject_tx_approval_id = reject_body["approvalId"]
        .as_str()
        .unwrap()
        .to_string();
    let reject_tx_id: String = sqlx::query_scalar(
        "SELECT entity_id FROM approvals WHERE id = ?",
    )
    .bind(&reject_tx_approval_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let res = with_cookie!(
        server
            .post(&format!("/api/transactions/{}/reject", reject_tx_id))
            .json(&json!({})),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should reject a pending transaction"
    );

    // DB: tx approval_status == REJECTED
    let tx_status: String =
        sqlx::query_scalar("SELECT approval_status FROM transactions WHERE id = ?")
            .bind(&reject_tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        tx_status, "REJECTED",
        "transaction approval_status should be REJECTED after admin rejects"
    );

    // ── Rejected tx creates activity_log with action='transaction_rejected'
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs WHERE entity_id = ? AND action = 'transaction_rejected'",
    )
    .bind(&reject_tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        activity_count > 0,
        "rejected tx should create activity_log with action='transaction_rejected'"
    );

    // ── Operator cannot reject -> 403 ───────────────────────────────────
    // Create yet another operator tx
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 150.0,
            "paymentMode": "CASH",
            "purpose": "Op reject attempt"
        })),
        &operator_cookie
    )
    .await;
    let op_reject_body: Value = res.json();
    let op_reject_approval_id = op_reject_body["approvalId"]
        .as_str()
        .unwrap();
    let op_reject_tx_id: String = sqlx::query_scalar(
        "SELECT entity_id FROM approvals WHERE id = ?",
    )
    .bind(op_reject_approval_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let res = with_cookie!(
        server
            .post(&format!("/api/transactions/{}/reject", op_reject_tx_id))
            .json(&json!({})),
        &operator_cookie
    )
    .await;
    assert_eq!(
        res.status_code().as_u16(),
        403,
        "operator should not be able to reject transactions"
    );

    // ── Approve via approval route creates TRANSACTION_APPROVED audit + activity log
    let res = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/approve", op_tx_approval_id))
            .json(&json!({})),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should approve operator tx via approval route"
    );

    if let Some(ref tx_id) = op_tx_id {
        let approved_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE entity_id = ? AND action = 'TRANSACTION_APPROVED'",
        )
        .bind(tx_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            approved_count > 0,
            "approving via approval route should create TRANSACTION_APPROVED audit log"
        );

        let activity_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM activity_logs WHERE entity_id = ? AND action = 'transaction_approved'",
        )
        .bind(tx_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            activity_count > 0,
            "approving via approval route should create activity log"
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // MEMBERSHIP AUTH GUARDS
    // ═══════════════════════════════════════════════════════════════════

    let res = server.get("/api/memberships").await;
    assert!(
        res.status_code().is_client_error(),
        "GET /api/memberships without auth should return 401"
    );

    let res = server
        .post("/api/memberships")
        .json(&json!({
            "memberId": "fake",
            "type": "MONTHLY",
            "amount": 250.0,
            "feeType": "SUBSCRIPTION",
            "isApplicationFee": false
        }))
        .await;
    assert!(
        res.status_code().is_client_error(),
        "POST /api/memberships without auth should return 401"
    );

    // ═══════════════════════════════════════════════════════════════════
    // MEMBERSHIP LIST (empty)
    // ═══════════════════════════════════════════════════════════════════

    let res = with_cookie!(server.get("/api/memberships"), &admin_cookie).await;
    assert!(
        res.status_code().is_success(),
        "admin should list memberships"
    );
    let body: Value = res.json();
    let is_empty = body.is_array() && body.as_array().unwrap().is_empty()
        || body.get("data").map_or(false, |d| {
            d.is_array() && d.as_array().unwrap().is_empty()
        });
    assert!(
        is_empty,
        "membership list should be empty without member_id param on fresh DB"
    );

    // ── Seed a member and insert membership directly into DB ────────────
    let test_member = common::seed_member(&pool, "Membership Test Member").await;

    let membership_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO memberships (id, member_id, type, amount, fee_type, status) VALUES (?, ?, 'MONTHLY', 250.0, 'SUBSCRIPTION', 'ACTIVE')",
    )
    .bind(&membership_id)
    .bind(&test_member.id)
    .execute(&pool)
    .await
    .unwrap();

    // ── List memberships by member_id -> returns inserted membership ────
    let res = with_cookie!(
        server.get(&format!(
            "/api/memberships?member_id={}",
            test_member.id
        )),
        &admin_cookie
    )
    .await;
    let body_text = res.text();
    assert!(
        body_text.contains(&membership_id),
        "membership list filtered by member_id should contain inserted membership"
    );

    // ── Get membership by id ────────────────────────────────────────────
    let res = with_cookie!(
        server.get(&format!("/api/memberships/{}", membership_id)),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "should get membership by id"
    );
    let body: Value = res.json();
    let body_str = serde_json::to_string(&body).unwrap();
    assert!(
        body_str.contains(&membership_id),
        "get membership should return correct id"
    );
    assert!(
        body_str.contains("MONTHLY"),
        "get membership should return correct type"
    );

    // ── Admin create MONTHLY membership (amount=250.0) -> direct ────────
    let another_member = common::seed_member(&pool, "Another Member").await;
    let res = with_cookie!(
        server.post("/api/memberships").json(&json!({
            "memberId": another_member.id,
            "type": "MONTHLY",
            "amount": 250.0,
            "feeType": "SUBSCRIPTION",
            "isApplicationFee": false
        })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should create a MONTHLY membership"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "direct",
        "admin membership create should be direct"
    );

    // ── Wrong amount (999.0 for MONTHLY) -> 400 ─────────────────────────
    let wrong_amount_member = common::seed_member(&pool, "Wrong Amount Member").await;
    let res = with_cookie!(
        server.post("/api/memberships").json(&json!({
            "memberId": wrong_amount_member.id,
            "type": "MONTHLY",
            "amount": 999.0,
            "feeType": "SUBSCRIPTION",
            "isApplicationFee": false
        })),
        &admin_cookie
    )
    .await;
    assert_eq!(
        res.status_code().as_u16(),
        400,
        "wrong amount for MONTHLY membership should return 400"
    );

    // ── Operator create membership -> pending_approval ──────────────────
    let op_member = common::seed_member(&pool, "Operator Membership Member").await;
    let res = with_cookie!(
        server.post("/api/memberships").json(&json!({
            "memberId": op_member.id,
            "type": "MONTHLY",
            "amount": 250.0,
            "feeType": "SUBSCRIPTION",
            "isApplicationFee": false
        })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should create a membership"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "pending_approval",
        "operator membership create should require approval"
    );

    // ── Approve membership via PATCH ────────────────────────────────────
    // Insert a PENDING membership to approve
    let pending_ms_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO memberships (id, member_id, type, amount, fee_type, status) VALUES (?, ?, 'MONTHLY', 250.0, 'SUBSCRIPTION', 'PENDING')",
    )
    .bind(&pending_ms_id)
    .bind(&test_member.id)
    .execute(&pool)
    .await
    .unwrap();

    let res = with_cookie!(
        server
            .patch(&format!("/api/memberships/{}", pending_ms_id))
            .json(&json!({ "action": "approve" })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should approve membership via PATCH"
    );
    let body: Value = res.json();
    assert_eq!(
        body["ok"], true,
        "membership approve should return ok==true"
    );

    // ── Reject membership via PATCH ─────────────────────────────────────
    let reject_ms_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO memberships (id, member_id, type, amount, fee_type, status) VALUES (?, ?, 'MONTHLY', 250.0, 'SUBSCRIPTION', 'PENDING')",
    )
    .bind(&reject_ms_id)
    .bind(&test_member.id)
    .execute(&pool)
    .await
    .unwrap();

    let res = with_cookie!(
        server
            .patch(&format!("/api/memberships/{}", reject_ms_id))
            .json(&json!({ "action": "reject", "notes": "Not needed" })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should reject membership via PATCH"
    );
    let body: Value = res.json();
    assert_eq!(
        body["ok"], true,
        "membership reject should return ok==true"
    );

    // ═══════════════════════════════════════════════════════════════════
    // includesSubscription creates membership record
    // ═══════════════════════════════════════════════════════════════════

    let sub_member = common::seed_member(&pool, "Subscription Member").await;
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "MEMBERSHIP",
            "amount": 250.0,
            "paymentMode": "CASH",
            "purpose": "Monthly subscription payment",
            "memberId": sub_member.id,
            "membershipType": "MONTHLY",
            "includesSubscription": true
        })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should create tx with includesSubscription"
    );

    // Verify membership record was created
    let ms_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memberships WHERE member_id = ?",
    )
    .bind(&sub_member.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        ms_count > 0,
        "includesSubscription should create a membership record"
    );

    // Verify membership details
    let (ms_type, ms_amount): (String, f64) = sqlx::query_as(
        "SELECT type, amount FROM memberships WHERE member_id = ? ORDER BY rowid DESC LIMIT 1",
    )
    .bind(&sub_member.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        ms_type, "MONTHLY",
        "created membership should have type MONTHLY"
    );
    assert!(
        (ms_amount - 250.0).abs() < f64::EPSILON,
        "created membership amount should be 250.0, got {}",
        ms_amount
    );

    // Verify the transaction category is MEMBERSHIP
    let body: Value = res.json();
    if let Some(tx_id) = body.get("transactionId").and_then(|v| v.as_str()) {
        let cat: String =
            sqlx::query_scalar("SELECT category FROM transactions WHERE id = ?")
                .bind(tx_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            cat, "MEMBERSHIP",
            "includesSubscription tx should have category MEMBERSHIP"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Member lifecycle state machine
// ---------------------------------------------------------------------------

#[tokio::test]
async fn member_lifecycle_state_machine() {
    let (server, pool, _tmp) = common::test_app().await;

    let admin = common::seed_admin_user(&pool).await;
    let operator = common::seed_operator_user(&pool).await;

    let admin_cookie = common::auth_cookie(&admin);
    let operator_cookie = common::auth_cookie(&operator);

    // ── Operator-created member appears in approval queue ───────────────
    let res = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Lifecycle Member",
            "email": "lifecycle@example.com",
            "phone": "1112223333"
        })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should submit a member"
    );
    let body: Value = res.json();
    let lifecycle_approval_id = body["approvalId"]
        .as_str()
        .expect("operator submit should return approvalId")
        .to_string();

    // Admin can see it in the approval list
    let res = with_cookie!(server.get("/api/approvals"), &admin_cookie).await;
    let body_text = res.text();
    assert!(
        body_text.contains(&lifecycle_approval_id),
        "operator-created member should appear in admin's approval queue"
    );

    // ── Activity log recorded on member submission ──────────────────────
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs WHERE action LIKE '%member%' OR action LIKE '%submit%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        activity_count > 0,
        "activity log should be recorded on member submission"
    );

    // ── Admin approve member_add -> creates user+member ─────────────────
    let res = with_cookie!(
        server
            .post(&format!(
                "/api/approvals/{}/approve",
                lifecycle_approval_id
            ))
            .json(&json!({})),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should approve member_add"
    );

    // Approval status in DB
    let approval_status: String =
        sqlx::query_scalar("SELECT status FROM approvals WHERE id = ?")
            .bind(&lifecycle_approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        approval_status, "APPROVED",
        "approval should be APPROVED in DB after admin approves"
    );

    // User created
    let user_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE email = 'lifecycle@example.com'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        user_count > 0,
        "user record should exist after admin approves member_add"
    );

    // Member created
    let member_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM members WHERE name = 'Lifecycle Member'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        member_count > 0,
        "member record should exist after admin approves member_add"
    );

    // ── Admin reject member_add -> no user ──────────────────────────────
    let res = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Reject Lifecycle",
            "email": "reject_lifecycle@example.com",
            "phone": "4445556666"
        })),
        &operator_cookie
    )
    .await;
    let body: Value = res.json();
    let reject_approval_id = body["approvalId"]
        .as_str()
        .expect("should have approvalId")
        .to_string();

    let res = with_cookie!(
        server
            .post(&format!("/api/approvals/{}/reject", reject_approval_id))
            .json(&json!({ "notes": "Not eligible" })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should reject member_add"
    );

    let approval_status: String =
        sqlx::query_scalar("SELECT status FROM approvals WHERE id = ?")
            .bind(&reject_approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        approval_status, "REJECTED",
        "approval should be REJECTED in DB after admin rejects"
    );

    let user_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE email = 'reject_lifecycle@example.com'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        user_count, 0,
        "no user should exist after admin rejects member_add"
    );

    // ── Rejection notes saved in DB ─────────────────────────────────────
    let notes: Option<String> =
        sqlx::query_scalar("SELECT notes FROM approvals WHERE id = ?")
            .bind(&reject_approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        notes.as_deref(),
        Some("Not eligible"),
        "rejection notes should be saved in DB"
    );

    // ── Operator update queues approval (not direct) ────────────────────
    // We need an existing member to update
    let existing_member = common::seed_member(&pool, "Existing For Update").await;

    let res = with_cookie!(
        server
            .patch(&format!("/api/members/{}", existing_member.id))
            .json(&json!({ "name": "Op Updated Name" })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should be able to submit an update"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "pending_approval",
        "operator update should queue an approval, not be direct"
    );

    // ── Admin update is direct (no approval) ────────────────────────────
    let res = with_cookie!(
        server
            .patch(&format!("/api/members/{}", existing_member.id))
            .json(&json!({ "name": "Admin Updated Name" })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should be able to update directly"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "direct",
        "admin update should be direct, no approval needed"
    );

    // ── Operator delete queues MEMBER_DELETE approval, user NOT suspended
    let delete_member = common::seed_member(&pool, "Op Delete Target").await;
    let res = with_cookie!(
        server.delete(&format!("/api/members/{}", delete_member.id)),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should be able to submit a delete request"
    );
    let body: Value = res.json();
    assert_eq!(
        body["action"], "pending_approval",
        "operator delete should queue an approval"
    );

    // User should NOT be suspended yet (still pending)
    let user_status: String = sqlx::query_scalar(
        "SELECT membership_status FROM users WHERE id = ?",
    )
    .bind(&delete_member.user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_ne!(
        user_status, "SUSPENDED",
        "user should NOT be suspended after operator delete (still pending approval)"
    );

    // ── Admin delete suspends immediately ───────────────────────────────
    let admin_delete_member = common::seed_member(&pool, "Admin Delete Target").await;
    let res = with_cookie!(
        server.delete(&format!("/api/members/{}", admin_delete_member.id)),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should delete member"
    );
    let body: Value = res.json();
    assert_eq!(body["ok"], true, "admin delete should return ok==true");

    let user_status: String = sqlx::query_scalar(
        "SELECT membership_status FROM users WHERE id = ?",
    )
    .bind(&admin_delete_member.user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        user_status, "SUSPENDED",
        "user should be SUSPENDED immediately after admin delete"
    );
}

// ---------------------------------------------------------------------------
// 5. Dashboard stats aggregation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dashboard_stats_aggregation() {
    let (server, pool, _tmp) = common::test_app().await;

    let admin = common::seed_admin_user(&pool).await;
    let operator = common::seed_operator_user(&pool).await;

    let admin_cookie = common::auth_cookie(&admin);
    let operator_cookie = common::auth_cookie(&operator);

    // ── Auth guard: stats without auth -> 401 ───────────────────────────
    let res = server.get("/api/dashboard/stats").await;
    assert!(
        res.status_code().is_client_error(),
        "GET /api/dashboard/stats without auth should return 401"
    );

    // ── Empty DB -> verify all required fields present ──────────────────
    let res = with_cookie!(
        server.get("/api/dashboard/stats"),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should get dashboard stats"
    );
    let stats: Value = res.json();
    assert!(
        stats.get("totalMembers").is_some(),
        "stats should have totalMembers"
    );
    assert!(
        stats.get("activeMembers").is_some(),
        "stats should have activeMembers"
    );
    assert!(
        stats.get("pendingApprovals").is_some(),
        "stats should have pendingApprovals"
    );
    assert!(
        stats.get("totalCashIn").is_some(),
        "stats should have totalCashIn"
    );
    assert!(
        stats.get("totalCashOut").is_some(),
        "stats should have totalCashOut"
    );
    assert!(
        stats.get("netBalance").is_some(),
        "stats should have netBalance"
    );

    // ── totalMembers counts only MEMBER role users ──────────────────────
    // Admin is already seeded (role=ADMIN), seed a MEMBER user
    let _member = common::seed_member_user(&pool).await;

    let res = with_cookie!(
        server.get("/api/dashboard/stats"),
        &admin_cookie
    )
    .await;
    let stats: Value = res.json();
    let total_members = stats["totalMembers"]
        .as_i64()
        .unwrap_or(0);
    assert_eq!(
        total_members, 1,
        "totalMembers should count only MEMBER role users (admin + 1 member seeded, expect 1), got {}",
        total_members
    );

    // ── activeMembers counts ACTIVE status ──────────────────────────────
    let active_members = stats["activeMembers"]
        .as_i64()
        .unwrap_or(0);
    assert!(
        active_members >= 1,
        "activeMembers should count users with ACTIVE status, got {}",
        active_members
    );

    // ── pendingApprovals increments after operator creates member ────────
    let pending_before = stats["pendingApprovals"]
        .as_i64()
        .unwrap_or(0);

    let res = with_cookie!(
        server.post("/api/members").json(&json!({
            "name": "Pending Stats Member",
            "email": "pending_stats@example.com",
            "phone": "1231231234"
        })),
        &operator_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "operator should create member for pending stats test"
    );

    let res = with_cookie!(
        server.get("/api/dashboard/stats"),
        &admin_cookie
    )
    .await;
    let stats: Value = res.json();
    let pending_after = stats["pendingApprovals"]
        .as_i64()
        .unwrap_or(0);
    assert!(
        pending_after > pending_before,
        "pendingApprovals should increment after operator creates member (before={}, after={})",
        pending_before,
        pending_after
    );

    // ── netBalance reflects CASH_IN and CASH_OUT ────────────────────────
    // Admin creates CASH_IN 5000
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_IN",
            "category": "OTHER",
            "amount": 5000.0,
            "paymentMode": "CASH",
            "purpose": "Dashboard test cash in"
        })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should create CASH_IN 5000"
    );

    // Admin creates CASH_OUT 2000
    let res = with_cookie!(
        server.post("/api/transactions").json(&json!({
            "type": "CASH_OUT",
            "category": "OTHER",
            "amount": 2000.0,
            "paymentMode": "CASH",
            "purpose": "Dashboard test cash out"
        })),
        &admin_cookie
    )
    .await;
    assert!(
        res.status_code().is_success(),
        "admin should create CASH_OUT 2000"
    );

    let res = with_cookie!(
        server.get("/api/dashboard/stats"),
        &admin_cookie
    )
    .await;
    let stats: Value = res.json();

    let total_cash_in = stats["totalCashIn"]
        .as_f64()
        .unwrap_or(0.0);
    assert!(
        total_cash_in >= 5000.0,
        "totalCashIn should be >= 5000.0, got {}",
        total_cash_in
    );

    let total_cash_out = stats["totalCashOut"]
        .as_f64()
        .unwrap_or(0.0);
    assert!(
        total_cash_out >= 2000.0,
        "totalCashOut should be >= 2000.0, got {}",
        total_cash_out
    );

    let net_balance = stats["netBalance"]
        .as_f64()
        .unwrap_or(0.0);
    // netBalance = totalCashIn - totalCashOut >= 5000 - 2000 = 3000
    assert!(
        net_balance >= 3000.0,
        "netBalance should be >= 3000.0 (5000 in - 2000 out), got {}",
        net_balance
    );
}
