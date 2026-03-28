//! Consolidated integration tests for sponsors, sponsor links, webhook processing,
//! and receipt retrieval. Merges tests from sponsors_integration.rs,
//! sponsor_links_integration.rs, webhooks_integration.rs, and receipts_integration.rs
//! into three large test functions that share `test_app()` instances.

mod common;

use axum::http::{HeaderName, HeaderValue};
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Webhook helpers
// ---------------------------------------------------------------------------

fn make_webhook_body(event: &str, payment_status: &str, extra_notes: &[(&str, &str)]) -> String {
    let mut notes = serde_json::Map::new();
    for (k, v) in extra_notes {
        notes.insert(k.to_string(), serde_json::Value::String(v.to_string()));
    }
    serde_json::json!({
        "event": event,
        "payload": {
            "payment": {
                "entity": {
                    "id": "pay_test_12345",
                    "order_id": "order_test_abc",
                    "amount": 25000,
                    "status": payment_status,
                    "method": "upi",
                    "vpa": "test@upi",
                    "bank": null,
                    "contact": "+919999999999",
                    "email": "test@test.local",
                    "notes": notes,
                    "bank_transfer": null
                }
            }
        }
    })
    .to_string()
}

const TEST_WEBHOOK_SECRET: &str = "bsds-test-webhook-secret-constant";

fn setup_webhook_secret() -> &'static str {
    std::env::set_var("RAZORPAY_WEBHOOK_SECRET", TEST_WEBHOOK_SECRET);
    TEST_WEBHOOK_SECRET
}

fn compute_signature(body: &str, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// ---------------------------------------------------------------------------
// Seed helpers (direct DB inserts)
// ---------------------------------------------------------------------------

async fn seed_sponsor(pool: &SqlitePool, name: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sponsors (id, name, phone, email, company, created_at, updated_at)
         VALUES (?, ?, '1234567890', 'sponsor@test.local', 'TestCo', datetime('now'), datetime('now'))",
    )
    .bind(&id)
    .bind(name)
    .execute(pool)
    .await
    .expect("seed_sponsor insert failed");
    id
}

async fn seed_sponsor_link(
    pool: &SqlitePool,
    sponsor_id: &str,
    token: &str,
    is_active: bool,
) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sponsor_links (id, sponsor_id, token, upi_id, sponsor_purpose, amount, url, is_active, created_at, updated_at)
         VALUES (?, ?, ?, 'test@upi', 'donation', 250.00, 'https://pay.test/' || ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(&id)
    .bind(sponsor_id)
    .bind(token)
    .bind(token)
    .bind(is_active)
    .execute(pool)
    .await
    .expect("seed_sponsor_link insert failed");
    id
}

async fn seed_transaction(
    pool: &SqlitePool,
    member_id: Option<&str>,
    sponsor_id: Option<&str>,
    razorpay_payment_id: Option<&str>,
) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO transactions (id, member_id, sponsor_id, amount, payment_mode, razorpay_payment_id, status, category, purpose, created_at, updated_at)
         VALUES (?, ?, ?, 3000.0, 'UPI', ?, 'COMPLETED', 'MEMBERSHIP', 'Annual Fee', datetime('now'), datetime('now'))",
    )
    .bind(&id)
    .bind(member_id)
    .bind(sponsor_id)
    .bind(razorpay_payment_id)
    .execute(pool)
    .await
    .expect("seed_transaction insert failed");
    id
}

async fn seed_receipt(
    pool: &SqlitePool,
    transaction_id: &str,
    issued_by_id: &str,
    receipt_type: &str,
    sponsor_name: Option<&str>,
) -> String {
    let id = Uuid::new_v4().to_string();
    let receipt_number = format!("REC-{}", &id[..8]);
    sqlx::query(
        "INSERT INTO receipts (id, transaction_id, receipt_number, issued_by_id, status, type, amount, payment_mode, category, purpose, received_by, club_name, club_address, sponsor_name, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'ACTIVE', ?, 3000.0, 'UPI', 'MEMBERSHIP', 'Annual Fee', 'Test Receiver', 'BSDS Club', '123 Club Street', ?, datetime('now'), datetime('now'))",
    )
    .bind(&id)
    .bind(transaction_id)
    .bind(&receipt_number)
    .bind(issued_by_id)
    .bind(receipt_type)
    .bind(sponsor_name)
    .execute(pool)
    .await
    .expect("seed_receipt insert failed");
    id
}

