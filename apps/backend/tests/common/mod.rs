//! Shared test infrastructure for all integration tests.
//!
//! Provides:
//!   - `test_app()` — spin up a TestServer backed by an isolated in-memory SQLite DB
//!   - `seed_admin_user()` — insert an ADMIN user and return credentials + session cookie
//!   - `seed_operator_user()` — insert an OPERATOR user
//!   - `seed_organiser_user()` — insert an ORGANISER user
//!   - `seed_member_user()` — insert a MEMBER user
//!   - `seed_user_with_role()` — insert a user with any specified role
//!   - `seed_member_record()` — create a full member record via direct POST (admin-authed)
//!   - `create_pending_approval()` — create a pending MEMBER_ADD approval via OPERATOR POST
//!   - `auth_cookie()` — build a valid session cookie for the given user data

use axum_test::TestServer;
use bsds_backend::{auth, db};
use sqlx::SqlitePool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// App factory
// ---------------------------------------------------------------------------

/// Spin up an isolated test application.
///
/// Each call creates a fresh, temporary SQLite file so tests cannot
/// interfere with each other.  The tempdir is kept alive via the returned
/// `tempfile::TempDir`; callers **must** keep it alive for the duration of
/// the test.
pub async fn test_app() -> (TestServer, SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    let db_path = dir.path().join("test.sqlite3");
    // sqlx SQLite URL: use absolute path with ?mode=rwc so it creates the file.
    let url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = db::connect(&url).await;
    let app = bsds_backend::build_router(pool.clone());
    let server = TestServer::new(app).expect("TestServer creation failed");
    (server, pool, dir)
}

// ---------------------------------------------------------------------------
// User seed helpers
// ---------------------------------------------------------------------------

pub struct SeedUser {
    pub id: String,
    pub email: String,
    pub password_plain: String,
    pub role: String,
    pub member_id: String,
}

/// Insert an ADMIN user and return its credentials.
pub async fn seed_admin_user(pool: &SqlitePool) -> SeedUser {
    seed_user(pool, "ADMIN", "admin@test.local", "AdminPass1!").await
}

/// Insert an OPERATOR user and return its credentials.
pub async fn seed_operator_user(pool: &SqlitePool) -> SeedUser {
    seed_user(pool, "OPERATOR", "operator@test.local", "OperatorPass1!").await
}

/// Insert an ORGANISER user and return its credentials.
pub async fn seed_organiser_user(pool: &SqlitePool) -> SeedUser {
    seed_user(pool, "ORGANISER", "organiser@test.local", "OrganiserPass1!").await
}

/// Insert a MEMBER user and return its credentials.
pub async fn seed_member_user(pool: &SqlitePool) -> SeedUser {
    seed_user(pool, "MEMBER", "member@test.local", "MemberPass1!").await
}

/// Insert a user with the given role.  The email is derived from the role
/// string so it is unique across calls with different roles in the same DB.
/// Returns `(user_id, cookie_header_value)` for convenience in security tests.
pub async fn seed_user_with_role(pool: &SqlitePool, role: &str) -> (String, String) {
    let unique_suffix = rand_u16();
    let email = format!("{}-{}@test.local", role.to_lowercase(), unique_suffix);
    let password = format!("{role}Pass1!");
    let user = seed_user(pool, role, &email, &password).await;
    let cookie = auth_cookie(&user);
    (user.id, cookie)
}

async fn seed_user(
    pool: &SqlitePool,
    role: &str,
    email: &str,
    password: &str,
) -> SeedUser {
    let id = Uuid::new_v4().to_string();
    let member_id = format!("BSDS-2026-{:04}-00", (rand_u16() % 9000) + 1000);
    let hashed = auth::hash_password(password).expect("hash_password failed");

    sqlx::query(
        "INSERT INTO users (id, member_id, name, email, phone, address, password, is_temp_password, role, membership_status, application_fee_paid)
         VALUES (?1, ?2, ?3, ?4, '+910000000000', 'Test Address', ?5, 0, ?6, 'ACTIVE', 1)",
    )
    .bind(&id)
    .bind(&member_id)
    .bind(format!("Test {role}"))
    .bind(email)
    .bind(&hashed)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed_user insert failed");

    SeedUser {
        id,
        email: email.to_string(),
        password_plain: password.to_string(),
        role: role.to_string(),
        member_id,
    }
}

