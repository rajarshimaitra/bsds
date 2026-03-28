//! # Receipts Integration Tests
//!
//! Covers: get receipt by id (happy path), get non-existent receipt (404),
//!         authentication guard.
//!
//! Does NOT cover: receipt PDF rendering (not in scope), receipt generation
//!                 triggered by transaction approval (side-effect tested
//!                 implicitly via transactions + approval flow).
//!
//! Protects: apps/backend/src/routes/receipts.rs

mod common;

use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_receipt_without_auth_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let resp = server.get("/api/receipts/some-id").await;
    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Get receipt
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_nonexistent_receipt_returns_404() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let resp = server
        .get("/api/receipts/00000000-0000-0000-0000-000000000000")
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_receipt_by_id_returns_correct_receipt() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    // We need a transaction first (receipt FK)
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

    let receipt_id = Uuid::new_v4().to_string();
    let receipt_number = format!("BSDS-REC-2026-0001");
    sqlx::query(
        "INSERT INTO receipts (id, transaction_id, receipt_number, issued_by_id, status, type,
                               amount, payment_mode, category, purpose, received_by, club_name, club_address)
         VALUES (?1, ?2, ?3, ?4, 'ACTIVE', 'MEMBER', 3000.0, 'UPI', 'MEMBERSHIP', 'Annual membership',
                 'Test Admin', 'Test Club', 'Test Address')",
    )
    .bind(&receipt_id)
    .bind(&tx_id)
    .bind(&receipt_number)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get(&format!("/api/receipts/{}", receipt_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["id"], receipt_id.as_str());
    assert_eq!(body["receipt_number"], receipt_number.as_str());
    assert_eq!(body["status"], "ACTIVE");
    assert_eq!(body["type"], "MEMBER");
    assert_eq!(body["amount"], 3000.0);
}

#[tokio::test]
async fn get_receipt_returns_sponsor_type_receipt() {
    let (server, pool, _dir) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let cookie = common::auth_cookie(&admin);

    let sp_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sponsors (id, name, phone, email, created_by_id) VALUES (?1, 'Sponsor X', '+91', 'sx@test.local', ?2)")
        .bind(&sp_id)
        .bind(&admin.id)
        .execute(&pool)
        .await
        .unwrap();

    let tx_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, sponsor_id, entered_by_id, approval_status, approval_source)
         VALUES (?1, 'CASH_IN', 'SPONSORSHIP', 50000.0, 'UPI', 'Title sponsorship', ?2, ?3, 'APPROVED', 'MANUAL')",
    )
    .bind(&tx_id)
    .bind(&sp_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let receipt_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO receipts (id, transaction_id, receipt_number, issued_by_id, status, type,
                               sponsor_name, amount, payment_mode, category, purpose, received_by, club_name, club_address)
         VALUES (?1, ?2, 'BSDS-REC-2026-0002', ?3, 'ACTIVE', 'SPONSOR', 'Sponsor X',
                 50000.0, 'UPI', 'SPONSORSHIP', 'Title sponsorship', 'Test Admin', 'Test Club', 'Test Address')",
    )
    .bind(&receipt_id)
    .bind(&tx_id)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let resp = server
        .get(&format!("/api/receipts/{}", receipt_id))
        .add_header(axum::http::HeaderName::from_static("cookie"), cookie.parse().unwrap())
        .await;

    resp.assert_status_ok();
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["type"], "SPONSOR");
    assert_eq!(body["sponsor_name"], "Sponsor X");
}
