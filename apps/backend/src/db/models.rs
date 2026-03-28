#![allow(dead_code)]

// ============================================================
// Database model structs — one per table
// All fields use snake_case matching the SQL column names.
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    pub member_id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub password: String,
    pub is_temp_password: bool,
    pub role: String,
    pub membership_status: String,
    pub membership_type: Option<String>,
    pub membership_start: Option<String>,
    pub membership_expiry: Option<String>,
    pub total_paid: f64,
    pub application_fee_paid: bool,
    pub annual_fee_start: Option<String>,
    pub annual_fee_expiry: Option<String>,
    pub annual_fee_paid: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct SubMember {
    pub id: String,
    pub member_id: String,
    pub parent_user_id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub password: String,
    pub is_temp_password: bool,
    pub relation: String,
    pub can_login: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Member {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub parent_member_id: Option<String>,
    pub joined_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Membership {
    pub id: String,
    pub member_id: String,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub r#type: String,
    pub fee_type: String,
    pub amount: f64,
    pub start_date: String,
    pub end_date: String,
    pub is_application_fee: bool,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Sponsor {
    pub id: String,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub company: Option<String>,
    pub created_by_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Transaction {
    pub id: String,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub r#type: String,
    pub category: String,
    pub amount: f64,
    pub payment_mode: String,
    pub purpose: String,
    pub remark: Option<String>,
    pub sponsor_purpose: Option<String>,
    pub member_id: Option<String>,
    pub sponsor_id: Option<String>,
    pub entered_by_id: String,
    pub approval_status: String,
    pub approval_source: String,
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
    pub includes_subscription: bool,
    pub includes_annual_fee: bool,
    pub includes_application_fee: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Receipt {
    pub id: String,
    pub transaction_id: String,
    pub receipt_number: String,
    pub issued_by_id: String,
    pub issued_at: String,
    pub status: String,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub r#type: String,
    pub member_name: Option<String>,
    pub member_code: Option<String>,
    pub membership_start: Option<String>,
    pub membership_end: Option<String>,
    pub sponsor_name: Option<String>,
    pub sponsor_company: Option<String>,
    pub sponsor_purpose: Option<String>,
    pub amount: f64,
    pub payment_mode: String,
    pub category: String,
    pub purpose: String,
    pub breakdown: Option<String>,
    pub remark: Option<String>,
    pub received_by: String,
    pub club_name: String,
    pub club_address: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct SponsorLink {
    pub id: String,
    pub sponsor_id: Option<String>,
    pub token: String,
    pub amount: Option<f64>,
    pub upi_id: String,
    pub bank_details: Option<String>,
    pub is_active: bool,
    pub created_by_id: String,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Approval {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub action: String,
    pub previous_data: Option<String>,
    pub new_data: Option<String>,
    pub requested_by_id: String,
    pub status: String,
    pub reviewed_by_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub transaction_id: String,
    pub event_type: String,
    pub transaction_snapshot: String,
    pub performed_by_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ActivityLog {
    pub id: String,
    pub user_id: String,
    pub action: String,
    pub description: String,
    pub metadata: Option<String>,
    pub created_at: String,
}