// ---------------------------------------------------------------------------
// Session cookie helper
// ---------------------------------------------------------------------------

/// Build a `Cookie: bsds_session=<token>` header value for a user.
pub fn auth_cookie(user: &SeedUser) -> String {
    let claims = auth::SessionClaims {
        user_id: user.id.clone(),
        username: user.email.clone(),
        role: user.role.clone(),
        member_id: Some(user.member_id.clone()),
        must_change_password: false,
    };
    let secret = std::env::var("SESSION_SECRET").unwrap_or_default();
    let token = auth::create_session_token(&claims, &secret);
    format!("bsds_session={token}")
}

// ---------------------------------------------------------------------------
// Member seed helper
// ---------------------------------------------------------------------------

pub struct SeedMember {
    pub id: String,
    pub user_id: String,
}

/// Insert a Member row (linked to a fresh MEMBER user).
pub async fn seed_member(pool: &SqlitePool, name: &str) -> SeedMember {
    let user = seed_user(
        pool,
        "MEMBER",
        &format!("{}@test.local", name.to_lowercase().replace(' ', ".")),
        "MemberPass1!",
    )
    .await;

    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO members (id, user_id, name, phone, email, address)
         VALUES (?1, ?2, ?3, '+910000000001', ?4, 'Some Address')",
    )
    .bind(&member_id)
    .bind(&user.id)
    .bind(name)
    .bind(&user.email)
    .execute(pool)
    .await
    .expect("seed_member insert failed");

    SeedMember {
        id: member_id,
        user_id: user.id,
    }
}

// ---------------------------------------------------------------------------
// Higher-level workflow helpers
// ---------------------------------------------------------------------------

/// Create a full member via the API (admin-direct path) and return the memberId.
///
/// This drives the actual POST /api/members endpoint rather than inserting
/// directly, so the result reflects the real service logic.
#[allow(dead_code)]
pub async fn seed_member_record(
    pool: &SqlitePool,
    _admin_cookie: &str,
    server: &TestServer,
) -> String {
    // We need a fresh unique email for each call.
    let n = rand_u16();
    let email = format!("api-member-{}@test.local", n);
    let phone = format!("+9190{:08}", n);

    // Ensure an admin user exists in the pool so the route handler can look it up.
    let admin = seed_admin_user(pool).await;
    let cookie = auth_cookie(&admin);

    let resp = server
        .post("/api/members")
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&serde_json::json!({
            "name": format!("API Member {}", n),
            "email": email,
            "phone": phone,
            "address": "123 Seed Street"
        }))
        .await;

    let body = resp.json::<serde_json::Value>();
    body["memberId"]
        .as_str()
        .expect("memberId missing from admin create response")
        .to_string()
}

/// Submit a member-add request as OPERATOR and return the `approvalId`.
///
/// This creates an OPERATOR user in the pool, sends the POST request, and
/// returns the approval ID returned in the response body.
pub async fn create_pending_approval(
    pool: &SqlitePool,
    server: &TestServer,
    operator_cookie: &str,
) -> String {
    let n = rand_u16();
    let email = format!("pending-member-{}@test.local", n);
    let phone = format!("+9191{:08}", n);

    let resp = server
        .post("/api/members")
        .add_header(
            axum::http::HeaderName::from_static("cookie"),
            operator_cookie.parse::<axum::http::HeaderValue>().unwrap(),
        )
        .json(&serde_json::json!({
            "name": format!("Pending Member {}", n),
            "email": email,
            "phone": phone,
            "address": "456 Pending Ave"
        }))
        .await;

    assert!(
        resp.status_code().is_success(),
        "create_pending_approval: POST /api/members failed with status {}",
        resp.status_code()
    );

    let body = resp.json::<serde_json::Value>();
    body["approvalId"]
        .as_str()
        .expect("approvalId missing from operator create response")
        .to_string()
}

// ---------------------------------------------------------------------------
// Tiny pseudo-random u16 — avoids pulling in rand in tests
// ---------------------------------------------------------------------------

fn rand_u16() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Combine nanosecond timestamp with a thread-local counter for uniqueness
    // even when two calls happen within the same nanosecond.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let mixed = nanos.wrapping_add(count.wrapping_mul(6364136223846793005));
    (mixed ^ (mixed >> 16) ^ (mixed >> 32)) as u16
}
