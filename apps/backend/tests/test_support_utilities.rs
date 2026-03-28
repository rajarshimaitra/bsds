//! Consolidated tests for support utilities.
//!
//! Combines tests from three support modules into a single file:
//! - `support/encrypt.rs` — encryption, decryption, and conditional helpers
//! - `support/member_id.rs` — member ID generation, validation, and parsing
//! - `support/receipt.rs` — receipt formatting, amount-to-words, and breakdown parsing

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use bsds_backend::support::encrypt::{
    decrypt, decrypt_if_needed, encrypt, encrypt_if_needed, is_encrypted, EncryptError,
};
use bsds_backend::support::member_id;
use bsds_backend::support::membership_rules::MembershipType;
use bsds_backend::support::receipt::{
    amount_to_words, build_membership_summary, category_label, is_receipt_eligible,
    membership_type_label, parse_breakdown, payment_mode_label, sponsor_purpose_label,
    MembershipDetails,
};

// ===========================================================================
// Encryption (support/encrypt.rs)
// ===========================================================================

fn test_key() -> [u8; 32] {
    [0xABu8; 32]
}

fn alt_key() -> [u8; 32] {
    [0xCDu8; 32]
}

#[test]
fn encrypt_then_decrypt_returns_original_plaintext() {
    let key = test_key();
    let plaintext = "hello world — this is a test";
    let encrypted = encrypt(plaintext, &key).unwrap();
    let decrypted = decrypt(&encrypted, &key).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn round_trip_preserves_unicode_and_special_characters() {
    let key = test_key();
    let plaintext = "नमस्ते 🌏 <>&\"'\\";
    let encrypted = encrypt(plaintext, &key).unwrap();
    let decrypted = decrypt(&encrypted, &key).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn round_trip_works_for_empty_string() {
    let key = test_key();
    let plaintext = "";
    let encrypted = encrypt(plaintext, &key).unwrap();
    let decrypted = decrypt(&encrypted, &key).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn encrypted_value_starts_with_enc_prefix() {
    let key = test_key();
    let encrypted = encrypt("hello", &key).unwrap();
    assert!(encrypted.starts_with("enc:"));
}

#[test]
fn is_encrypted_returns_true_for_enc_prefixed_value() {
    assert!(is_encrypted("enc:some_base64_here"));
}

#[test]
fn is_encrypted_returns_false_for_plaintext() {
    assert!(!is_encrypted("plaintext"));
    assert!(!is_encrypted(""));
}

#[test]
fn same_plaintext_encrypts_to_different_ciphertexts() {
    let key = test_key();
    let a = encrypt("same input", &key).unwrap();
    let b = encrypt("same input", &key).unwrap();
    assert_ne!(a, b);
}

#[test]
fn decrypt_plaintext_without_enc_prefix_returns_not_encrypted_error() {
    let key = test_key();
    let err = decrypt("plaintext", &key).unwrap_err();
    assert!(matches!(err, EncryptError::NotEncrypted));
}

#[test]
fn decrypt_with_wrong_key_returns_decryption_failed_error() {
    let key = test_key();
    let wrong_key = alt_key();
    let encrypted = encrypt("secret", &key).unwrap();
    let err = decrypt(&encrypted, &wrong_key).unwrap_err();
    assert!(matches!(err, EncryptError::DecryptionFailed));
}

#[test]
fn decrypt_truncated_blob_returns_blob_too_short_error() {
    let key = test_key();
    let tiny = BASE64.encode([0u8; 5]);
    let bad = format!("enc:{tiny}");
    let err = decrypt(&bad, &key).unwrap_err();
    assert!(matches!(err, EncryptError::BlobTooShort { .. }));
}

#[test]
fn encrypt_if_needed_encrypts_plaintext_value() {
    let key = test_key();
    let result = encrypt_if_needed(Some("plain"), &key).unwrap();
    assert!(is_encrypted(result.as_deref().unwrap()));
}

#[test]
fn encrypt_if_needed_skips_already_encrypted_value() {
    let key = test_key();
    let encrypted = encrypt("value", &key).unwrap();
    let result = encrypt_if_needed(Some(&encrypted), &key).unwrap().unwrap();
    assert_eq!(result, encrypted);
}

#[test]
fn encrypt_if_needed_returns_none_for_none_input() {
    let key = test_key();
    let result = encrypt_if_needed(None, &key).unwrap();
    assert!(result.is_none());
}

#[test]
fn decrypt_if_needed_decrypts_enc_prefixed_value() {
    let key = test_key();
    let original = "secret value";
    let encrypted = encrypt(original, &key).unwrap();
    let result = decrypt_if_needed(Some(&encrypted), &key);
    assert_eq!(result.unwrap(), original);
}

#[test]
fn decrypt_if_needed_returns_plaintext_unchanged() {
    let key = test_key();
    let result = decrypt_if_needed(Some("already plain"), &key);
    assert_eq!(result.unwrap(), "already plain");
}

#[test]
fn decrypt_if_needed_returns_none_for_none_input() {
    let key = test_key();
    let result = decrypt_if_needed(None, &key);
    assert!(result.is_none());
}

// ===========================================================================
// Member ID (support/member_id.rs)
// ===========================================================================

#[test]
fn build_member_id_formats_year_and_sequence_and_suffix_correctly() {
    assert_eq!(member_id::build_member_id(2026, 1, 0), "BSDS-2026-0001-00");
}

#[test]
fn build_member_id_zero_pads_sequence_to_four_digits() {
    assert_eq!(
        member_id::build_member_id(2026, 42, 0),
        "BSDS-2026-0042-00"
    );
    assert_eq!(
        member_id::build_member_id(2026, 9999, 0),
        "BSDS-2026-9999-00"
    );
}

#[test]
fn build_member_id_formats_sub_member_suffix_correctly() {
    assert_eq!(
        member_id::build_member_id(2026, 1, 1),
        "BSDS-2026-0001-01"
    );
    assert_eq!(
        member_id::build_member_id(2026, 1, 3),
        "BSDS-2026-0001-03"
    );
}

#[test]
fn generate_member_id_with_no_existing_members_starts_at_0001() {
    assert_eq!(
        member_id::generate_member_id(2026, None),
        "BSDS-2026-0001-00"
    );
}

#[test]
fn generate_member_id_increments_from_current_max() {
    assert_eq!(
        member_id::generate_member_id(2026, Some(5)),
        "BSDS-2026-0006-00"
    );
}

#[test]
fn generate_member_id_uses_correct_year() {
    assert!(member_id::generate_member_id(2025, None).starts_with("BSDS-2025-"));
    assert!(member_id::generate_member_id(2026, None).starts_with("BSDS-2026-"));
}

#[test]
fn generate_member_id_always_ends_with_00_suffix() {
    assert!(member_id::generate_member_id(2026, Some(10)).ends_with("-00"));
}

#[test]
fn generate_sub_member_id_replaces_last_two_chars_with_index() {
    assert_eq!(
        member_id::generate_sub_member_id("BSDS-2026-0001-00", 1).unwrap(),
        "BSDS-2026-0001-01"
    );
}

#[test]
fn generate_sub_member_id_index_2_produces_02_suffix() {
    assert_eq!(
        member_id::generate_sub_member_id("BSDS-2026-0001-00", 2).unwrap(),
        "BSDS-2026-0001-02"
    );
}

#[test]
fn generate_sub_member_id_index_3_produces_03_suffix() {
    assert_eq!(
        member_id::generate_sub_member_id("BSDS-2026-0001-00", 3).unwrap(),
        "BSDS-2026-0001-03"
    );
}

#[test]
fn generate_sub_member_id_index_0_returns_error() {
    assert!(member_id::generate_sub_member_id("BSDS-2026-0001-00", 0).is_err());
}

#[test]
fn generate_sub_member_id_index_4_returns_error() {
    assert!(member_id::generate_sub_member_id("BSDS-2026-0001-00", 4).is_err());
}

#[test]
fn generate_sub_member_id_too_short_parent_returns_error() {
    assert!(member_id::generate_sub_member_id("X", 1).is_err());
}

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

#[test]
fn parse_sequence_number_extracts_numeric_nnnn_segment() {
    assert_eq!(
        member_id::parse_sequence_number("BSDS-2026-0042-00"),
        Some(42)
    );
    assert_eq!(
        member_id::parse_sequence_number("BSDS-2026-0001-00"),
        Some(1)
    );
    assert_eq!(
        member_id::parse_sequence_number("BSDS-2026-9999-03"),
        Some(9999)
    );
}

#[test]
fn parse_sequence_number_returns_none_for_invalid_format() {
    assert_eq!(member_id::parse_sequence_number("invalid"), None);
    assert_eq!(member_id::parse_sequence_number(""), None);
    assert_eq!(member_id::parse_sequence_number("BSDS-2026-0001"), None);
}

#[test]
fn build_member_order_receipt_reference_length_within_40_chars() {
    let reference =
        member_id::build_member_order_receipt_reference("BSDS-2026-0001-00", 1711234567);
    assert!(reference.len() <= 40);
}

#[test]
fn build_sponsor_order_receipt_reference_length_within_40_chars() {
    let reference =
        member_id::build_sponsor_order_receipt_reference("abc123xyz", 1711234567);
    assert!(reference.len() <= 40);
}

#[test]
fn build_sponsor_payment_fallback_receipt_includes_payment_id_segment() {
    let reference =
        member_id::build_sponsor_payment_fallback_receipt("pay_abcdefghijk");
    assert!(reference.contains("BSDS-PAY-"));
}

#[test]
fn member_id_prefix_includes_brand_and_year() {
    assert_eq!(member_id::member_id_prefix(2026), "BSDS-2026-");
}

#[test]
fn receipt_number_prefix_includes_brand_rec_and_year() {
    assert_eq!(
        member_id::receipt_number_prefix(2026),
        "BSDS-REC-2026-"
    );
}

// ===========================================================================
// Receipt Formatting (support/receipt.rs)
// ===========================================================================

#[test]
fn amount_to_words_zero_returns_zero_rupees_only() {
    assert_eq!(amount_to_words(0.0), "Zero Rupees Only");
}

#[test]
fn amount_to_words_negative_returns_invalid_amount() {
    assert_eq!(amount_to_words(-1.0), "Invalid Amount");
}

#[test]
fn amount_to_words_1500() {
    assert_eq!(
        amount_to_words(1500.0),
        "One Thousand Five Hundred Rupees Only"
    );
}

#[test]
fn amount_to_words_250() {
    assert_eq!(
        amount_to_words(250.0),
        "Two Hundred and Fifty Rupees Only"
    );
}

#[test]
fn amount_to_words_10000() {
    assert_eq!(amount_to_words(10000.0), "Ten Thousand Rupees Only");
}

#[test]
fn amount_to_words_one_lakh() {
    let result = amount_to_words(100000.0);
    assert!(result.contains("Lakh"));
}

#[test]
fn amount_to_words_250000() {
    assert_eq!(
        amount_to_words(250000.0),
        "Two Lakh Fifty Thousand Rupees Only"
    );
}

#[test]
fn amount_to_words_one_crore() {
    assert_eq!(
        amount_to_words(10000000.0),
        "One Crore Rupees Only"
    );
}

#[test]
fn amount_to_words_with_paise() {
    let result = amount_to_words(1250.50);
    assert!(result.contains("Paise"));
    assert!(result.contains("Fifty Paise"));
}

#[test]
fn payment_mode_label_upi() {
    assert_eq!(payment_mode_label("UPI"), "UPI");
}

#[test]
fn payment_mode_label_bank_transfer() {
    assert_eq!(payment_mode_label("BANK_TRANSFER"), "Bank Transfer");
}

#[test]
fn payment_mode_label_cash() {
    assert_eq!(payment_mode_label("CASH"), "Cash");
}

#[test]
fn payment_mode_label_unknown_returns_unchanged() {
    assert_eq!(payment_mode_label("CHEQUE"), "CHEQUE");
}

#[test]
fn category_label_membership() {
    assert_eq!(category_label("MEMBERSHIP"), "Membership");
}

#[test]
fn category_label_sponsorship() {
    assert_eq!(category_label("SPONSORSHIP"), "Sponsorship");
}

#[test]
fn category_label_expense() {
    assert_eq!(category_label("EXPENSE"), "Expense");
}

#[test]
fn category_label_other() {
    assert_eq!(category_label("OTHER"), "Other");
}

#[test]
fn membership_type_label_monthly() {
    assert_eq!(
        membership_type_label(MembershipType::Monthly),
        "Monthly Subscription"
    );
}

#[test]
fn membership_type_label_half_yearly() {
    assert_eq!(
        membership_type_label(MembershipType::HalfYearly),
        "Half-yearly Subscription"
    );
}

#[test]
fn membership_type_label_annual() {
    assert_eq!(
        membership_type_label(MembershipType::Annual),
        "Annual Subscription"
    );
}

#[test]
fn sponsor_purpose_label_title_sponsor() {
    assert_eq!(
        sponsor_purpose_label(Some("TITLE_SPONSOR")),
        "Title Sponsor"
    );
}

#[test]
fn sponsor_purpose_label_gold_sponsor() {
    assert_eq!(
        sponsor_purpose_label(Some("GOLD_SPONSOR")),
        "Gold Sponsor"
    );
}

#[test]
fn sponsor_purpose_label_none_returns_empty() {
    assert_eq!(sponsor_purpose_label(None), "");
}

#[test]
fn sponsor_purpose_label_empty_string_returns_empty() {
    assert_eq!(sponsor_purpose_label(Some("")), "");
}

#[test]
fn sponsor_purpose_label_custom_returns_unchanged() {
    assert_eq!(
        sponsor_purpose_label(Some("CUSTOM_PURPOSE")),
        "CUSTOM_PURPOSE"
    );
}

#[test]
fn is_receipt_eligible_membership_cash_in() {
    assert!(is_receipt_eligible("MEMBERSHIP", "CASH_IN"));
}

#[test]
fn is_receipt_eligible_sponsorship_cash_in() {
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
fn is_receipt_eligible_other_cash_in_returns_false() {
    assert!(!is_receipt_eligible("OTHER", "CASH_IN"));
}

#[test]
fn build_membership_summary_non_membership_returns_none() {
    let result = build_membership_summary("SPONSORSHIP", 500.0, None);
    assert!(result.is_none());
}

#[test]
fn build_membership_summary_fallback_no_details() {
    let (purpose, items) = build_membership_summary("MEMBERSHIP", 500.0, None).unwrap();
    assert_eq!(purpose, "Membership");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].amount, 500.0);
    assert!(!items[0].label.is_empty());
}

#[test]
fn build_membership_summary_subscription_only() {
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
fn build_membership_summary_annual_fee_only() {
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
fn build_membership_summary_annual_and_subscription() {
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
fn build_membership_summary_application_fee() {
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

#[test]
fn parse_breakdown_none_returns_none() {
    let result = parse_breakdown(None);
    assert!(result.is_none());
}

#[test]
fn parse_breakdown_empty_array_returns_none() {
    let value = serde_json::json!([]);
    let result = parse_breakdown(Some(&value));
    assert!(result.is_none());
}

#[test]
fn parse_breakdown_non_array_returns_none() {
    let value = serde_json::json!({"not": "an array"});
    let result = parse_breakdown(Some(&value));
    assert!(result.is_none());
}

#[test]
fn parse_breakdown_valid_array() {
    let value = serde_json::json!([
        {"label": "Subscription", "amount": 300.0},
        {"label": "Annual Fee", "amount": 500.0}
    ]);
    let result = parse_breakdown(Some(&value)).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].label, "Subscription");
    assert_eq!(result[0].amount, 300.0);
    assert_eq!(result[1].label, "Annual Fee");
    assert_eq!(result[1].amount, 500.0);
}

#[test]
fn parse_breakdown_skips_malformed() {
    let value = serde_json::json!([
        {"label": "Valid", "amount": 100.0},
        {"bad": "entry"}
    ]);
    let result = parse_breakdown(Some(&value)).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "Valid");
}
