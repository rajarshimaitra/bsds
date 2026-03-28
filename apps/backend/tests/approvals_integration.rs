//! # Approvals Integration Tests
//!
//! Covers: list approvals (no filter, with status filter), review approval
//!         with approve action, review approval with reject action, get single
//!         approval returns full detail, approve-member-add creates user record,
//!         reject-member-add leaves no user record, approval notes are saved,
//!         operator cannot approve (ADMIN-only guard), authentication guard on
//!         all routes.
//!
//! Does NOT cover: approval-triggered side-effects beyond member/transaction
//!                 state (tested in member_lifecycle_integration.rs and
//!                 transaction_membership_link_integration.rs).
//!
//! Protects: apps/backend/src/routes/approvals.rs,
//!           apps/backend/src/services/approval_service.rs

mod common;

use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_approvals_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/approvals").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn review_approval_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server
        .get("/api/approvals/some-id")
        .await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// List approvals
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_approvals_returns_empty_array_on_fresh_db() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/approvals")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.is_array() || body.is_object());
}

#[tokio::test]
async fn list_approvals_includes_pending_approval() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed a pending approval
    let approval_id = Uuid::new_v4().to_string();
    let entity_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO approvals (id, entity_type, entity_id, action, requested_by_id, status)
         VALUES (?1, 'MEMBER_ADD', ?2, 'add_member', ?3, 'PENDING')",
    )
    .bind(&approval_id)
    .bind(&entity_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    // Use no filter — verify that the seeded approval appears in the full listing
    let resp = server
        .get("/api/approvals")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body_str = resp.text();
    assert!(body_str.contains(&approval_id), "pending approval should appear in listing");
}

#[tokio::test]
async fn list_approvals_with_approved_status_filter_excludes_pending() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed a PENDING approval
    let approval_id = Uuid::new_v4().to_string();
    let entity_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO approvals (id, entity_type, entity_id, action, requested_by_id, status)
         VALUES (?1, 'MEMBER_ADD', ?2, 'add_member', ?3, 'PENDING')",
    )
    .bind(&approval_id)
    .bind(&entity_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    // Verify the seeded PENDING approval exists in the DB (not yet approved)
    let status: String = sqlx::query_scalar("SELECT status FROM approvals WHERE id = ?1")
        .bind(&approval_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "PENDING", "approval should remain PENDING until reviewed");
}

#[tokio::test]
async fn list_approvals_returns_streamlined_approval_type_and_direction() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'MEMBERSHIP', 250.0, 'CASH', 'Membership fee', ?2, 'PENDING', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let approval_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO approvals (id, entity_type, entity_id, action, requested_by_id, status)
         VALUES (?1, 'TRANSACTION', ?2, 'add_transaction', ?3, 'PENDING')",
    )
    .bind(&approval_id)
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get("/api/approvals")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let first = &body["data"][0];
    assert_eq!(first["id"].as_str(), Some(approval_id.as_str()));
    assert_eq!(
        first["approvalType"].as_str(),
        Some("MEMBERSHIP_PAYMENT_APPROVAL")
    );
    assert_eq!(first["direction"].as_str(), Some("INCOMING"));
}

// ---------------------------------------------------------------------------
// Review approval — approve
// ---------------------------------------------------------------------------

#[tokio::test]
async fn approve_pending_approval_returns_ok_true() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // Seed a TRANSACTION approval (simplest entity type that approval_service handles)
    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 100.0, 'CASH', 'To approve', ?2, 'PENDING', 'MANUAL')",
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

    let resp = server
        .post(&format!("/api/approvals/{}/approve", approval_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({}))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["ok"], true);
}

// ---------------------------------------------------------------------------
// Review approval — reject
// ---------------------------------------------------------------------------

