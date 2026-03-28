use serde_json::Value as JsonValue;

pub const MEMBERSHIP_APPROVAL: &str = "MEMBERSHIP_APPROVAL";
pub const MEMBERSHIP_PAYMENT_APPROVAL: &str = "MEMBERSHIP_PAYMENT_APPROVAL";
pub const TRANSACTION_APPROVAL: &str = "TRANSACTION_APPROVAL";

pub const INCOMING: &str = "INCOMING";
pub const OUTGOING: &str = "OUTGOING";

pub fn is_known_approval_type(value: &str) -> bool {
    matches!(
        value,
        MEMBERSHIP_APPROVAL | MEMBERSHIP_PAYMENT_APPROVAL | TRANSACTION_APPROVAL
    )
}

pub fn approval_type_label(value: &str) -> &'static str {
    match value {
        MEMBERSHIP_PAYMENT_APPROVAL => "Membership",
        TRANSACTION_APPROVAL => "Transaction",
        _ => "Membership",
    }
}

pub fn approval_type_from_entity(
    entity_type: &str,
    transaction_category: Option<&str>,
) -> &'static str {
    if entity_type == "TRANSACTION" {
        if transaction_category == Some("MEMBERSHIP") {
            MEMBERSHIP_PAYMENT_APPROVAL
        } else {
            TRANSACTION_APPROVAL
        }
    } else {
        MEMBERSHIP_APPROVAL
    }
}

pub fn direction_from_transaction_type(transaction_type: Option<&str>) -> Option<&'static str> {
    match transaction_type {
        Some("CASH_IN") => Some(INCOMING),
        Some("CASH_OUT") => Some(OUTGOING),
        _ => None,
    }
}

pub fn read_metadata_approval_type(metadata: Option<&JsonValue>) -> Option<&str> {
    let value = metadata?.get("approvalType")?.as_str()?;
    if is_known_approval_type(value) {
        Some(value)
    } else {
        None
    }
}

pub fn read_metadata_direction(metadata: Option<&JsonValue>) -> Option<&str> {
    match metadata?.get("direction")?.as_str() {
        Some(INCOMING) => Some(INCOMING),
        Some(OUTGOING) => Some(OUTGOING),
        _ => None,
    }
}
