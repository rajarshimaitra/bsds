//! Audit and activity log entry builders.
//!
//! These helpers build the data structures for audit/activity log entries.
//! Actual DB insertion is handled by the repository layer.

use serde_json::Value as JsonValue;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Parameters for creating a financial audit log entry.
#[derive(Debug, Clone)]
pub struct AuditLogParams {
    pub transaction_id: String,
    pub event_type: AuditEventType,
    pub transaction_snapshot: JsonValue,
    pub performed_by_id: String,
}

/// Allowed audit event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AuditEventType {
    #[serde(rename = "TRANSACTION_CREATED")]
    TransactionCreated,
    #[serde(rename = "TRANSACTION_APPROVED")]
    TransactionApproved,
    #[serde(rename = "TRANSACTION_REJECTED")]
    TransactionRejected,
}

impl AuditEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TransactionCreated => "TRANSACTION_CREATED",
            Self::TransactionApproved => "TRANSACTION_APPROVED",
            Self::TransactionRejected => "TRANSACTION_REJECTED",
        }
    }

    pub fn from_str_label(s: &str) -> Option<Self> {
        match s {
            "TRANSACTION_CREATED" => Some(Self::TransactionCreated),
            "TRANSACTION_APPROVED" => Some(Self::TransactionApproved),
            "TRANSACTION_REJECTED" => Some(Self::TransactionRejected),
            _ => None,
        }
    }
}

/// Parameters for creating a system-wide activity log entry.
#[derive(Debug, Clone)]
pub struct ActivityLogParams {
    pub user_id: String,
    pub action: String,
    pub description: String,
    pub metadata: Option<JsonValue>,
}

