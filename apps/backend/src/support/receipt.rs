//! Receipt formatting, data structures, and utility functions.
//!
//! Merges both `receipt.ts` (server helpers) and `receipt-utils.ts` (shared
//! constants and amount-to-words conversion) from the TypeScript codebase.
//!
//! DB operations (issue, cancel, fetch) are handled by the repository layer.
//! This module provides:
//!   - Club constants
//!   - `ReceiptData` struct
//!   - Label mapping helpers
//!   - Breakdown building for membership and sponsorship receipts
//!   - `amount_to_words` — Indian English rupee conversion

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::support::membership_rules::{
    self, MembershipType, ANNUAL_MEMBERSHIP_FEE, APPLICATION_FEE,
};

// ---------------------------------------------------------------------------
// Club constants
// ---------------------------------------------------------------------------

pub const CLUB_NAME: &str = "Deshapriya Park Sarbojanin Durgotsav";
pub const CLUB_ADDRESS: &str =
    "Deshapriya Park, Bhawanipur, Kolkata - 700 026, West Bengal";
pub const CLUB_PHONE: &str = "+91 98300 XXXXX";

// ---------------------------------------------------------------------------
// Receipt data shape
// ---------------------------------------------------------------------------

/// A single line-item in the receipt breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownItem {
    pub label: String,
    pub amount: f64,
}

/// Complete receipt data for rendering or API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptData {
    pub receipt_number: String,
    pub date: String,
    pub status: String,
    /// `"MEMBER"` or `"SPONSOR"`
    #[serde(rename = "type")]
    pub r#type: String,

    /// Human-readable purpose summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Itemized breakdown of the paid components.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakdown: Option<Vec<BreakdownItem>>,

    // Member-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_name: Option<String>,
    /// BSDS-YYYY-NNNN-SS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub membership_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub membership_end: Option<String>,

    // Sponsor-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sponsor_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sponsor_company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sponsor_purpose: Option<String>,

    // Common fields
    pub amount: f64,
    pub payment_mode: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    pub received_by: String,
    pub club_name: String,
    pub club_address: String,
}

// ---------------------------------------------------------------------------
// Label helpers
// ---------------------------------------------------------------------------

/// Human-readable payment mode label.
pub fn payment_mode_label(mode: &str) -> &str {
    match mode {
        "UPI" => "UPI",
        "BANK_TRANSFER" => "Bank Transfer",
        "CASH" => "Cash",
        _ => mode,
    }
}

/// Human-readable category label.
pub fn category_label(category: &str) -> &str {
    match category {
        "MEMBERSHIP" => "Membership",
        "SPONSORSHIP" => "Sponsorship",
        "EXPENSE" => "Expense",
        "OTHER" => "Other",
        _ => category,
    }
}

/// Human-readable membership type label.
pub fn membership_type_label(membership_type: MembershipType) -> &'static str {
    match membership_type {
        MembershipType::Monthly => "Monthly Subscription",
        MembershipType::HalfYearly => "Half-yearly Subscription",
        MembershipType::Annual => "Annual Subscription",
    }
}

/// Human-readable sponsor purpose label.
pub fn sponsor_purpose_label(purpose: Option<&str>) -> &str {
    match purpose {
        None | Some("") => "",
        Some("TITLE_SPONSOR") => "Title Sponsor",
        Some("GOLD_SPONSOR") => "Gold Sponsor",
        Some("SILVER_SPONSOR") => "Silver Sponsor",
        Some("FOOD_PARTNER") => "Food Partner",
        Some("MEDIA_PARTNER") => "Media Partner",
        Some("STALL_VENDOR") => "Stall Vendor",
        Some("MARKETING_PARTNER") => "Marketing Partner",
        Some(other) => other,
    }
}

// ---------------------------------------------------------------------------
// Receipt eligibility
// ---------------------------------------------------------------------------

/// Returns `true` if the transaction category and type qualify for a receipt.
pub fn is_receipt_eligible(category: &str, tx_type: &str) -> bool {
    tx_type == "CASH_IN" && (category == "MEMBERSHIP" || category == "SPONSORSHIP")
}

// ---------------------------------------------------------------------------
// Breakdown builders
// ---------------------------------------------------------------------------

/// Membership details needed for breakdown building.
/// Mirrors `TransactionMembershipDetails` from the TypeScript service.
#[derive(Debug, Clone, Default)]
pub struct MembershipDetails {
    pub membership_type: Option<MembershipType>,
    pub is_application_fee: bool,
    pub includes_subscription: bool,
    pub includes_annual_fee: bool,
    pub includes_application_fee: bool,
}

