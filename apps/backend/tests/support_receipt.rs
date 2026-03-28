//! # Receipt Support Tests
//!
//! Covers: amount_to_words (zero, basic amounts, lakhs, crores, paise),
//!         payment_mode_label, category_label, membership_type_label,
//!         sponsor_purpose_label, is_receipt_eligible, build_membership_summary
//!         (with details, fallback, non-membership category), parse_breakdown.
//!
//! Does NOT cover: PDF rendering, DB-level receipt operations (repository layer),
//!                 receipt number sequence generation (covered in support_member_id.rs).
//!
//! Protects: apps/backend/src/support/receipt.rs

use bsds_backend::support::receipt::{
    amount_to_words, build_membership_summary, category_label, is_receipt_eligible,
    membership_type_label, parse_breakdown, payment_mode_label, sponsor_purpose_label,
    BreakdownItem, MembershipDetails,
};
use bsds_backend::support::membership_rules::MembershipType;

// ---------------------------------------------------------------------------
// amount_to_words
// ---------------------------------------------------------------------------

#[test]
fn amount_to_words_zero_returns_zero_rupees_only() {
    assert_eq!(amount_to_words(0.0), "Zero Rupees Only");
}

#[test]
fn amount_to_words_negative_returns_invalid_amount() {
    assert_eq!(amount_to_words(-1.0), "Invalid Amount");
}

#[test]
fn amount_to_words_1500_returns_one_thousand_five_hundred() {
    assert_eq!(amount_to_words(1500.0), "One Thousand Five Hundred Rupees Only");
}

#[test]
fn amount_to_words_250_returns_two_hundred_fifty() {
    assert_eq!(amount_to_words(250.0), "Two Hundred and Fifty Rupees Only");
}

#[test]
fn amount_to_words_10000_returns_ten_thousand() {
    assert_eq!(amount_to_words(10000.0), "Ten Thousand Rupees Only");
}

#[test]
fn amount_to_words_one_lakh_uses_lakh_denomination() {
    let result = amount_to_words(100000.0);
    assert!(result.contains("Lakh"), "should use Lakh denomination: {result}");
}

#[test]
fn amount_to_words_250000_returns_two_lakh_fifty_thousand() {
    assert_eq!(amount_to_words(250000.0), "Two Lakh Fifty Thousand Rupees Only");
}

#[test]
fn amount_to_words_one_crore_uses_crore_denomination() {
    assert_eq!(amount_to_words(10000000.0), "One Crore Rupees Only");
}

#[test]
fn amount_to_words_with_paise_includes_paise_suffix() {
    let result = amount_to_words(1250.50);
    assert!(result.contains("Paise"), "should include Paise: {result}");
    assert!(result.contains("Fifty Paise"), "should show 50 paise: {result}");
}

// ---------------------------------------------------------------------------
// payment_mode_label
// ---------------------------------------------------------------------------

#[test]
fn payment_mode_label_upi_returns_upi() {
    assert_eq!(payment_mode_label("UPI"), "UPI");
}

#[test]
fn payment_mode_label_bank_transfer_returns_bank_transfer() {
    assert_eq!(payment_mode_label("BANK_TRANSFER"), "Bank Transfer");
}

#[test]
fn payment_mode_label_cash_returns_cash() {
    assert_eq!(payment_mode_label("CASH"), "Cash");
}

#[test]
fn payment_mode_label_unknown_returns_input_unchanged() {
    assert_eq!(payment_mode_label("CRYPTO"), "CRYPTO");
}

// ---------------------------------------------------------------------------
// category_label
// ---------------------------------------------------------------------------

#[test]
fn category_label_membership_returns_membership() {
    assert_eq!(category_label("MEMBERSHIP"), "Membership");
}

#[test]
fn category_label_sponsorship_returns_sponsorship() {
    assert_eq!(category_label("SPONSORSHIP"), "Sponsorship");
}

#[test]
fn category_label_expense_returns_expense() {
    assert_eq!(category_label("EXPENSE"), "Expense");
}

#[test]
fn category_label_other_returns_other() {
    assert_eq!(category_label("OTHER"), "Other");
}

// ---------------------------------------------------------------------------
// membership_type_label
// ---------------------------------------------------------------------------

#[test]
fn membership_type_label_monthly_returns_monthly_subscription() {
    assert_eq!(membership_type_label(MembershipType::Monthly), "Monthly Subscription");
}

#[test]
fn membership_type_label_half_yearly_returns_half_yearly_subscription() {
    assert_eq!(membership_type_label(MembershipType::HalfYearly), "Half-yearly Subscription");
}

#[test]
fn membership_type_label_annual_returns_annual_subscription() {
    assert_eq!(membership_type_label(MembershipType::Annual), "Annual Subscription");
}

// ---------------------------------------------------------------------------
// sponsor_purpose_label
// ---------------------------------------------------------------------------

#[test]
fn sponsor_purpose_label_title_sponsor_returns_title_sponsor() {
    assert_eq!(sponsor_purpose_label(Some("TITLE_SPONSOR")), "Title Sponsor");
}

#[test]
fn sponsor_purpose_label_gold_returns_gold_sponsor() {
    assert_eq!(sponsor_purpose_label(Some("GOLD_SPONSOR")), "Gold Sponsor");
}

#[test]
fn sponsor_purpose_label_none_returns_empty_string() {
    assert_eq!(sponsor_purpose_label(None), "");
}

