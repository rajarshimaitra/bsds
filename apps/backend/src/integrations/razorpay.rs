//! Razorpay HTTP client for BSDS Dashboard.
//!
//! Provides:
//!   - `RazorpayClient` — create orders, verify payment and webhook signatures
//!   - `rupees_to_paise` / `paise_to_rupees` — INR conversion helpers
//!
//! All amounts accepted by Razorpay are in **paise** (INR x 100).
//!
//! Configuration (environment variables):
//!   - `RAZORPAY_KEY_ID`         — API key ID
//!   - `RAZORPAY_KEY_SECRET`     — API key secret (used for payment signature verification)
//!   - `RAZORPAY_WEBHOOK_SECRET` — webhook HMAC secret
//!   - `RAZORPAY_TEST_MODE`      — set to `"true"` for test-mode UI hints

use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::env;

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayOrder {
    pub id: String,
    pub entity: String,
    /// Amount in paise.
    pub amount: u64,
    pub amount_paid: u64,
    pub amount_due: u64,
    pub currency: String,
    pub receipt: Option<String>,
    pub status: String,
    #[serde(default)]
    pub notes: HashMap<String, String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateOrderOptions {
    /// Amount in paise (INR x 100). Must be a positive integer.
    pub amount: u64,
    /// ISO-4217 currency code -- always `"INR"` for this application.
    pub currency: String,
    /// Internal reference shown on Razorpay dashboard. Max 40 chars.
    pub receipt: String,
    /// Key-value metadata stored on the order and echoed in webhooks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<HashMap<String, String>>,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum RazorpayError {
    #[error("RAZORPAY_KEY_ID and RAZORPAY_KEY_SECRET must be set in environment variables")]
    MissingCredentials,

    #[error("RAZORPAY_WEBHOOK_SECRET is not set")]
    MissingWebhookSecret,

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Razorpay API error (HTTP {status}): {body}")]
    Api { status: u16, body: String },
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Razorpay API client.
///
/// Reads credentials from environment on construction. Holds a reusable
/// `reqwest::Client` for connection pooling.
#[derive(Debug, Clone)]
pub struct RazorpayClient {
    key_id: String,
    key_secret: String,
    http: Client,
}

impl RazorpayClient {
    /// Create a new client. Returns an error if credentials are not set.
    pub fn from_env() -> Result<Self, RazorpayError> {
        let key_id = env::var("RAZORPAY_KEY_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(RazorpayError::MissingCredentials)?;
        let key_secret = env::var("RAZORPAY_KEY_SECRET")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(RazorpayError::MissingCredentials)?;

        Ok(Self {
            key_id,
            key_secret,
            http: Client::new(),
        })
    }

    /// Create a Razorpay order.
    ///
    /// `options.amount` must be in **paise** (multiply INR by 100 before calling).
    pub async fn create_order(
        &self,
        options: &CreateOrderOptions,
    ) -> Result<RazorpayOrder, RazorpayError> {
        let resp = self
            .http
            .post("https://api.razorpay.com/v1/orders")
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .json(options)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(RazorpayError::Api { status, body });
        }

        let order: RazorpayOrder = resp.json().await?;
        Ok(order)
    }

    /// Verify a Razorpay payment signature after client-side checkout.
    ///
    /// Razorpay signs: `HMAC-SHA256(order_id + "|" + payment_id, key_secret)`
    pub fn verify_payment_signature(
        &self,
        order_id: &str,
        payment_id: &str,
        signature: &str,
    ) -> bool {
        let payload = format!("{order_id}|{payment_id}");
        verify_hmac_sha256(&self.key_secret, &payload, signature)
    }

    /// Get the key ID (needed for client-side checkout initialization).
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

// ---------------------------------------------------------------------------
// Webhook signature verification
// ---------------------------------------------------------------------------

/// Verify the HMAC-SHA256 signature on an incoming Razorpay webhook request.
///
/// Uses `RAZORPAY_WEBHOOK_SECRET` from environment.
///
/// `body` must be the **raw** request body string (not parsed JSON).
/// `signature` is the value of the `x-razorpay-signature` header.
pub fn verify_webhook_signature(body: &str, signature: &str) -> Result<bool, RazorpayError> {
    let secret = env::var("RAZORPAY_WEBHOOK_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or(RazorpayError::MissingWebhookSecret)?;

    Ok(verify_hmac_sha256(&secret, body, signature))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` when `RAZORPAY_TEST_MODE=true`.
pub fn is_test_mode() -> bool {
    env::var("RAZORPAY_TEST_MODE")
        .map(|v| v == "true")
        .unwrap_or(false)
}

/// Convert an INR rupee amount (f64) to Razorpay paise (u64).
pub fn rupees_to_paise(rupees: f64) -> u64 {
    (rupees * 100.0).round() as u64
}

/// Convert Razorpay paise (u64) back to INR rupees.
pub fn paise_to_rupees(paise: u64) -> f64 {
    paise as f64 / 100.0
}

/// Constant-time HMAC-SHA256 verification.
///
/// `expected_hex` is the hex-encoded signature to verify against.
fn verify_hmac_sha256(secret: &str, payload: &str, expected_hex: &str) -> bool {
    let Ok(expected_bytes) = hex::decode(expected_hex) else {
        return false;
    };

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(payload.as_bytes());

    // `verify_slice` uses constant-time comparison internally
    mac.verify_slice(&expected_bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rupees_to_paise() {
        assert_eq!(rupees_to_paise(1500.0), 150000);
        assert_eq!(rupees_to_paise(99.99), 9999);
    }

    #[test]
    fn test_paise_to_rupees() {
        assert_eq!(paise_to_rupees(150000), 1500.0);
        assert_eq!(paise_to_rupees(9999), 99.99);
    }

    #[test]
    fn test_hmac_verification() {
        // Known test vector
        let secret = "test_secret";
        let payload = "order_123|pay_456";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(verify_hmac_sha256(secret, payload, &signature));
        assert!(!verify_hmac_sha256(secret, payload, "wrong_hex"));
        assert!(!verify_hmac_sha256(secret, "different_payload", &signature));
    }
}
