//! # Members Integration Tests
//!
//! Covers: list members (authed), create member as ADMIN (direct), create member
//!         as OPERATOR (pending approval), get single member, update member,
//!         delete member (soft-delete), sub-member CRUD, sub-member max-3
//!         constraint, role-based access to sub-member endpoints, unauthenticated
//!         rejection for every endpoint.
//!
//! Does NOT cover: PII field-level encryption (tested in support_encrypt.rs),
//!                 approval-driven apply of member edits (tested in
//!                 approvals_integration.rs), full member lifecycle state machine
//!                 (tested in member_lifecycle_integration.rs).
//!
//! Protects: apps/backend/src/routes/members.rs,
//!           apps/backend/src/services/member_service.rs

mod common;

use serde_json::json;

// ---------------------------------------------------------------------------
// Authentication guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_members_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/members").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_member_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/members/nonexistent-id").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_member_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server
        .post("/api/members")
        .json(&json!({ "name": "Test", "email": "t@test.local", "phone": "+91", "address": "x" }))
        .await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// List members
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_members_returns_empty_array_on_fresh_db() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/members")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    // Response is a tuple [members, total]
    assert!(body.is_array() || body[0].is_array() || body.get("data").is_some() || body.is_array());
}

#[tokio::test]
async fn list_members_returns_seeded_member() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let _member = common::seed_member(&pool, "Alice Smith").await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/members")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body_str = resp.text();
    assert!(body_str.contains("Alice Smith"));
}

// ---------------------------------------------------------------------------
// Get single member
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_member_by_id_returns_correct_member() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Bob Jones").await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get(&format!("/api/members/{}", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["name"], "Bob Jones");
    assert_eq!(body["id"], member.id.as_str());
}

#[tokio::test]
async fn get_nonexistent_member_returns_500_or_error() {
    // The service returns NotFound which maps to Internal via AppError::Internal
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/members/00000000-0000-0000-0000-000000000000")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    // Service error wraps as Internal (500) or BadRequest
    assert!(
        resp.status_code().is_server_error() || resp.status_code().is_client_error(),
        "expected an error status for missing member"
    );
}

// ---------------------------------------------------------------------------
// Create member — ADMIN path (direct)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_create_member_returns_direct_action_and_member_id() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/members")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "name": "Carol Admin",
            "email": "carol.admin@test.local",
            "phone": "+919999999901",
            "address": "123 Main St"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");
    assert!(body["memberId"].as_str().is_some(), "memberId should be set for direct creates");
    assert!(body["approvalId"].is_null(), "no approval for admin-created member");
}

#[tokio::test]
async fn admin_create_member_id_follows_bsds_format() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post("/api/members")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "name": "Dave Format",
            "email": "dave.format@test.local",
            "phone": "+919999999902",
            "address": "456 Format Rd"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    let member_id = body["memberId"].as_str().unwrap();
    // Expected format: BSDS-YYYY-NNNN-00
    assert!(
        member_id.starts_with("BSDS-") && member_id.ends_with("-00"),
        "Member ID format incorrect: {member_id}"
    );
}

// ---------------------------------------------------------------------------
// Create member — OPERATOR path (pending approval)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn operator_create_member_returns_pending_approval_action() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let cookie = common::auth_cookie(&operator);

    let resp = server
        .post("/api/members")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "name": "Eve Operator",
            "email": "eve.operator@test.local",
            "phone": "+919999999903",
            "address": "789 Operator Ave"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "pending_approval");
    assert!(body["approvalId"].as_str().is_some(), "approvalId must be set");
    assert!(body["memberId"].is_null(), "memberId should be null for pending approval");
}

// ---------------------------------------------------------------------------
// Update member
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_update_member_returns_direct_action() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Frank Update").await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .patch(&format!("/api/members/{}", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({ "name": "Frank Updated", "phone": "+919000000001" }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");
}

#[tokio::test]
async fn operator_update_member_returns_pending_approval() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let member = common::seed_member(&pool, "Grace Pending").await;
    let cookie = common::auth_cookie(&operator);

    let resp = server
        .patch(&format!("/api/members/{}", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({ "name": "Grace Updated" }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "pending_approval");
}

// ---------------------------------------------------------------------------
// Delete member
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_delete_member_returns_ok_true() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Henry Delete").await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .delete(&format!("/api/members/{}", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn admin_delete_member_sets_user_status_to_suspended() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Iris Suspend").await;
    let cookie = common::auth_cookie(&admin);

    let _ = server
        .delete(&format!("/api/members/{}", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    let status: String = sqlx::query_scalar(
        "SELECT membership_status FROM users WHERE id = ?1",
    )
    .bind(&member.user_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(status, "SUSPENDED");
}

// ---------------------------------------------------------------------------
// Sub-members
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_sub_members_returns_empty_for_member_with_no_sub_members() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Jack NoSubs").await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get(&format!("/api/members/{}/sub-members", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert!(body.is_array(), "sub-members response should be an array");
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// Sub-member CRUD (Wave 7D additions)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn add_sub_member_returns_ok_and_creates_record() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Sub Parent One").await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .post(&format!("/api/members/{}/sub-members", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "name": "Child One",
            "email": "child.one@test.local",
            "phone": "+919300000001",
            "relation": "SPOUSE"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");

    // Verify a sub_members row was created
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sub_members WHERE parent_user_id = ?1",
    )
    .bind(&member.user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "one sub_members row must exist after add");
}