#[test]
fn sponsor_purpose_label_empty_string_returns_empty_string() {
    assert_eq!(sponsor_purpose_label(Some("")), "");
}

#[test]
fn sponsor_purpose_label_custom_value_returned_unchanged() {
    assert_eq!(sponsor_purpose_label(Some("CUSTOM_VALUE")), "CUSTOM_VALUE");
}

// ---------------------------------------------------------------------------
// is_receipt_eligible
// ---------------------------------------------------------------------------

#[test]
fn is_receipt_eligible_membership_cash_in_returns_true() {
    assert!(is_receipt_eligible("MEMBERSHIP", "CASH_IN"));
}

#[test]
fn is_receipt_eligible_sponsorship_cash_in_returns_true() {
    assert!(is_receipt_eligible("SPONSORSHIP", "CASH_IN"));
}

#[test]
fn is_receipt_eligible_expense_cash_in_returns_false() {
    assert!(!is_receipt_eligible("EXPENSE", "CASH_IN"));
}

#[test]
fn is_receipt_eligible_membership_cash_out_returns_false() {
    assert!(!is_receipt_eligible("MEMBERSHIP", "CASH_OUT"));
}

#[test]
fn is_receipt_eligible_other_category_returns_false() {
    assert!(!is_receipt_eligible("OTHER", "CASH_IN"));
}

// ---------------------------------------------------------------------------
// build_membership_summary
// ---------------------------------------------------------------------------

#[test]
fn build_membership_summary_returns_none_for_non_membership_category() {
    let result = build_membership_summary("SPONSORSHIP", 1000.0, None);
    assert!(result.is_none());
}

#[test]
fn build_membership_summary_fallback_when_no_details_provided() {
    let (purpose, items) = build_membership_summary("MEMBERSHIP", 500.0, None).unwrap();
    assert_eq!(purpose, "Membership");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].amount, 500.0);
    assert_eq!(items[0].label, "Membership");
}

#[test]
fn build_membership_summary_with_subscription_only() {
    let details = MembershipDetails {
        membership_type: Some(MembershipType::Monthly),
        includes_subscription: true,
        includes_annual_fee: false,
        includes_application_fee: false,
        is_application_fee: false,
    };
    let (purpose, items) = build_membership_summary("MEMBERSHIP", 250.0, Some(&details)).unwrap();
    assert_eq!(items.len(), 1);
    assert!(purpose.contains("Monthly Subscription"));
}

#[test]
fn build_membership_summary_with_annual_fee_only() {
    let details = MembershipDetails {
        membership_type: None,
        includes_subscription: false,
        includes_annual_fee: true,
        includes_application_fee: false,
        is_application_fee: false,
    };
    let (purpose, items) = build_membership_summary("MEMBERSHIP", 5000.0, Some(&details)).unwrap();
    assert_eq!(items.len(), 1);
    assert!(purpose.contains("Annual Membership Fee"));
}

#[test]
fn build_membership_summary_with_annual_fee_and_subscription() {
    let details = MembershipDetails {
        membership_type: Some(MembershipType::Monthly),
        includes_subscription: true,
        includes_annual_fee: true,
        includes_application_fee: false,
        is_application_fee: false,
    };
    let (purpose, items) = build_membership_summary("MEMBERSHIP", 5250.0, Some(&details)).unwrap();
    assert_eq!(items.len(), 2, "should have 2 line items");
    assert!(purpose.contains("Annual Membership Fee"));
    assert!(purpose.contains("Monthly Subscription"));
}

#[test]
fn build_membership_summary_with_application_fee_flag() {
    let details = MembershipDetails {
        membership_type: None,
        includes_subscription: false,
        includes_annual_fee: false,
        includes_application_fee: true,
        is_application_fee: false,
    };
    let (purpose, items) = build_membership_summary("MEMBERSHIP", 10000.0, Some(&details)).unwrap();
    assert_eq!(items.len(), 1);
    assert!(purpose.contains("Application Fee"));
}

// ---------------------------------------------------------------------------
// parse_breakdown
// ---------------------------------------------------------------------------

#[test]
fn parse_breakdown_returns_none_for_none_input() {
    assert!(parse_breakdown(None).is_none());
}

#[test]
fn parse_breakdown_returns_none_for_empty_array() {
    let json = serde_json::json!([]);
    assert!(parse_breakdown(Some(&json)).is_none());
}

#[test]
fn parse_breakdown_returns_none_for_non_array_value() {
    let json = serde_json::json!("not an array");
    assert!(parse_breakdown(Some(&json)).is_none());
}

#[test]
fn parse_breakdown_parses_valid_breakdown_array() {
    let json = serde_json::json!([
        { "label": "Monthly Subscription", "amount": 250.0 },
        { "label": "Application Fee", "amount": 10000.0 }
    ]);
    let items = parse_breakdown(Some(&json)).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].label, "Monthly Subscription");
    assert_eq!(items[0].amount, 250.0);
    assert_eq!(items[1].label, "Application Fee");
    assert_eq!(items[1].amount, 10000.0);
}

#[test]
fn parse_breakdown_skips_malformed_entries_silently() {
    let json = serde_json::json!([
        { "label": "Valid Item", "amount": 100.0 },
        { "no_label_field": true, "amount": 200.0 }
    ]);
    let items = parse_breakdown(Some(&json)).unwrap();
    assert_eq!(items.len(), 1, "malformed entry should be silently skipped");
    assert_eq!(items[0].label, "Valid Item");
}
