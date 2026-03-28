pub mod permissions;
pub mod temp_password;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Json,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Session claims
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionClaims {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub member_id: Option<String>,
    pub must_change_password: bool,
}

// ---------------------------------------------------------------------------
// Cookie helpers
// ---------------------------------------------------------------------------

pub const COOKIE_NAME: &str = "bsds_session";

/// Build a `Set-Cookie` header value that stores the session token.
pub fn make_cookie(token: &str) -> String {
    format!(
        "{COOKIE_NAME}={token}; HttpOnly; SameSite=Strict; Path=/"
    )
}

/// Build a `Set-Cookie` header value that clears the session cookie.
pub fn clear_cookie() -> String {
    format!(
        "{COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"
    )
}

// ---------------------------------------------------------------------------
// HMAC-SHA256 signed session tokens
// ---------------------------------------------------------------------------

/// Create a session token: base64url(json) + "." + hex(hmac).
pub fn create_session_token(claims: &SessionClaims, secret: &str) -> String {
    let json = serde_json::to_string(claims).expect("SessionClaims must serialize");
    let payload = URL_SAFE_NO_PAD.encode(json.as_bytes());

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());

    format!("{payload}.{sig}")
}

/// Verify a session token and return the decoded claims, or `None` if invalid.
pub fn verify_session_token(token: &str, secret: &str) -> Option<SessionClaims> {
    let (payload, sig_hex) = token.rsplit_once('.')?;

    // Verify HMAC
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(payload.as_bytes());
    let expected_sig = hex::decode(sig_hex).ok()?;
    mac.verify_slice(&expected_sig).ok()?;

    // Decode claims
    let json_bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&json_bytes).ok()
}

// ---------------------------------------------------------------------------
// Password helpers (bcrypt cost 12)
// ---------------------------------------------------------------------------

pub fn hash_password(plain: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(plain, 12)
}

pub fn verify_password(plain: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(plain, hash)
}

// ---------------------------------------------------------------------------
// Axum extractor — pulls SessionClaims from the request cookie
// ---------------------------------------------------------------------------

pub struct AuthSession(pub SessionClaims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
        let cookie_header = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let token = cookie_header.split(';').find_map(|seg| {
            let seg = seg.trim();
            let (name, value) = seg.split_once('=')?;
            if name.trim() == COOKIE_NAME {
                Some(value.trim())
            } else {
                None
            }
        });

        let token = match token {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "authentication required" })),
                ));
            }
        };

        match verify_session_token(token, &secret) {
            Some(claims) => Ok(AuthSession(claims)),
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "invalid or expired session" })),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_session_token() {
        let claims = SessionClaims {
            user_id: "u1".into(),
            username: "alice".into(),
            role: "ADMIN".into(),
            member_id: Some("BSDS-2026-0001-00".into()),
            must_change_password: false,
        };
        let secret = "test-secret-key";
        let token = create_session_token(&claims, secret);
        let decoded = verify_session_token(&token, secret).expect("should verify");
        assert_eq!(decoded.user_id, "u1");
        assert_eq!(decoded.username, "alice");
        assert_eq!(decoded.role, "ADMIN");
        assert_eq!(decoded.member_id.as_deref(), Some("BSDS-2026-0001-00"));
        assert!(!decoded.must_change_password);
    }

    #[test]
    fn tampered_token_rejected() {
        let claims = SessionClaims {
            user_id: "u1".into(),
            username: "alice".into(),
            role: "ADMIN".into(),
            member_id: None,
            must_change_password: false,
        };
        let token = create_session_token(&claims, "secret-a");
        assert!(verify_session_token(&token, "secret-b").is_none());
    }

    #[test]
    fn password_hash_and_verify() {
        let hash = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn cookie_helpers() {
        let c = make_cookie("tok123");
        assert!(c.contains("bsds_session=tok123"));
        assert!(c.contains("HttpOnly"));
        assert!(c.contains("SameSite=Strict"));
        assert!(c.contains("Path=/"));

        let cl = clear_cookie();
        assert!(cl.contains("Max-Age=0"));
    }
}