/// Build the purpose and breakdown for a membership receipt.
///
/// Returns `None` if the category is not `"MEMBERSHIP"`.
pub fn build_membership_summary(
    category: &str,
    amount: f64,
    details: Option<&MembershipDetails>,
) -> Option<(String, Vec<BreakdownItem>)> {
    if category != "MEMBERSHIP" {
        return None;
    }

    let mut items: Vec<BreakdownItem> = Vec::new();
    let mut labels: Vec<String> = Vec::new();

    if let Some(d) = details {
        if d.includes_annual_fee {
            let fee = ANNUAL_MEMBERSHIP_FEE as f64;
            items.push(BreakdownItem {
                label: "Annual Membership Fee".to_string(),
                amount: fee,
            });
            labels.push("Annual Membership Fee".to_string());
        }

        if d.includes_subscription {
            if let Some(mt) = d.membership_type {
                let fee = membership_rules::membership_fee(mt) as f64;
                let label = membership_type_label(mt).to_string();
                items.push(BreakdownItem {
                    label: label.clone(),
                    amount: fee,
                });
                labels.push(label);
            }
        }

        if d.is_application_fee || d.includes_application_fee {
            let fee = APPLICATION_FEE as f64;
            items.push(BreakdownItem {
                label: "Application Fee".to_string(),
                amount: fee,
            });
            labels.push("Application Fee".to_string());
        }
    }

    if !items.is_empty() {
        let purpose = labels.join(", ");
        return Some((purpose, items));
    }

    // Fallback: single line with total
    Some((
        "Membership".to_string(),
        vec![BreakdownItem {
            label: "Membership".to_string(),
            amount,
        }],
    ))
}

// ---------------------------------------------------------------------------
// Breakdown parsing from JSON
// ---------------------------------------------------------------------------

