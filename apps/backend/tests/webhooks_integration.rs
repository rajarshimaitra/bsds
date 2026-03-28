//! # Webhooks Integration Tests
//!
//! Covers: Razorpay webhook missing signature returns 401, tampered/invalid
//!         signature returns 401, non-capture events are acknowledged and
//!         ignored, duplicate payment idempotency, sponsor payment routing
//!         (when sponsorLinkToken is present in notes).
//!
//! Does NOT cover: actual HMAC signature generation against a real Razorpay
//!                 secret (would require env-var injection for live testing;
//!                 we test the guard at the HTTP level), WhatsApp notification
//!                 side-effects.
//!
//! Protects: apps/backend/src/routes/webhooks.rs

mod common;

// ---------------------------------------------------------------------------
// Helper to build a minimal Razorpay webhook body
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

/// Single constant secret used by all webhook tests.
/// All tests use the same value so concurrent set_var/remove_var calls don't
/// cause HMAC mismatches between tests running in parallel threads.
const TEST_WEBHOOK_SECRET: &str = "bsds-test-webhook-secret-constant";

/// Set the webhook secret env var and return it.  Every test that sends a
/// signed payload must call this at the top.
fn setup_webhook_secret() -> &'static str {
    std::env::set_var("RAZORPAY_WEBHOOK_SECRET", TEST_WEBHOOK_SECRET);
    TEST_WEBHOOK_SECRET
}

/// Compute the correct HMAC-SHA256 hex signature for the given body and secret.
fn compute_signature(body: &str, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// ---------------------------------------------------------------------------
// Signature guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn webhook_without_signature_header_returns_401() {
    let (server, _pool, _dir) = common::test_app().await;
    let body = make_webhook_body("payment.captured", "captured", &[]);

    let resp = server
        .post("/api/webhooks/razorpay")
        .text(body)
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn webhook_with_invalid_signature_returns_401_when_secret_is_configured() {
    let (server, _pool, _dir) = common::test_app().await;

    setup_webhook_secret();

    let body = make_webhook_body("payment.captured", "captured", &[]);

    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            axum::http::HeaderName::from_static("x-razorpay-signature"),
            "totally-wrong-signature".parse::<axum::http::HeaderValue>().unwrap(),
        )
        .text(body)
        .await;

    resp.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn webhook_with_valid_signature_and_non_capture_event_is_acknowledged_and_ignored() {
    let (server, _pool, _dir) = common::test_app().await;
    let secret = setup_webhook_secret();

    let body = make_webhook_body("payment.failed", "failed", &[]);
    let sig = compute_signature(&body, secret);

    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            axum::http::HeaderName::from_static("x-razorpay-signature"),
            sig.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .text(body)
        .await;

    resp.assert_status_ok();
    let json_body = resp.json::<serde_json::Value>();
    assert_eq!(json_body["ok"], true);
    assert_eq!(json_body["ignored"], true);
}

#[tokio::test]
async fn webhook_payment_captured_with_missing_member_id_returns_400() {
    let (server, _pool, _dir) = common::test_app().await;
    let secret = setup_webhook_secret();

    // payment.captured but no memberId and no sponsorLinkToken in notes
    let body = make_webhook_body("payment.captured", "captured", &[]);
    let sig = compute_signature(&body, secret);

    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            axum::http::HeaderName::from_static("x-razorpay-signature"),
            sig.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .text(body)
        .await;

    // Missing memberId in notes → 400 Bad Request
    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn webhook_sponsor_payment_with_sponsor_link_token_returns_ok() {
    let (server, pool, _dir) = common::test_app().await;
    let secret = setup_webhook_secret();

    let admin = common::seed_admin_user(&pool).await;

    // Create a sponsor link
    let link_id = uuid::Uuid::new_v4().to_string();
    let token = "webhook-test-token-sp";
    sqlx::query(
        "INSERT INTO sponsor_links (id, token, upi_id, is_active, created_by_id)
         VALUES (?1, ?2, 'pay@upi', 1, ?3)",
    )
    .bind(&link_id)
    .bind(token)
    .bind(&admin.id)
    .execute(&pool)
    .await
    .unwrap();

    let body = make_webhook_body(
        "payment.captured",
        "captured",
        &[
            ("sponsorLinkToken", token),
            ("sponsorPurpose", "GOLD_SPONSOR"),
        ],
    );
    let sig = compute_signature(&body, secret);

    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            axum::http::HeaderName::from_static("x-razorpay-signature"),
            sig.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .text(body)
        .await;

    resp.assert_status_ok();
    let json_body = resp.json::<serde_json::Value>();
    assert_eq!(json_body["ok"], true);
    assert_eq!(json_body["kind"], "sponsor");
}

#[tokio::test]
async fn webhook_duplicate_payment_id_returns_already_processed_true() {
    let (server, pool, _dir) = common::test_app().await;
    let secret = setup_webhook_secret();

    let admin = common::seed_admin_user(&pool).await;

    // Pre-insert a transaction with this razorpay_payment_id
    let tx_id = uuid::Uuid::new_v4().to_string();
    let payment_id = "pay_test_12345";
    sqlx::query(
        "INSERT INTO transactions (id, type, category, amount, payment_mode, purpose, entered_by_id, approval_status, approval_source, razorpay_payment_id)
         VALUES (?1, 'CASH_IN', 'MEMBERSHIP', 250.0, 'UPI', 'Duplicate test', ?2, 'APPROVED', 'RAZORPAY_WEBHOOK', ?3)",
    )
    .bind(&tx_id)
    .bind(&admin.id)
    .bind(payment_id)
    .execute(&pool)
    .await
    .unwrap();

    // Create a member to avoid "missing memberId" path
    let member = common::seed_member(&pool, "Dup Test Member").await;

    let body = make_webhook_body(
        "payment.captured",
        "captured",
        &[("memberId", &member.id)],
    );
    let sig = compute_signature(&body, secret);

    let resp = server
        .post("/api/webhooks/razorpay")
        .add_header(
            axum::http::HeaderName::from_static("x-razorpay-signature"),
            sig.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .text(body)
        .await;

    resp.assert_status_ok();
    let json_body = resp.json::<serde_json::Value>();
    assert_eq!(json_body["alreadyProcessed"], true);
    assert_eq!(json_body["kind"], "membership");
}
