//! Server-side input validation for BSDS Dashboard API routes.
//!
//! Standalone validation functions — no Zod equivalent in Rust, so these are
//! plain functions returning `Result<(), String>` or `Option<String>` (error).

use std::collections::HashSet;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// String sanitization
// ---------------------------------------------------------------------------

/// Sanitize a string input by trimming whitespace and stripping ASCII control
/// characters (0x00-0x1F and 0x7F).
pub fn sanitize_string(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|&c| !c.is_ascii_control())
        .collect()
}

// ---------------------------------------------------------------------------
// Field validators
// ---------------------------------------------------------------------------

/// Validate an Indian phone number in `+91XXXXXXXXXX` format.
pub fn validate_phone(phone: &str) -> Result<(), String> {
    let re_pattern = phone.len() == 13
        && phone.starts_with("+91")
        && phone[3..].chars().all(|c| c.is_ascii_digit());
    if !re_pattern {
        return Err("Phone must be in +91XXXXXXXXXX format".to_string());
    }
    Ok(())
}

/// Validate an email address (basic format check).
pub fn validate_email(email: &str) -> Result<(), String> {
    // Basic check: non-empty, has exactly one @, both parts non-empty, domain has a dot.
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err("Must be a valid email address".to_string());
    }
    if !parts[1].contains('.') {
        return Err("Must be a valid email address".to_string());
    }
    Ok(())
}

/// Validate a name field (non-empty, max 255 chars).
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name is required".to_string());
    }
    if name.len() > 255 {
        return Err("Name must be 255 characters or less".to_string());
    }
    Ok(())
}

/// Validate a UUID string.
pub fn validate_uuid(value: &str, field_name: &str) -> Result<(), String> {
    Uuid::parse_str(value).map_err(|_| format!("{field_name} must be a valid UUID"))?;
    Ok(())
}

/// Validate an amount: must be positive, at most 2 decimal places.
pub fn validate_amount(amount: f64) -> Result<(), String> {
    if amount <= 0.0 {
        return Err("amount must be a positive number".to_string());
    }
    // Check at most 2 decimal places: multiply by 100, compare with rounded.
    let scaled = amount * 100.0;
    if (scaled - scaled.round()).abs() > 1e-9 {
        return Err("amount must have at most 2 decimal places".to_string());
    }
    Ok(())
}

/// Validate a string field is non-empty with a max length.
pub fn validate_required_string(value: &str, field_name: &str, max_len: usize) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{field_name} is required"));
    }
    if value.len() > max_len {
        return Err(format!(
            "{field_name} must be {max_len} characters or less"
        ));
    }
    Ok(())
}