/// Parse a breakdown from a JSON value (as stored in the DB).
pub fn parse_breakdown(value: Option<&JsonValue>) -> Option<Vec<BreakdownItem>> {
    let arr = value?.as_array()?;
    let items: Vec<BreakdownItem> = arr
        .iter()
        .filter_map(|entry| {
            let obj = entry.as_object()?;
            let label = obj.get("label")?.as_str()?.to_string();
            let amount = obj.get("amount")?.as_f64()?;
            Some(BreakdownItem { label, amount })
        })
        .collect();

    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

// ---------------------------------------------------------------------------
// Amount to words — Indian English (handles up to crores)
// ---------------------------------------------------------------------------

const ONES: &[&str] = &[
    "", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine",
    "Ten", "Eleven", "Twelve", "Thirteen", "Fourteen", "Fifteen", "Sixteen",
    "Seventeen", "Eighteen", "Nineteen",
];

const TENS: &[&str] = &[
    "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty",
    "Ninety",
];

fn three_digit_words(n: u64) -> String {
    if n == 0 {
        return String::new();
    }
    if n < 20 {
        return ONES[n as usize].to_string();
    }
    if n < 100 {
        let tens = TENS[(n / 10) as usize];
        let ones = n % 10;
        if ones != 0 {
            return format!("{tens} {}", ONES[ones as usize]);
        }
        return tens.to_string();
    }
    let hundreds = n / 100;
    let remainder = n % 100;
    let mut result = format!("{} Hundred", ONES[hundreds as usize]);
    if remainder != 0 {
        result.push_str(" and ");
        result.push_str(&three_digit_words(remainder));
    }
    result
}

/// Convert a rupee amount into Indian English words.
///
/// Examples:
///   - `1500`     -> `"One Thousand Five Hundred Rupees Only"`
///   - `250000`   -> `"Two Lakh Fifty Thousand Rupees Only"`
///   - `1250.50`  -> `"One Thousand Two Hundred and Fifty Rupees and Fifty Paise Only"`
pub fn amount_to_words(amount: f64) -> String {
    if amount < 0.0 {
        return "Invalid Amount".to_string();
    }
    if amount == 0.0 {
        return "Zero Rupees Only".to_string();
    }

    let rupees = amount.floor() as u64;
    let paise = ((amount - amount.floor()) * 100.0).round() as u64;

    let mut result = String::new();

    if rupees > 0 {
        let crore = rupees / 10_000_000;
        let lakh = (rupees % 10_000_000) / 100_000;
        let thousand = (rupees % 100_000) / 1_000;
        let remainder = rupees % 1_000;

        if crore > 0 {
            result.push_str(&three_digit_words(crore));
            result.push_str(" Crore ");
        }
        if lakh > 0 {
            result.push_str(&three_digit_words(lakh));
            result.push_str(" Lakh ");
        }
        if thousand > 0 {
            result.push_str(&three_digit_words(thousand));
            result.push_str(" Thousand ");
        }
        if remainder > 0 {
            result.push_str(&three_digit_words(remainder));
            result.push(' ');
        }

        result = result.trim_end().to_string();
        result.push_str(" Rupees");
    }

    if paise > 0 {
        result.push_str(" and ");
        result.push_str(&three_digit_words(paise));
        result.push_str(" Paise");
    }

    result = result.trim().to_string();
    result.push_str(" Only");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_to_words_basic() {
        assert_eq!(amount_to_words(0.0), "Zero Rupees Only");
        assert_eq!(amount_to_words(-5.0), "Invalid Amount");
        assert_eq!(
            amount_to_words(1500.0),
            "One Thousand Five Hundred Rupees Only"
        );
        assert_eq!(
            amount_to_words(10000.0),
            "Ten Thousand Rupees Only"
        );
    }

    #[test]
    fn test_amount_to_words_lakh_crore() {
        assert_eq!(
            amount_to_words(250000.0),
            "Two Lakh Fifty Thousand Rupees Only"
        );
        assert_eq!(
            amount_to_words(10000000.0),
            "One Crore Rupees Only"
        );
    }

    #[test]
    fn test_amount_to_words_paise() {
        assert_eq!(
            amount_to_words(1250.50),
            "One Thousand Two Hundred and Fifty Rupees and Fifty Paise Only"
        );
    }

    #[test]
    fn test_payment_mode_label() {
        assert_eq!(payment_mode_label("UPI"), "UPI");
        assert_eq!(payment_mode_label("BANK_TRANSFER"), "Bank Transfer");
        assert_eq!(payment_mode_label("CASH"), "Cash");
        assert_eq!(payment_mode_label("OTHER"), "OTHER");
    }

    #[test]
    fn test_category_label() {
        assert_eq!(category_label("MEMBERSHIP"), "Membership");
        assert_eq!(category_label("SPONSORSHIP"), "Sponsorship");
    }

    #[test]
    fn test_sponsor_purpose_label() {
        assert_eq!(sponsor_purpose_label(Some("TITLE_SPONSOR")), "Title Sponsor");
        assert_eq!(sponsor_purpose_label(None), "");
        assert_eq!(sponsor_purpose_label(Some("CUSTOM")), "CUSTOM");
    }

    #[test]
    fn test_is_receipt_eligible() {
        assert!(is_receipt_eligible("MEMBERSHIP", "CASH_IN"));
        assert!(is_receipt_eligible("SPONSORSHIP", "CASH_IN"));
        assert!(!is_receipt_eligible("EXPENSE", "CASH_IN"));
        assert!(!is_receipt_eligible("MEMBERSHIP", "CASH_OUT"));
    }

    #[test]
    fn test_build_membership_summary_with_details() {
        let details = MembershipDetails {
            membership_type: Some(MembershipType::Monthly),
            includes_subscription: true,
            includes_annual_fee: true,
            includes_application_fee: false,
            is_application_fee: false,
        };
        let (purpose, items) = build_membership_summary("MEMBERSHIP", 5250.0, Some(&details)).unwrap();
        assert_eq!(items.len(), 2);
        assert!(purpose.contains("Annual Membership Fee"));
        assert!(purpose.contains("Monthly Subscription"));
    }

    #[test]
    fn test_build_membership_summary_fallback() {
        let (purpose, items) = build_membership_summary("MEMBERSHIP", 500.0, None).unwrap();
        assert_eq!(purpose, "Membership");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].amount, 500.0);
    }

    #[test]
    fn test_build_membership_summary_non_membership() {
        assert!(build_membership_summary("SPONSORSHIP", 1000.0, None).is_none());
    }

    #[test]
    fn test_parse_breakdown() {
        let json = serde_json::json!([
            {"label": "Monthly", "amount": 250},
            {"label": "Application Fee", "amount": 10000}
        ]);
        let items = parse_breakdown(Some(&json)).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "Monthly");
        assert_eq!(items[1].amount, 10000.0);
    }

    #[test]
    fn test_parse_breakdown_invalid() {
        assert!(parse_breakdown(None).is_none());
        assert!(parse_breakdown(Some(&serde_json::json!("not array"))).is_none());
        assert!(parse_breakdown(Some(&serde_json::json!([]))).is_none());
    }
}
