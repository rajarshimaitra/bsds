//! Member ID generation and validation.
//!
//! Format: `BSDS-YYYY-NNNN-SS`
//!   - `BSDS` (or configured brand code) prefix
//!   - `YYYY` = joining year (4 digits)
//!   - `NNNN` = auto-increment primary member number (zero-padded to 4 digits)
//!   - `SS`   = sub-member index (`00` = primary, `01`-`03` = sub-members)
//!
//! The brand code defaults to `"BSDS"` but can be overridden via the
//! `DATA_BRAND_CODE` environment variable.

use std::env;

// ---------------------------------------------------------------------------
// Brand code
// ---------------------------------------------------------------------------

/// Read the brand code from environment, defaulting to `"BSDS"`.
fn data_brand_code() -> String {
    env::var("DATA_BRAND_CODE")
        .ok()
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "BSDS".to_string())
}

// ---------------------------------------------------------------------------
// Prefix helpers
// ---------------------------------------------------------------------------

/// Returns the member ID prefix for a given year, e.g. `"BSDS-2026-"`.
pub fn member_id_prefix(year: u32) -> String {
    format!("{}-{}-", data_brand_code(), year)
}

/// Returns the receipt number prefix for a given year, e.g. `"BSDS-REC-2026-"`.
pub fn receipt_number_prefix(year: u32) -> String {
    format!("{}-REC-{}-", data_brand_code(), year)
}

// ---------------------------------------------------------------------------
// Primary member ID
// ---------------------------------------------------------------------------

/// Build a member ID from components.
///
/// ```text
/// build_member_id(2026, 1, 0) => "BSDS-2026-0001-00"
/// build_member_id(2026, 42, 2) => "BSDS-2026-0042-02"
/// ```
pub fn build_member_id(year: u32, sequence: u32, suffix: u32) -> String {
    format!(
        "{}{:04}-{:02}",
        member_id_prefix(year),
        sequence,
        suffix
    )
}

/// Generate the next primary member ID given the current highest sequence
/// number for this year. The caller is responsible for querying the DB to
/// determine `current_max_sequence` (pass `None` if no members exist yet).
///
/// Returns e.g. `"BSDS-2026-0001-00"`.
pub fn generate_member_id(year: u32, current_max_sequence: Option<u32>) -> String {
    let next_seq = current_max_sequence.map_or(1, |s| s + 1);
    build_member_id(year, next_seq, 0)
}

// ---------------------------------------------------------------------------
// Sub-member ID
// ---------------------------------------------------------------------------

/// Generate a sub-member ID given the parent's primary member ID and the
/// 1-based sub-member index (1, 2, or 3).
///
/// ```text
/// generate_sub_member_id("BSDS-2026-0001-00", 1) => Ok("BSDS-2026-0001-01")
/// ```
pub fn generate_sub_member_id(parent_member_id: &str, index: u32) -> Result<String, String> {
    if !(1..=3).contains(&index) {
        return Err(format!("Sub-member index must be 1-3, got {index}"));
    }
    // Replace the last two characters (SS suffix) with the sub-member index
    if parent_member_id.len() < 2 {
        return Err("Parent member ID is too short".to_string());
    }
    let base = &parent_member_id[..parent_member_id.len() - 2];
    Ok(format!("{base}{:02}", index))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Parse the numeric NNNN segment from a member ID.
/// Returns `None` if the format does not match.
pub fn parse_sequence_number(member_id: &str) -> Option<u32> {
    let brand = data_brand_code();
    // Expected: BRAND-YYYY-NNNN-SS
    let parts: Vec<&str> = member_id.split('-').collect();
    if parts.len() != 4 {
        return None;
    }
    if parts[0] != brand {
        return None;
    }
    // YYYY must be exactly 4 digits
    if parts[1].len() != 4 || parts[1].parse::<u32>().is_err() {
        return None;
    }
    // NNNN must be exactly 4 digits
    if parts[2].len() != 4 {
        return None;
    }
    let seq = parts[2].parse::<u32>().ok()?;
    // SS must be exactly 2 digits
    if parts[3].len() != 2 || parts[3].parse::<u32>().is_err() {
        return None;
    }
    Some(seq)
}

/// Returns `true` if the string matches the `BSDS-YYYY-NNNN-SS` format.
pub fn is_valid_member_id(id: &str) -> bool {
    parse_sequence_number(id).is_some()
}

// ---------------------------------------------------------------------------
// Receipt reference helpers (ported from branding.ts)
// ---------------------------------------------------------------------------

/// Build a Razorpay receipt reference for member orders (max 40 chars).
pub fn build_member_order_receipt_reference(member_id: &str, timestamp: u64) -> String {
    let brand = data_brand_code();
    let prefix = &member_id[..member_id.len().min(8)];
    let raw = format!("{brand}-{prefix}-{timestamp}");
    raw[..raw.len().min(40)].to_string()
}

/// Build a Razorpay receipt reference for sponsor orders (max 40 chars).
pub fn build_sponsor_order_receipt_reference(token: &str, timestamp: u64) -> String {
    let brand = data_brand_code();
    let prefix = &token[..token.len().min(8)];
    let raw = format!("{brand}-SP-{prefix}-{timestamp}");
    raw[..raw.len().min(40)].to_string()
}

/// Build a fallback receipt reference from a Razorpay payment ID.
pub fn build_sponsor_payment_fallback_receipt(payment_id: &str) -> String {
    let brand = data_brand_code();
    let end = payment_id.len().min(12);
    let start = 4.min(end);
    let segment = &payment_id[start..end];
    format!("{brand}-PAY-{}", segment.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_member_id() {
        let id = build_member_id(2026, 1, 0);
        assert_eq!(id, "BSDS-2026-0001-00");
    }

    #[test]
    fn test_generate_member_id_first() {
        let id = generate_member_id(2026, None);
        assert_eq!(id, "BSDS-2026-0001-00");
    }

    #[test]
    fn test_generate_member_id_next() {
        let id = generate_member_id(2026, Some(5));
        assert_eq!(id, "BSDS-2026-0006-00");
    }

    #[test]
    fn test_generate_sub_member_id() {
        let id = generate_sub_member_id("BSDS-2026-0001-00", 2).unwrap();
        assert_eq!(id, "BSDS-2026-0001-02");
    }

    #[test]
    fn test_sub_member_id_invalid_index() {
        assert!(generate_sub_member_id("BSDS-2026-0001-00", 0).is_err());
        assert!(generate_sub_member_id("BSDS-2026-0001-00", 4).is_err());
    }

    #[test]
    fn test_is_valid_member_id() {
        assert!(is_valid_member_id("BSDS-2026-0001-00"));
        assert!(is_valid_member_id("BSDS-2026-0042-03"));
        assert!(!is_valid_member_id("BSDS-26-0001-00"));
        assert!(!is_valid_member_id("invalid"));
        assert!(!is_valid_member_id("BSDS-2026-001-00"));
    }

    #[test]
    fn test_parse_sequence_number() {
        assert_eq!(parse_sequence_number("BSDS-2026-0042-00"), Some(42));
        assert_eq!(parse_sequence_number("invalid"), None);
    }
}