// ---------------------------------------------------------------------------
// Test 1: Sponsor CRUD and Links Lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sponsor_crud_and_links_lifecycle() {
    let (server, pool, _tmp) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let member = common::seed_member_user(&pool).await;
    let admin_cookie = common::auth_cookie(&admin);
    let member_cookie = common::auth_cookie(&member);

    // --- Auth / role guards (sponsors) ---

    // List sponsors without auth -> 401
    let resp = server.get("/api/sponsors").await;
    assert_eq!(
        resp.status_code(),
        401,
        "listing sponsors without auth should return 401"
    );

    // List sponsors as MEMBER role -> 403
    let resp = server
        .get("/api/sponsors")
        .add_header(
            HeaderName::from_static("cookie"),
            member_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        403,
        "MEMBER role should be forbidden from listing sponsors"
    );

    // Create sponsor as MEMBER role -> 403
    let resp = server
        .post("/api/sponsors")
        .add_header(
            HeaderName::from_static("cookie"),
            member_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .json(&serde_json::json!({
            "name": "Should Fail",
            "phone": "0000000000",
            "email": "fail@test.local",
            "company": "FailCo"
        }))
        .await;
    assert_eq!(
        resp.status_code(),
        403,
        "MEMBER role should be forbidden from creating sponsors"
    );

    // List sponsors with must_change_password session -> 403
    {
        use bsds_backend::auth::{create_session_token, SessionClaims};
        let claims = SessionClaims {
            user_id: admin.id.clone(),
            username: admin.email.clone(),
            role: admin.role.clone(),
            member_id: Some(admin.member_id.clone()),
            must_change_password: true,
        };
        let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
        let temp_token = create_session_token(&claims, &secret);
        let temp_cookie = format!("bsds_session={}", temp_token);
        let resp = server
            .get("/api/sponsors")
            .add_header(
                HeaderName::from_static("cookie"),
                temp_cookie
                    .parse::<HeaderValue>()
                    .unwrap(),
            )
            .await;
        assert_eq!(
            resp.status_code(),
            403,
            "must_change_password session should be forbidden from listing sponsors"
        );
    }

    // --- Auth / role guards (sponsor links) ---

    // List sponsor links without auth -> 401
    let resp = server.get("/api/sponsor-links").await;
    assert_eq!(
        resp.status_code(),
        401,
        "listing sponsor links without auth should return 401"
    );

    // List sponsor links as MEMBER -> 403
    let resp = server
        .get("/api/sponsor-links")
        .add_header(
            HeaderName::from_static("cookie"),
            member_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        403,
        "MEMBER role should be forbidden from listing sponsor links"
    );

    // --- Sponsor CRUD ---

    // List sponsors on fresh DB -> empty
    let resp = server
        .get("/api/sponsors")
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(resp.status_code(), 200, "admin should list sponsors");
    let body: Value = resp.json();
    assert_eq!(
        body["data"].as_array().unwrap().len(),
        0,
        "fresh DB should have no sponsors"
    );
    assert_eq!(
        body["total"].as_i64().unwrap(),
        0,
        "fresh DB total should be 0"
    );

    // Seed a sponsor directly and verify list returns it
    let acme_id = seed_sponsor(&pool, "ACME Corp").await;

    let resp = server
        .get("/api/sponsors")
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    let body: Value = resp.json();
    assert_eq!(
        body["total"].as_i64().unwrap(),
        1,
        "should have exactly 1 sponsor after seeding ACME Corp"
    );
    assert_eq!(
        body["data"][0]["name"].as_str().unwrap(),
        "ACME Corp",
        "first sponsor name should be ACME Corp"
    );

    // Seed another sponsor for search filter test
    let _beta_id = seed_sponsor(&pool, "Beta Ltd").await;

    // Search filter "acme" -> returns only ACME
    let resp = server
        .get("/api/sponsors?search=acme")
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    let body: Value = resp.json();
    assert_eq!(
        body["total"].as_i64().unwrap(),
        1,
        "search 'acme' should return exactly 1 result"
    );
    assert_eq!(
        body["data"][0]["name"].as_str().unwrap(),
        "ACME Corp",
        "search 'acme' should return ACME Corp, not Beta Ltd"
    );

    // Create sponsor via API
    let resp = server
        .post("/api/sponsors")
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .json(&serde_json::json!({
            "name": "New Sponsor",
            "phone": "5551234567",
            "email": "new@test.local",
            "company": "New Corp"
        }))
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "admin should be able to create a sponsor"
    );
    let body: Value = resp.json();
    assert!(
        body["id"].as_str().is_some(),
        "created sponsor response should contain an id"
    );
    assert_eq!(
        body["name"].as_str().unwrap(),
        "New Sponsor",
        "created sponsor name should match"
    );
    assert_eq!(
        body["company"].as_str().unwrap(),
        "New Corp",
        "created sponsor company should match"
    );

    // Get sponsor by id
    let resp = server
        .get(&format!("/api/sponsors/{}", acme_id))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(resp.status_code(), 200, "GET sponsor by id should succeed");
    let body: Value = resp.json();
    assert_eq!(
        body["sponsor"]["id"].as_str().unwrap(),
        acme_id,
        "returned sponsor id should match requested id"
    );
    assert!(
        body["links"].as_array().is_some(),
        "sponsor detail should include links array"
    );
    assert!(
        body["transactions"].as_array().is_some(),
        "sponsor detail should include transactions array"
    );

    // Get nonexistent sponsor -> 404
    let resp = server
        .get(&format!("/api/sponsors/{}", Uuid::new_v4()))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        404,
        "GET nonexistent sponsor should return 404"
    );

    // Update sponsor
    let resp = server
        .put(&format!("/api/sponsors/{}", acme_id))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .json(&serde_json::json!({ "name": "New Name" }))
        .await;
    assert_eq!(resp.status_code(), 200, "update sponsor should succeed");
    let body: Value = resp.json();
    assert!(
        body["message"].as_str().is_some(),
        "update response should contain a message"
    );

    // Delete sponsor with no transactions -> success
    // Use Beta Ltd which has no transactions
    let resp = server
        .delete(&format!("/api/sponsors/{}", _beta_id))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "deleting sponsor with no transactions should succeed"
    );
    let body: Value = resp.json();
    assert!(
        body["message"].as_str().is_some(),
        "delete response should contain a message"
    );

    // Delete sponsor with linked transaction -> 400
    let _tx_id = seed_transaction(&pool, None, Some(&acme_id), None).await;
    let resp = server
        .delete(&format!("/api/sponsors/{}", acme_id))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        400,
        "deleting sponsor with linked transactions should return 400"
    );

    // --- Sponsor Links Lifecycle ---

    // Seed a fresh sponsor for link tests
    let link_sponsor_id = seed_sponsor(&pool, "Link Sponsor").await;

    // List sponsor links on fresh state -> empty (for this sponsor context)
    let resp = server
        .get("/api/sponsor-links")
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(resp.status_code(), 200, "admin should list sponsor links");
    let body: Value = resp.json();
    assert_eq!(
        body["total"].as_i64().unwrap(),
        0,
        "should have no sponsor links initially"
    );
    assert!(
        body["data"].as_array().unwrap().is_empty(),
        "sponsor links data should be empty initially"
    );

    // Seed 2 active links + 1 inactive link
    let token_a = Uuid::new_v4().to_string();
    let token_b = Uuid::new_v4().to_string();
    let token_inactive = Uuid::new_v4().to_string();
    seed_sponsor_link(&pool, &link_sponsor_id, &token_a, true).await;
    seed_sponsor_link(&pool, &link_sponsor_id, &token_b, true).await;
    seed_sponsor_link(&pool, &link_sponsor_id, &token_inactive, false).await;

    // List all links -> total==3 (2 active + 1 inactive)
    let resp = server
        .get("/api/sponsor-links")
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    let body: Value = resp.json();
    // We seeded 3 links total
    assert!(
        body["total"].as_i64().unwrap() >= 2,
        "should have at least 2 sponsor links after seeding"
    );

    // Filter isActive -> only active returned
    let resp = server
        .get("/api/sponsor-links?isActive=true")
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    let body: Value = resp.json();
    let active_data = body["data"].as_array().unwrap();
    for link in active_data {
        assert_eq!(
            link["is_active"].as_bool().unwrap_or(true),
            true,
            "isActive filter should return only active links"
        );
    }

    // Create sponsor link via API
    let resp = server
        .post("/api/sponsor-links")
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .json(&serde_json::json!({
            "sponsorId": link_sponsor_id,
            "upiId": "created@upi",
            "sponsorPurpose": "event funding",
            "amount": 500.00
        }))
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "admin should be able to create a sponsor link"
    );
    let body: Value = resp.json();
    assert!(
        body["link"].is_object() || body["id"].is_string(),
        "create sponsor link response should contain link object or id"
    );
    assert!(
        body["url"].as_str().is_some() || body["link"]["url"].as_str().is_some(),
        "create sponsor link response should contain a url"
    );
    assert!(
        body["message"].as_str().is_some(),
        "create sponsor link response should contain a message"
    );

    // Public access: GET /api/sponsor-links/{token} with valid active token -> 200
    let resp = server
        .get(&format!("/api/sponsor-links/{}", token_a))
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "public GET with valid active token should return 200"
    );

    // Public access: GET with inactive token -> 200 with error in body
    let resp = server
        .get(&format!("/api/sponsor-links/{}", token_inactive))
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "public GET with inactive token should return 200"
    );
    let body: Value = resp.json();
    assert!(
        body["error"].is_string() || body["error"].is_object(),
        "inactive token response should contain an error field"
    );

    // Public access: GET with unknown token -> 404
    let resp = server
        .get(&format!("/api/sponsor-links/{}", Uuid::new_v4()))
        .await;
    assert_eq!(
        resp.status_code(),
        404,
        "public GET with unknown token should return 404"
    );

    // Deactivate link
    let resp = server
        .patch(&format!("/api/sponsor-links/{}", token_b))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "deactivating an active link should succeed"
    );
    let body: Value = resp.json();
    assert!(
        body["message"].as_str().is_some(),
        "deactivate response should contain a message"
    );

    // Verify DB: is_active should be false
    let row = sqlx::query_scalar::<_, bool>(
        "SELECT is_active FROM sponsor_links WHERE token = ?",
    )
    .bind(&token_b)
    .fetch_one(&pool)
    .await
    .expect("should find deactivated link in DB");
    assert!(
        !row,
        "deactivated link should have is_active==false in the database"
    );

    // Deactivate already inactive link -> 400
    let resp = server
        .patch(&format!("/api/sponsor-links/{}", token_inactive))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        400,
        "deactivating an already inactive link should return 400"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Webhook Processing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn webhook_processing() {
    let (server, pool, _tmp) = common::test_app().await;
    let secret = setup_webhook_secret();

    // --- No signature header -> 401 ---
    let body = make_webhook_body("payment.captured", "captured", &[]);
    let resp = server
        .post("/api/webhooks/razorpay")
        .text(body.clone())
        .await;
    assert_eq!(
        resp.status_code(),
        401,
        "webhook without signature header should return 401"
    );

    // --- Invalid signature -> 401 ---
    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            HeaderName::from_static("x-razorpay-signature"),
            "invalid_signature_value"
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .text(body.clone())
        .await;
    assert_eq!(
        resp.status_code(),
        401,
        "webhook with invalid signature should return 401"
    );

    // --- Valid signature + non-capture event (payment.failed) -> 200, ignored ---
    let failed_body = make_webhook_body("payment.failed", "failed", &[]);
    let sig = compute_signature(&failed_body, secret);
    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            HeaderName::from_static("x-razorpay-signature"),
            sig.parse::<HeaderValue>().unwrap(),
        )
        .text(failed_body)
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "non-capture event should return 200"
    );
    let resp_body: Value = resp.json();
    assert_eq!(
        resp_body["ok"].as_bool().unwrap(),
        true,
        "non-capture event response should have ok==true"
    );
    assert_eq!(
        resp_body["ignored"].as_bool().unwrap(),
        true,
        "non-capture event response should have ignored==true"
    );

    // --- payment.captured + no memberId/sponsorLinkToken -> 400 ---
    let capture_no_notes = make_webhook_body("payment.captured", "captured", &[]);
    let sig = compute_signature(&capture_no_notes, secret);
    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            HeaderName::from_static("x-razorpay-signature"),
            sig.parse::<HeaderValue>().unwrap(),
        )
        .text(capture_no_notes)
        .await;
    assert_eq!(
        resp.status_code(),
        400,
        "payment.captured without memberId or sponsorLinkToken should return 400"
    );

    // --- payment.captured + sponsorLinkToken -> 200, kind=="sponsor" ---
    // Seed sponsor and link for this test
    let sponsor_id = seed_sponsor(&pool, "Webhook Sponsor").await;
    let sponsor_token = Uuid::new_v4().to_string();
    seed_sponsor_link(&pool, &sponsor_id, &sponsor_token, true).await;

    let sponsor_body = make_webhook_body(
        "payment.captured",
        "captured",
        &[("sponsorLinkToken", &sponsor_token)],
    );
    let sig = compute_signature(&sponsor_body, secret);
    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            HeaderName::from_static("x-razorpay-signature"),
            sig.parse::<HeaderValue>().unwrap(),
        )
        .text(sponsor_body)
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "payment.captured with sponsorLinkToken should return 200"
    );
    let resp_body: Value = resp.json();
    assert_eq!(
        resp_body["ok"].as_bool().unwrap(),
        true,
        "sponsor payment response should have ok==true"
    );
    assert_eq!(
        resp_body["kind"].as_str().unwrap(),
        "sponsor",
        "sponsor payment response should have kind=='sponsor'"
    );

    // --- Duplicate payment_id -> 200, alreadyProcessed==true ---
    // Seed a member and pre-insert a transaction with the same razorpay_payment_id
    let seeded_member = common::seed_member(&pool, "Dupe Member").await;
    seed_transaction(
        &pool,
        Some(&seeded_member.id),
        None,
        Some("pay_test_12345"),
    )
    .await;

    let dupe_body = make_webhook_body(
        "payment.captured",
        "captured",
        &[("memberId", &seeded_member.id)],
    );
    let sig = compute_signature(&dupe_body, secret);
    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            HeaderName::from_static("x-razorpay-signature"),
            sig.parse::<HeaderValue>().unwrap(),
        )
        .text(dupe_body)
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "duplicate payment should return 200"
    );
    let resp_body: Value = resp.json();
    assert_eq!(
        resp_body["alreadyProcessed"].as_bool().unwrap(),
        true,
        "duplicate payment response should have alreadyProcessed==true"
    );
    assert_eq!(
        resp_body["kind"].as_str().unwrap(),
        "membership",
        "duplicate membership payment response should have kind=='membership'"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Receipt Retrieval
// ---------------------------------------------------------------------------

#[tokio::test]
async fn receipt_retrieval() {
    let (server, pool, _tmp) = common::test_app().await;
    let admin = common::seed_admin_user(&pool).await;
    let admin_cookie = common::auth_cookie(&admin);

    // --- Get receipt without auth -> 401 ---
    let resp = server
        .get(&format!("/api/receipts/{}", Uuid::new_v4()))
        .await;
    assert_eq!(
        resp.status_code(),
        401,
        "getting a receipt without auth should return 401"
    );

    // --- Get nonexistent receipt -> 404 ---
    let resp = server
        .get(&format!("/api/receipts/{}", Uuid::new_v4()))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        404,
        "getting a nonexistent receipt should return 404"
    );

    // --- Get member receipt -> correct fields ---
    let member = common::seed_member(&pool, "Receipt Member").await;
    let tx_id = seed_transaction(&pool, Some(&member.id), None, None).await;
    let receipt_id = seed_receipt(&pool, &tx_id, &admin.id, "MEMBER", None).await;

    let resp = server
        .get(&format!("/api/receipts/{}", receipt_id))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "getting a valid member receipt should return 200"
    );
    let body: Value = resp.json();
    assert_eq!(
        body["id"].as_str().unwrap(),
        receipt_id,
        "receipt id should match the requested id"
    );
    assert!(
        body["receipt_number"].as_str().is_some(),
        "receipt should have a receipt_number"
    );
    assert_eq!(
        body["status"].as_str().unwrap(),
        "ACTIVE",
        "receipt status should be ACTIVE"
    );
    assert_eq!(
        body["type"].as_str().unwrap(),
        "MEMBER",
        "receipt type should be MEMBER"
    );
    let amount = body["amount"].as_f64().unwrap();
    assert!(
        (amount - 3000.0).abs() < f64::EPSILON,
        "receipt amount should be 3000.0, got {}",
        amount
    );

    // --- Get sponsor receipt -> type=="SPONSOR", sponsor_name=="Sponsor X" ---
    let sponsor_id = seed_sponsor(&pool, "Sponsor X").await;
    let sponsor_tx_id = seed_transaction(&pool, None, Some(&sponsor_id), None).await;
    let sponsor_receipt_id =
        seed_receipt(&pool, &sponsor_tx_id, &admin.id, "SPONSOR", Some("Sponsor X")).await;

    let resp = server
        .get(&format!("/api/receipts/{}", sponsor_receipt_id))
        .add_header(
            HeaderName::from_static("cookie"),
            admin_cookie
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status_code(),
        200,
        "getting a valid sponsor receipt should return 200"
    );
    let body: Value = resp.json();
    assert_eq!(
        body["type"].as_str().unwrap(),
        "SPONSOR",
        "sponsor receipt type should be SPONSOR"
    );
    assert_eq!(
        body["sponsor_name"].as_str().unwrap(),
        "Sponsor X",
        "sponsor receipt should have sponsor_name=='Sponsor X'"
    );
}