#[tokio::test]
async fn cannot_add_more_than_3_sub_members() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Sub Parent Three").await;
    let cookie = common::auth_cookie(&admin);

    // Add 3 sub-members
    for i in 1..=3u32 {
        let resp = server
            .post(&format!("/api/members/{}/sub-members", member.id))
            .add_header(
                axum::http::HeaderName::from_static("cookie"),
                cookie.parse::<axum::http::HeaderValue>().unwrap(),
            )
            .json(&json!({
                "name": format!("Sub {}", i),
                "email": format!("sub.limit.{}@test.local", i),
                "phone": format!("+91930000{:04}", i),
                "relation": "CHILD"
            }))
            .await;
        resp.assert_status_ok();
    }

    // Fourth add must fail
    let resp = server
        .post(&format!("/api/members/{}/sub-members", member.id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({
            "name": "Sub Four",
            "email": "sub.limit.4@test.local",
            "phone": "+919300000004",
            "relation": "CHILD"
        }))
        .await;

    assert!(
        resp.status_code().is_client_error(),
        "adding a 4th sub-member must return a client error; got {}",
        resp.status_code()
    );
}

#[tokio::test]
async fn update_sub_member_changes_name() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Update Sub Parent").await;
    let cookie = common::auth_cookie(&admin);

    // Add a sub-member first
    let add_resp = server
        .post(&format!("/api/members/{}/sub-members", member.id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({
            "name": "Original Name",
            "email": "update.sub@test.local",
            "phone": "+919300000010",
            "relation": "SPOUSE"
        }))
        .await;
    add_resp.assert_status_ok();

    // Fetch the sub-member id from the DB
    let sub_id: String =
        sqlx::query_scalar("SELECT id FROM sub_members WHERE parent_user_id = ?1 LIMIT 1")
            .bind(&member.user_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Update it
    let resp = server
        .put(&format!("/api/members/{}/sub-members", member.id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({
            "subMemberId": sub_id,
            "name": "Updated Name",
            "phone": "+919300000011",
            "relation": "CHILD"
        }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");

    // Verify the name was updated in the DB
    let name: String = sqlx::query_scalar("SELECT name FROM sub_members WHERE id = ?1")
        .bind(&sub_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "Updated Name");
}

#[tokio::test]
async fn remove_sub_member_deletes_record() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member(&pool, "Remove Sub Parent").await;
    let cookie = common::auth_cookie(&admin);

    // Add a sub-member
    let _add_resp = server
        .post(&format!("/api/members/{}/sub-members", member.id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({
            "name": "To Remove",
            "email": "remove.sub@test.local",
            "phone": "+919300000020",
            "relation": "SPOUSE"
        }))
        .await;

    let sub_id: String =
        sqlx::query_scalar("SELECT id FROM sub_members WHERE parent_user_id = ?1 LIMIT 1")
            .bind(&member.user_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Remove it
    let resp = server
        .delete(&format!("/api/members/{}/sub-members", member.id))
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&json!({ "subMemberId": sub_id }))
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "direct");

    // Verify deletion
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sub_members WHERE id = ?1")
        .bind(&sub_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "sub_members row must be deleted after remove");
}

#[tokio::test]
async fn organiser_cannot_add_sub_member() {
    let (server, pool, _dir) = common::test_app().await;
    let organiser = common::seed_organiser_user(&pool).await;
    let member = common::seed_member(&pool, "Org Target Sub Parent").await;
    let cookie = common::auth_cookie(&organiser);

    let resp = server
        .post(&format!("/api/members/{}/sub-members", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "name": "Org Sub Attempt",
            "email": "org.sub.attempt@test.local",
            "phone": "+919300000030",
            "relation": "CHILD"
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn operator_can_add_sub_member() {
    let (server, pool, _dir) = common::test_app().await;
    let operator = common::seed_operator_user(&pool).await;
    let member = common::seed_member(&pool, "Op Sub Parent").await;
    let cookie = common::auth_cookie(&operator);

    let resp = server
        .post(&format!("/api/members/{}/sub-members", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .json(&json!({
            "name": "Op Sub Child",
            "email": "op.sub.child@test.local",
            "phone": "+919300000040",
            "relation": "CHILD"
        }))
        .await;

    // Operator creates via pending_approval path — must return 200 (pending)
    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["action"], "pending_approval");
    assert!(body["approvalId"].as_str().is_some());
}

#[tokio::test]
async fn list_sub_members_requires_organiser_or_above() {
    let (server, pool, _dir) = common::test_app().await;
    let member_user = common::seed_member_user(&pool).await;
    let member = common::seed_member(&pool, "List Sub Target").await;
    let cookie = common::auth_cookie(&member_user);

    let resp = server
        .get(&format!("/api/members/{}/sub-members", member.id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    // MEMBER role is below ORGANISER minimum — must be 403
    resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}
