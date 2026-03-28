//! Membership business rules: fee constants, duration calculations, expiry
//! logic, and amount validation.
//!
//! These are pure functions with no DB access.

use chrono::NaiveDate;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Membership subscription types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MembershipType {
    #[serde(rename = "MONTHLY")]
    Monthly,
    #[serde(rename = "HALF_YEARLY")]
    HalfYearly,
    #[serde(rename = "ANNUAL")]
    Annual,
}

impl MembershipType {
    /// Parse from the string representation used in the database.
    pub fn from_str_label(s: &str) -> Option<Self> {
        match s {
            "MONTHLY" => Some(Self::Monthly),
            "HALF_YEARLY" => Some(Self::HalfYearly),
            "ANNUAL" => Some(Self::Annual),
            _ => None,
        }
    }

    /// Return the database string label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Monthly => "MONTHLY",
            Self::HalfYearly => "HALF_YEARLY",
            Self::Annual => "ANNUAL",
        }
    }
}

// ---------------------------------------------------------------------------
// Fee constants (INR)
// ---------------------------------------------------------------------------

/// Subscription fee for each membership type.
pub fn membership_fee(membership_type: MembershipType) -> u64 {
    match membership_type {
        MembershipType::Monthly => 250,
        MembershipType::HalfYearly => 1_500,
        MembershipType::Annual => 3_000,
    }
}

/// Annual membership fee (one-time per year, all members).
pub const ANNUAL_MEMBERSHIP_FEE: u64 = 5_000;

/// Application fee (one-time, first membership only).
pub const APPLICATION_FEE: u64 = 10_000;

// ---------------------------------------------------------------------------
// Duration constants (days)
// ---------------------------------------------------------------------------

/// Number of days each membership type lasts.
pub fn membership_duration_days(membership_type: MembershipType) -> i64 {
    match membership_type {
        MembershipType::Monthly => 30,
        MembershipType::HalfYearly => 180,
        MembershipType::Annual => 365,
    }
}

/// Duration of the annual fee period in days.
pub const ANNUAL_FEE_DURATION_DAYS: i64 = 365;

/// Number of days before expiry when reminder notifications should be sent.
pub const EXPIRY_REMINDER_DAYS: i64 = 15;

// ---------------------------------------------------------------------------
// Date calculations
// ---------------------------------------------------------------------------

/// Result of a membership date calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Calculate membership start and end dates.
///
/// If the member has an active (non-expired) membership, the new period
/// starts the day after current expiry. Otherwise it starts today.
pub fn calculate_membership_dates(
    membership_type: MembershipType,
    current_expiry: Option<NaiveDate>,
    today: NaiveDate,
) -> DateRange {
    let start_date = match current_expiry {
        Some(expiry) if expiry >= today => expiry + chrono::Duration::days(1),
        _ => today,
    };

    let duration = membership_duration_days(membership_type);
    let end_date = start_date + chrono::Duration::days(duration - 1);

    DateRange {
        start_date,
        end_date,
    }
}

/// Calculate annual fee start and end dates.
///
/// If the member's annual fee is still active, the new period starts the day
/// after current expiry. Otherwise it starts today.
pub fn calculate_annual_fee_dates(
    current_annual_fee_expiry: Option<NaiveDate>,
    today: NaiveDate,
) -> DateRange {
    let start_date = match current_annual_fee_expiry {
        Some(expiry) if expiry >= today => expiry + chrono::Duration::days(1),
        _ => today,
    };

    let end_date = start_date + chrono::Duration::days(ANNUAL_FEE_DURATION_DAYS - 1);

    DateRange {
        start_date,
        end_date,
    }
}

// ---------------------------------------------------------------------------
// Amount validation
// ---------------------------------------------------------------------------

/// Validate that the amount matches the expected fee for the membership type.
/// Returns `None` if valid, or `Some(error_message)` if invalid.
pub fn validate_membership_amount(
    membership_type: MembershipType,
    amount: u64,
    is_application_fee: bool,
) -> Option<String> {
    let expected_fee = membership_fee(membership_type);
    let expected_total = if is_application_fee {
        expected_fee + APPLICATION_FEE
    } else {
        expected_fee
    };

    if amount != expected_total {
        if is_application_fee {
            return Some(format!(
                "Amount must be exactly \u{20b9}{expected_total} (\u{20b9}{APPLICATION_FEE} application fee + \u{20b9}{expected_fee} membership fee) for {} membership",
                membership_type.as_str()
            ));
        }
        return Some(format!(
            "Amount must be exactly \u{20b9}{expected_fee} for {} membership. No partial payments allowed.",
            membership_type.as_str()
        ));
    }

    None
}

/// Validate that the amount matches the annual membership fee.
/// Returns `None` if valid, or `Some(error_message)` if invalid.
pub fn validate_annual_fee_amount(amount: u64) -> Option<String> {
    if amount != ANNUAL_MEMBERSHIP_FEE {
        return Some(format!(
            "Annual membership fee must be exactly \u{20b9}{ANNUAL_MEMBERSHIP_FEE}"
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_membership_fees() {
        assert_eq!(membership_fee(MembershipType::Monthly), 250);
        assert_eq!(membership_fee(MembershipType::HalfYearly), 1500);
        assert_eq!(membership_fee(MembershipType::Annual), 3000);
    }

    #[test]
    fn test_calculate_membership_dates_no_expiry() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let range = calculate_membership_dates(MembershipType::Monthly, None, today);
        assert_eq!(range.start_date, today);
        assert_eq!(range.end_date, NaiveDate::from_ymd_opt(2026, 3, 30).unwrap());
    }

    #[test]
    fn test_calculate_membership_dates_active_expiry() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let range = calculate_membership_dates(MembershipType::Monthly, Some(expiry), today);
        assert_eq!(range.start_date, NaiveDate::from_ymd_opt(2026, 3, 16).unwrap());
    }

    #[test]
    fn test_calculate_membership_dates_expired() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        let range = calculate_membership_dates(MembershipType::Monthly, Some(expiry), today);
        assert_eq!(range.start_date, today);
    }

    #[test]
    fn test_validate_membership_amount_ok() {
        assert!(validate_membership_amount(MembershipType::Monthly, 250, false).is_none());
    }

    #[test]
    fn test_validate_membership_amount_with_application() {
        assert!(validate_membership_amount(MembershipType::Monthly, 10250, true).is_none());
        assert!(validate_membership_amount(MembershipType::Monthly, 250, true).is_some());
    }

    #[test]
    fn test_validate_annual_fee() {
        assert!(validate_annual_fee_amount(5000).is_none());
        assert!(validate_annual_fee_amount(4999).is_some());
    }

    #[test]
    fn test_membership_type_roundtrip() {
        for t in [MembershipType::Monthly, MembershipType::HalfYearly, MembershipType::Annual] {
            assert_eq!(MembershipType::from_str_label(t.as_str()), Some(t));
        }
    }
}
