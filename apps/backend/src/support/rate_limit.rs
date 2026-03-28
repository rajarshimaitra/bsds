//! In-memory sliding-window rate limiter.
//!
//! Uses a `HashMap<String, Vec<u64>>` where each entry stores timestamps (ms)
//! of recent requests. Expired entries are pruned lazily on each check.
//!
//! Designed for single-instance deployment. For multi-instance deployments,
//! swap the in-memory store for Redis.
//!
//! // TODO: persistent store (Redis) for multi-instance deployments

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// Outcome of a rate limit check.
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// `true` if the request is allowed, `false` if rate limit exceeded.
    pub success: bool,
    /// Number of remaining attempts in the current window.
    pub remaining: u32,
    /// When the oldest entry in the window expires (epoch milliseconds).
    pub reset_at_ms: u64,
}

// ---------------------------------------------------------------------------
// Pre-configured limits
// ---------------------------------------------------------------------------

/// 5 attempts per 15 minutes -- login and change-password.
pub const LOGIN_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    max_attempts: 5,
    window_ms: 15 * 60 * 1000,
};

/// 100 requests per minute -- general authenticated API routes.
pub const API_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    max_attempts: 100,
    window_ms: 60 * 1000,
};

/// 50 requests per minute -- webhook endpoints.
pub const WEBHOOK_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    max_attempts: 50,
    window_ms: 60 * 1000,
};

/// 30 requests per minute -- public endpoints (sponsor checkout, etc.).
pub const PUBLIC_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    max_attempts: 30,
    window_ms: 60 * 1000,
};

#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    pub max_attempts: u32,
    pub window_ms: u64,
}

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

/// Thread-safe in-memory rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    store: Arc<Mutex<HashMap<String, Vec<u64>>>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check and record a request for the given key.
    ///
    /// # Arguments
    /// * `key`          - Unique identifier for the bucket (e.g. `"login:192.168.1.1"`)
    /// * `max_attempts` - Maximum requests allowed within the window
    /// * `window_ms`    - Sliding window duration in milliseconds
    pub fn check(&self, key: &str, max_attempts: u32, window_ms: u64) -> RateLimitResult {
        let now = now_ms();
        let cutoff = now.saturating_sub(window_ms);

        let mut store = self.store.lock().expect("rate limiter lock poisoned");

        // Get existing timestamps, prune expired entries
        let valid: Vec<u64> = store
            .get(key)
            .map(|timestamps| timestamps.iter().copied().filter(|&t| t > cutoff).collect())
            .unwrap_or_default();

        // Check if limit is exceeded
        if valid.len() >= max_attempts as usize {
            let oldest = valid[0]; // sorted oldest -> newest
            let reset_at = oldest + window_ms;
            return RateLimitResult {
                success: false,
                remaining: 0,
                reset_at_ms: reset_at,
            };
        }

        // Record this request
        let mut valid = valid;
        valid.push(now);
        let remaining = max_attempts - valid.len() as u32;
        let oldest = valid[0];
        let reset_at = oldest + window_ms;

        store.insert(key.to_string(), valid);

        RateLimitResult {
            success: true,
            remaining,
            reset_at_ms: reset_at,
        }
    }

    /// Check using a pre-configured rate limit.
    pub fn check_config(&self, key: &str, config: RateLimitConfig) -> RateLimitResult {
        self.check(key, config.max_attempts, config.window_ms)
    }

    /// Remove all expired entries from the store.
    /// Called periodically to prevent memory leaks in long-running processes.
    pub fn cleanup(&self, max_age_ms: u64) {
        let now = now_ms();
        let cutoff = now.saturating_sub(max_age_ms);

        let mut store = self.store.lock().expect("rate limiter lock poisoned");
        store.retain(|_, timestamps| {
            timestamps.retain(|&t| t > cutoff);
            !timestamps.is_empty()
        });
    }

    /// Clear all rate limit entries. Intended for tests only.
    pub fn clear(&self) {
        let mut store = self.store.lock().expect("rate limiter lock poisoned");
        store.clear();
    }

    /// Number of keys currently in the store. Intended for tests only.
    pub fn store_size(&self) -> usize {
        let store = self.store.lock().expect("rate limiter lock poisoned");
        store.len()
    }
}

// ---------------------------------------------------------------------------
// Key helpers
// ---------------------------------------------------------------------------

/// Build a rate-limit key from an IP address and a prefix.
///
/// If `forwarded_for` is provided (from `X-Forwarded-For` header), uses the
/// first IP in the comma-separated list. Falls back to `real_ip`, then
/// `"unknown"`.
pub fn rate_limit_key(
    prefix: &str,
    forwarded_for: Option<&str>,
    real_ip: Option<&str>,
) -> String {
    let ip = forwarded_for
        .and_then(|ff| ff.split(',').next())
        .map(str::trim)
        .or(real_ip)
        .unwrap_or("unknown");
    format!("{prefix}:{ip}")
}

/// Build a rate-limit key scoped to a specific user ID.
pub fn rate_limit_key_for_user(user_id: &str, prefix: &str) -> String {
    format!("{prefix}:user:{user_id}")
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_under_limit() {
        let limiter = RateLimiter::new();
        let result = limiter.check("test", 5, 60_000);
        assert!(result.success);
        assert_eq!(result.remaining, 4);
    }

    #[test]
    fn test_blocks_over_limit() {
        let limiter = RateLimiter::new();
        for _ in 0..5 {
            limiter.check("test", 5, 60_000);
        }
        let result = limiter.check("test", 5, 60_000);
        assert!(!result.success);
        assert_eq!(result.remaining, 0);
    }

    #[test]
    fn test_different_keys_independent() {
        let limiter = RateLimiter::new();
        for _ in 0..5 {
            limiter.check("a", 5, 60_000);
        }
        let result = limiter.check("b", 5, 60_000);
        assert!(result.success);
    }

    #[test]
    fn test_rate_limit_key() {
        assert_eq!(
            rate_limit_key("login", Some("1.2.3.4, 5.6.7.8"), None),
            "login:1.2.3.4"
        );
        assert_eq!(
            rate_limit_key("login", None, Some("9.8.7.6")),
            "login:9.8.7.6"
        );
        assert_eq!(
            rate_limit_key("login", None, None),
            "login:unknown"
        );
    }

    #[test]
    fn test_rate_limit_key_for_user() {
        assert_eq!(
            rate_limit_key_for_user("user-123", "api"),
            "api:user:user-123"
        );
    }

    #[test]
    fn test_clear() {
        let limiter = RateLimiter::new();
        limiter.check("test", 5, 60_000);
        assert_eq!(limiter.store_size(), 1);
        limiter.clear();
        assert_eq!(limiter.store_size(), 0);
    }
}
