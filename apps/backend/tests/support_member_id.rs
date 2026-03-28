//! # Member ID Support Tests
//!
//! Covers: generate_member_id format, sequential numbering, sub-member ID
//!         derivation, sub-member index bounds, is_valid_member_id, parse_sequence_number,
//!         receipt reference helpers, brand code environment override.
//!
//! Does NOT cover: DB-level uniqueness constraint (covered by integration test
//!                 admin_create_member_id_follows_bsds_format in members_integration.rs).
//!
//! Protects: apps/backend/src/support/member_id.rs

use bsds_backend::support::member_id;

// ---------------------------------------------------------------------------
// build_member_id
// ---------------------------------------------------------------------------

#[test]
fn build_member_id_formats_year_and_sequence_and_suffix_correctly() {
    let id = member_id::build_member_id(2026, 1, 0);
    assert_eq!(id, "BSDS-2026-0001-00");
}

#[test]
fn build_member_id_zero_pads_sequence_to_four_digits() {
    assert_eq!(member_id::build_member_id(2026, 42, 0), "BSDS-2026-0042-00");
    assert_eq!(member_id::build_member_id(2026, 9999, 0), "BSDS-2026-9999-00");
}

#[test]
fn build_member_id_formats_sub_member_suffix_correctly() {
    assert_eq!(member_id::build_member_id(2026, 1, 1), "BSDS-2026-0001-01");
    assert_eq!(member_id::build_member_id(2026, 1, 3), "BSDS-2026-0001-03");
}

// ---------------------------------------------------------------------------
// generate_member_id
// ---------------------------------------------------------------------------

#[test]
fn generate_member_id_with_no_existing_members_starts_at_0001() {
    let id = member_id::generate_member_id(2026, None);
    assert_eq!(id, "BSDS-2026-0001-00");
}

#[test]
fn generate_member_id_increments_from_current_max() {
    let id = member_id::generate_member_id(2026, Some(5));
    assert_eq!(id, "BSDS-2026-0006-00");
}

#[test]
fn generate_member_id_uses_correct_year() {
    let id_2025 = member_id::generate_member_id(2025, None);
    let id_2026 = member_id::generate_member_id(2026, None);
    assert!(id_2025.starts_with("BSDS-2025-"), "year 2025 prefix incorrect");
    assert!(id_2026.starts_with("BSDS-2026-"), "year 2026 prefix incorrect");
}

#[test]
fn generate_member_id_always_ends_with_00_suffix() {
    let id = member_id::generate_member_id(2026, Some(10));
    assert!(id.ends_with("-00"), "primary member must end with -00");
}

// ---------------------------------------------------------------------------
// generate_sub_member_id
// ---------------------------------------------------------------------------

#[test]
fn generate_sub_member_id_replaces_last_two_chars_with_index() {
    let id = member_id::generate_sub_member_id("BSDS-2026-0001-00", 1).unwrap();
    assert_eq!(id, "BSDS-2026-0001-01");
}

#[test]
fn generate_sub_member_id_index_2_produces_02_suffix() {
    let id = member_id::generate_sub_member_id("BSDS-2026-0001-00", 2).unwrap();
    assert_eq!(id, "BSDS-2026-0001-02");
}

#[test]
fn generate_sub_member_id_index_3_produces_03_suffix() {
    let id = member_id::generate_sub_member_id("BSDS-2026-0001-00", 3).unwrap();
    assert_eq!(id, "BSDS-2026-0001-03");
}

#[test]
fn generate_sub_member_id_index_0_returns_error() {
    let result = member_id::generate_sub_member_id("BSDS-2026-0001-00", 0);
    assert!(result.is_err(), "index 0 should be rejected");
}

#[test]
fn generate_sub_member_id_index_4_returns_error() {
    let result = member_id::generate_sub_member_id("BSDS-2026-0001-00", 4);
    assert!(result.is_err(), "index 4 should be rejected");
}

#[test]
fn generate_sub_member_id_too_short_parent_returns_error() {
    let result = member_id::generate_sub_member_id("X", 1);
    assert!(result.is_err(), "too-short parent ID should be rejected");
}

// ---------------------------------------------------------------------------
// is_valid_member_id
// ---------------------------------------------------------------------------

#[test]
fn is_valid_member_id_returns_true_for_well_formed_primary_id() {
    assert!(member_id::is_valid_member_id("BSDS-2026-0001-00"));
}

#[test]
fn is_valid_member_id_returns_true_for_sub_member_id() {
    assert!(member_id::is_valid_member_id("BSDS-2026-0042-03"));
}

#[test]
fn is_valid_member_id_returns_false_for_wrong_year_length() {
    assert!(!member_id::is_valid_member_id("BSDS-26-0001-00"));
}

#[test]
fn is_valid_member_id_returns_false_for_wrong_sequence_length() {
    assert!(!member_id::is_valid_member_id("BSDS-2026-001-00"));
}

#[test]
fn is_valid_member_id_returns_false_for_completely_invalid_string() {
    assert!(!member_id::is_valid_member_id("invalid"));
    assert!(!member_id::is_valid_member_id(""));
    assert!(!member_id::is_valid_member_id("BSDS-2026-0001"));
}

// ---------------------------------------------------------------------------
// parse_sequence_number
// ---------------------------------------------------------------------------

#[test]
fn parse_sequence_number_extracts_numeric_nnnn_segment() {
    assert_eq!(member_id::parse_sequence_number("BSDS-2026-0042-00"), Some(42));
    assert_eq!(member_id::parse_sequence_number("BSDS-2026-0001-00"), Some(1));
    assert_eq!(member_id::parse_sequence_number("BSDS-2026-9999-03"), Some(9999));
}

#[test]
fn parse_sequence_number_returns_none_for_invalid_format() {
    assert_eq!(member_id::parse_sequence_number("invalid"), None);
    assert_eq!(member_id::parse_sequence_number(""), None);
    assert_eq!(member_id::parse_sequence_number("BSDS-2026-0001"), None);
}

// ---------------------------------------------------------------------------
// Receipt reference helpers
// ---------------------------------------------------------------------------

#[test]
fn build_member_order_receipt_reference_length_within_40_chars() {
    let ref_str = member_id::build_member_order_receipt_reference("BSDS-2026-0001-00", 1711234567);
    assert!(ref_str.len() <= 40, "receipt ref too long: {}", ref_str.len());
}

#[test]
fn build_sponsor_order_receipt_reference_length_within_40_chars() {
    let ref_str = member_id::build_sponsor_order_receipt_reference("abc123xyz", 1711234567);
    assert!(ref_str.len() <= 40, "sponsor ref too long: {}", ref_str.len());
}

#[test]
fn build_sponsor_payment_fallback_receipt_includes_payment_id_segment() {
    let ref_str = member_id::build_sponsor_payment_fallback_receipt("pay_abcdefghijk");
    assert!(ref_str.contains("BSDS-PAY-"), "should include BSDS-PAY- prefix");
}

// ---------------------------------------------------------------------------
// member_id_prefix helper
// ---------------------------------------------------------------------------

#[test]
fn member_id_prefix_includes_brand_and_year() {
    let prefix = member_id::member_id_prefix(2026);
    assert_eq!(prefix, "BSDS-2026-");
}

#[test]
fn receipt_number_prefix_includes_brand_rec_and_year() {
    let prefix = member_id::receipt_number_prefix(2026);
    assert_eq!(prefix, "BSDS-REC-2026-");
}