/// Source data for building a transaction audit snapshot.
#[derive(Debug, Clone)]
pub struct TransactionSnapshotSource {
    pub id: String,
    pub r#type: String,
    pub category: String,
    pub amount: String,
    pub payment_mode: String,
    pub purpose: String,
    pub remark: Option<String>,
    pub sponsor_purpose: Option<String>,
    pub approval_status: String,
    pub approval_source: String,
    pub entered_by_id: Option<String>,
    pub approved_by_id: Option<String>,
    pub approved_at: Option<String>,
    pub razorpay_payment_id: Option<String>,
    pub razorpay_order_id: Option<String>,
    pub sender_name: Option<String>,
    pub sender_phone: Option<String>,
    pub sender_upi_id: Option<String>,
    pub sender_bank_account: Option<String>,
    pub sender_bank_name: Option<String>,
    pub sponsor_sender_name: Option<String>,
    pub sponsor_sender_contact: Option<String>,
    pub receipt_number: Option<String>,
    pub member_id: Option<String>,
    pub sponsor_id: Option<String>,
    pub created_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Snapshot builders
// ---------------------------------------------------------------------------

/// Build a JSON snapshot of a transaction for the audit log.
///
/// All fields are included even if null, matching the TypeScript version.
pub fn build_transaction_audit_snapshot(source: &TransactionSnapshotSource) -> JsonValue {
    let mut map = serde_json::Map::new();

    let insert_str = |m: &mut serde_json::Map<String, JsonValue>, key: &str, val: &str| {
        m.insert(key.to_string(), JsonValue::String(val.to_string()));
    };
    let insert_opt =
        |m: &mut serde_json::Map<String, JsonValue>, key: &str, val: &Option<String>| {
            m.insert(
                key.to_string(),
                val.as_ref()
                    .map(|v| JsonValue::String(v.clone()))
                    .unwrap_or(JsonValue::Null),
            );
        };

    insert_str(&mut map, "id", &source.id);
    insert_str(&mut map, "type", &source.r#type);
    insert_str(&mut map, "category", &source.category);
    insert_str(&mut map, "amount", &source.amount);
    insert_str(&mut map, "paymentMode", &source.payment_mode);
    insert_str(&mut map, "purpose", &source.purpose);
    insert_opt(&mut map, "remark", &source.remark);
    insert_opt(&mut map, "sponsorPurpose", &source.sponsor_purpose);
    insert_str(&mut map, "approvalStatus", &source.approval_status);
    insert_str(&mut map, "approvalSource", &source.approval_source);
    insert_opt(&mut map, "enteredById", &source.entered_by_id);
    insert_opt(&mut map, "approvedById", &source.approved_by_id);
    insert_opt(&mut map, "approvedAt", &source.approved_at);
    insert_opt(&mut map, "razorpayPaymentId", &source.razorpay_payment_id);
    insert_opt(&mut map, "razorpayOrderId", &source.razorpay_order_id);
    insert_opt(&mut map, "senderName", &source.sender_name);
    insert_opt(&mut map, "senderPhone", &source.sender_phone);
    insert_opt(&mut map, "senderUpiId", &source.sender_upi_id);
    insert_opt(&mut map, "senderBankAccount", &source.sender_bank_account);
    insert_opt(&mut map, "senderBankName", &source.sender_bank_name);
    insert_opt(&mut map, "sponsorSenderName", &source.sponsor_sender_name);
    insert_opt(&mut map, "sponsorSenderContact", &source.sponsor_sender_contact);
    insert_opt(&mut map, "receiptNumber", &source.receipt_number);
    insert_opt(&mut map, "memberId", &source.member_id);
    insert_opt(&mut map, "sponsorId", &source.sponsor_id);
    insert_opt(&mut map, "createdAt", &source.created_at);

    JsonValue::Object(map)
}

/// Resolve a transaction snapshot from an audit log entry.
///
/// If the entry already contains a valid JSON object snapshot, returns it.
/// If it contains a transaction source, builds the snapshot from it.
/// Otherwise returns an empty object.
pub fn resolve_audit_snapshot(
    transaction_snapshot: Option<&JsonValue>,
    transaction: Option<&TransactionSnapshotSource>,
) -> JsonValue {
    if let Some(snapshot) = transaction_snapshot {
        if snapshot.is_object() {
            return snapshot.clone();
        }
    }

    if let Some(tx) = transaction {
        return build_transaction_audit_snapshot(tx);
    }

    JsonValue::Object(serde_json::Map::new())
}

/// Build metadata map for activity log entries (convenience helper).
pub fn build_activity_metadata(pairs: &[(&str, &str)]) -> JsonValue {
    let map: HashMap<&str, &str> = pairs.iter().copied().collect();
    serde_json::to_value(map).unwrap_or(JsonValue::Object(serde_json::Map::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_snapshot_includes_all_fields() {
        let source = TransactionSnapshotSource {
            id: "tx-1".into(),
            r#type: "CASH_IN".into(),
            category: "MEMBERSHIP".into(),
            amount: "1500".into(),
            payment_mode: "UPI".into(),
            purpose: "Monthly subscription".into(),
            remark: None,
            sponsor_purpose: None,
            approval_status: "APPROVED".into(),
            approval_source: "ADMIN".into(),
            entered_by_id: Some("user-1".into()),
            approved_by_id: Some("user-2".into()),
            approved_at: Some("2026-03-01T00:00:00Z".into()),
            razorpay_payment_id: None,
            razorpay_order_id: None,
            sender_name: None,
            sender_phone: None,
            sender_upi_id: None,
            sender_bank_account: None,
            sender_bank_name: None,
            sponsor_sender_name: None,
            sponsor_sender_contact: None,
            receipt_number: None,
            member_id: Some("member-1".into()),
            sponsor_id: None,
            created_at: Some("2026-03-01T00:00:00Z".into()),
        };

        let snapshot = build_transaction_audit_snapshot(&source);
        let obj = snapshot.as_object().unwrap();
        assert_eq!(obj["id"], "tx-1");
        assert_eq!(obj["amount"], "1500");
        assert!(obj["remark"].is_null());
        assert_eq!(obj["memberId"], "member-1");
    }

    #[test]
    fn test_resolve_existing_snapshot() {
        let existing = serde_json::json!({"id": "tx-1"});
        let result = resolve_audit_snapshot(Some(&existing), None);
        assert_eq!(result["id"], "tx-1");
    }

    #[test]
    fn test_resolve_empty_returns_empty_object() {
        let result = resolve_audit_snapshot(None, None);
        assert!(result.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_build_activity_metadata() {
        let meta = build_activity_metadata(&[("key", "value"), ("foo", "bar")]);
        assert_eq!(meta["key"], "value");
        assert_eq!(meta["foo"], "bar");
    }
}