/// Validate an optional string field has a max length.
pub fn validate_optional_string(value: Option<&str>, field_name: &str, max_len: usize) -> Result<(), String> {
    if let Some(v) = value {
        if v.len() > max_len {
            return Err(format!(
                "{field_name} must be {max_len} characters or less"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Enum validators
// ---------------------------------------------------------------------------

/// Valid transaction types.
pub const TRANSACTION_TYPES: &[&str] = &["CASH_IN", "CASH_OUT"];

/// Valid transaction categories.
pub const TRANSACTION_CATEGORIES: &[&str] = &["MEMBERSHIP", "SPONSORSHIP", "EXPENSE", "OTHER"];

/// Valid payment modes.
pub const PAYMENT_MODES: &[&str] = &["UPI", "BANK_TRANSFER", "CASH"];

/// Valid sponsor purposes.
pub const SPONSOR_PURPOSES: &[&str] = &[
    "TITLE_SPONSOR",
    "GOLD_SPONSOR",
    "SILVER_SPONSOR",
    "FOOD_PARTNER",
    "MEDIA_PARTNER",
    "STALL_VENDOR",
    "MARKETING_PARTNER",
    "OTHER",
];

/// Valid expense purposes.
pub const EXPENSE_PURPOSES: &[&str] = &[
    "DECORATION_PANDAL",
    "IDOL_MURTI",
    "LIGHTING_SOUND",
    "FOOD_BHOG_PRASAD",
    "PRIEST_PUROHIT",
    "TRANSPORT_LOGISTICS",
    "PRINTING_PUBLICITY",
    "CULTURAL_PROGRAM",
    "CLEANING_SANITATION",
    "ELECTRICITY_GENERATOR",
    "SECURITY",
    "OTHER",
];

/// Valid approval statuses.
pub const APPROVAL_STATUSES: &[&str] = &["PENDING", "APPROVED", "REJECTED"];

/// Valid membership types.
pub const MEMBERSHIP_TYPES: &[&str] = &["MONTHLY", "HALF_YEARLY", "ANNUAL"];

/// Valid member statuses.
pub const MEMBER_STATUSES: &[&str] = &[
    "PENDING_APPROVAL",
    "PENDING_PAYMENT",
    "ACTIVE",
    "EXPIRED",
    "SUSPENDED",
];

/// Validate a value is one of the allowed options.
pub fn validate_enum(value: &str, allowed: &[&str], field_name: &str) -> Result<(), String> {
    if !allowed.contains(&value) {
        return Err(format!(
            "{field_name} must be one of: {}",
            allowed.join(", ")
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Password validation
// ---------------------------------------------------------------------------

/// Common weak passwords that must be rejected.
fn common_passwords() -> HashSet<&'static str> {
    [
        "password",
        "password1",
        "password123",
        "12345678",
        "123456789",
        "1234567890",
        "qwerty123",
        "qwertyuiop",
        "admin1234",
        "letmein1",
        "welcome1",
        "monkey123",
        "abc12345",
        "iloveyou1",
        "sunshine1",
    ]
    .into_iter()
    .collect()
}

/// Validate a new password: min 8, max 128, not common, different from current.
pub fn validate_password_change(
    current_password: &str,
    new_password: &str,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if current_password.len() < 8 {
        errors.push("Current password must be at least 8 characters".to_string());
    }
    if new_password.len() < 8 {
        errors.push("New password must be at least 8 characters".to_string());
    }
    if new_password.len() > 128 {
        errors.push("New password must be 128 characters or less".to_string());
    }
    if common_passwords().contains(new_password.to_lowercase().as_str()) {
        errors.push(
            "This password is too common \u{2014} please choose a stronger password".to_string(),
        );
    }
    if current_password == new_password {
        errors.push("New password must be different from current password".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// IFSC code validation
// ---------------------------------------------------------------------------

/// Validate an IFSC code (format: `XXXX0XXXXXX`).
pub fn validate_ifsc(code: &str) -> Result<(), String> {
    if code.len() != 11 {
        return Err("ifscCode must be in XXXX0XXXXXX format".to_string());
    }
    let chars: Vec<char> = code.chars().collect();
    let valid = chars[..4].iter().all(|c| c.is_ascii_uppercase())
        && chars[4] == '0'
        && chars[5..].iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if !valid {
        return Err("ifscCode must be in XXXX0XXXXXX format".to_string());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Pagination validation
// ---------------------------------------------------------------------------

/// Parsed and validated pagination parameters.
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub page: u32,
    pub limit: u32,
    pub offset: u32,
}

/// Parse and validate pagination query parameters.
/// Defaults: page=1, limit=20. Limit is clamped to [1, 100].
pub fn validate_pagination(page: Option<u32>, limit: Option<u32>) -> Pagination {
    let page = page.unwrap_or(1).max(1);
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;
    Pagination {
        page,
        limit,
        offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("  hello\x00world  "), "helloworld");
        assert_eq!(sanitize_string("clean"), "clean");
    }

    #[test]
    fn test_validate_phone() {
        assert!(validate_phone("+919830012345").is_ok());
        assert!(validate_phone("9830012345").is_err());
        assert!(validate_phone("+91983001234").is_err()); // too short
        assert!(validate_phone("+9198300123456").is_err()); // too long
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@domain").is_err()); // no dot
    }

    #[test]
    fn test_validate_name() {
        assert!(validate_name("John").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name(&"x".repeat(256)).is_err());
    }

    #[test]
    fn test_validate_uuid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000", "id").is_ok());
        assert!(validate_uuid("not-a-uuid", "id").is_err());
    }

    #[test]
    fn test_validate_amount() {
        assert!(validate_amount(100.0).is_ok());
        assert!(validate_amount(99.99).is_ok());
        assert!(validate_amount(0.0).is_err());
        assert!(validate_amount(-1.0).is_err());
    }

    #[test]
    fn test_validate_enum() {
        assert!(validate_enum("CASH_IN", TRANSACTION_TYPES, "type").is_ok());
        assert!(validate_enum("INVALID", TRANSACTION_TYPES, "type").is_err());
    }

    #[test]
    fn test_validate_password_change() {
        assert!(validate_password_change("oldpass12", "newpass99").is_ok());
        assert!(validate_password_change("short", "newpass99").is_err());
        assert!(validate_password_change("oldpass12", "oldpass12").is_err());
        assert!(validate_password_change("oldpass12", "password123").is_err());
    }

    #[test]
    fn test_validate_ifsc() {
        assert!(validate_ifsc("SBIN0001234").is_ok());
        assert!(validate_ifsc("sbin0001234").is_err());
        assert!(validate_ifsc("SHORT").is_err());
    }

    #[test]
    fn test_validate_pagination() {
        let p = validate_pagination(None, None);
        assert_eq!(p.page, 1);
        assert_eq!(p.limit, 20);
        assert_eq!(p.offset, 0);

        let p = validate_pagination(Some(3), Some(50));
        assert_eq!(p.offset, 100);

        let p = validate_pagination(Some(0), Some(200));
        assert_eq!(p.page, 1);
        assert_eq!(p.limit, 100);
    }
}