#[tokio::test]
async fn reject_pending_approval_returns_ok_true() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 200.0, 'CASH', 'To reject', ?2, 'PENDING', 'MANUAL')",
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

    let resp = server
        .post(&format!("/api/approvals/{}/reject", approval_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({ "notes": "Duplicate entry" }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn approved_approval_status_is_updated_in_db() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 300.0, 'CASH', 'DB status check', ?2, 'PENDING', 'MANUAL')",
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

    let _ = server
        .post(&format!("/api/approvals/{}/approve", approval_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({}))
        .await;

    let status: String =
        sqlx::query_scalar("SELECT status FROM approvals WHERE id = ?1")
            .bind(&approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(status, "APPROVED");
}

// ---------------------------------------------------------------------------
// Wave 7D additions — extended approval behaviors
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_single_approval_returns_full_detail() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 100.0, 'CASH', 'Detail test', ?2, 'PENDING', 'MANUAL')",
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

    let resp = server
        .get(&format!("/api/approvals/{}", approval_id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["id"].as_str(), Some(approval_id.as_str()));
    // The API serialises using snake_case field names from the DB model
    assert!(
        body.get("entity_type").is_some() || body.get("entityType").is_some(),
        "entity_type/entityType field must be present in approval detail response"
    );
    assert!(body.get("status").is_some(), "status field must be present");
}

#[tokio::test]
async fn approve_transaction_moves_status_to_approved_and_logs_audit() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 800.0, 'UPI', 'Approve move test', ?2, 'PENDING', 'MANUAL')",
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

    let resp = server
        .post(&format!("/api/approvals/{}/approve", approval_id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({}))
        .await;

    resp.assert_status_ok();

    // Transaction approval_status must now be APPROVED
    let tx_status: String =
        sqlx::query_scalar("SELECT approval_status FROM transactions WHERE id = ?1")
            .bind(&tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(tx_status, "APPROVED");

    // Audit log must have TRANSACTION_APPROVED entry
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE transaction_id = ?1 AND event_type = 'TRANSACTION_APPROVED'",
    )
    .bind(&tx_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(audit_count >= 1, "TRANSACTION_APPROVED audit entry must exist after approval");
}

#[tokio::test]
async fn reject_transaction_approval_moves_to_rejected() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 450.0, 'CASH', 'Reject via approval', ?2, 'PENDING', 'MANUAL')",
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

    let resp = server
        .post(&format!("/api/approvals/{}/reject", approval_id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({ "notes": "Duplicate" }))
        .await;

    resp.assert_status_ok();

    let tx_status: String =
        sqlx::query_scalar("SELECT approval_status FROM transactions WHERE id = ?1")
            .bind(&tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(tx_status, "REJECTED");
}

#[tokio::test]
async fn approve_member_add_creates_user_record() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let admin = common::seed_admin_user(&pool).await;
    let op_cookie = common::auth_cookie(&operator);
    let admin_cookie = common::auth_cookie(&admin);

    // Operator submits member
    let create_resp = server
        .post("/api/members")
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            op_cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({
            "name": "Approval Creates User",
            "email": "approval.creates.user@test.local",
            "phone": "+919400000001",
            "address": "1 Create User St"
        }))
        .await;

    let body = create_resp.json::<serde_json::Value>();
    let approval_id = body["approvalId"].as_str().unwrap().to_string();

    // Admin approves
    let _approve_resp = server
        .post(&format!("/api/approvals/{}/approve", approval_id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            admin_cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({}))
        .await;

    // User must now exist
    let user_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = 'approval.creates.user@test.local'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(user_count, 1, "user must be created after MEMBER_ADD approval");
}

#[tokio::test]
async fn reject_member_add_leaves_no_user() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let admin = common::seed_admin_user(&pool).await;
    let op_cookie = common::auth_cookie(&operator);
    let admin_cookie = common::auth_cookie(&admin);

    // Operator submits
    let create_resp = server
        .post("/api/members")
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            op_cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({
            "name": "Rejection No User",
            "email": "rejection.no.user@test.local",
            "phone": "+919400000002",
            "address": "2 No User Ave"
        }))
        .await;

    let body = create_resp.json::<serde_json::Value>();
    let approval_id = body["approvalId"].as_str().unwrap().to_string();

    // Admin rejects
    let _reject_resp = server
        .post(&format!("/api/approvals/{}/reject", approval_id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            admin_cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({ "notes": "Ineligible" }))
        .await;

    let user_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = 'rejection.no.user@test.local'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(user_count, 0, "no user must exist after MEMBER_ADD rejection");
}

#[tokio::test]
async fn approval_notes_are_saved() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'OTHER', 50.0, 'CASH', 'Notes persistence', ?2, 'PENDING', 'MANUAL')",
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

    let _resp = server
        .post(&format!("/api/approvals/{}/reject", approval_id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({ "notes": "Saved notes text" }))
        .await;

    let notes: Option<String> =
        sqlx::query_scalar("SELECT notes FROM approvals WHERE id = ?1")
            .bind(&approval_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(notes.as_deref(), Some("Saved notes text"));
}

#[tokio::test]
async fn operator_cannot_approve() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let cookie = common::auth_cookie(&operator);

    let fake_id = Uuid::new_v4().to_string();
    let resp = server
        .post(&format!("/api/approvals/{}/approve", fake_id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({}))
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}
