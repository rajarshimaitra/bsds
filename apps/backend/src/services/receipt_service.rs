//! Receipt Service — generate and store immutable receipt snapshots.
//!
//! A receipt is issued at transaction-creation time for every eligible
//! CASH_IN MEMBERSHIP or SPONSORSHIP transaction, regardless of approval
//! status.  The snapshot captures member/sponsor details as they exist at
//! payment time so the receipt remains correct even if records are later edited.

use chrono::Datelike;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::support::receipt::{
    CLUB_ADDRESS, CLUB_NAME,
    category_label, payment_mode_label, sponsor_purpose_label,
    is_receipt_eligible,
};
use crate::support::membership_rules::{MembershipType, ANNUAL_MEMBERSHIP_FEE, APPLICATION_FEE};

// ---------------------------------------------------------------------------
// Local row structs (avoid compile-time query! macro which needs DATABASE_URL)
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct TxRow {
    id: String,
    tx_type: String,
    category: String,
    amount: f64,
    payment_mode: String,
    remark: Option<String>,
    sponsor_purpose: Option<String>,
    member_id: Option<String>,
    sponsor_id: Option<String>,
    sender_name: Option<String>,
    includes_subscription: bool,
    includes_annual_fee: bool,
    includes_application_fee: bool,
    receipt_number: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct SponsorRow {
    name: String,
    company: Option<String>,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    name: String,
    user_member_id: Option<String>,
    membership_start: Option<String>,
    membership_expiry: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ReceiptRow {
    receipt_number: String,
    issued_at: String,
    status: String,
    r#type: String,
    purpose: Option<String>,
    breakdown: Option<String>,
    member_name: Option<String>,
    member_code: Option<String>,
    membership_start: Option<String>,
    membership_end: Option<String>,
    sponsor_name: Option<String>,
    sponsor_company: Option<String>,
    sponsor_purpose: Option<String>,
    amount: f64,
    payment_mode: Option<String>,
    category: Option<String>,
    remark: Option<String>,
    received_by: Option<String>,
    club_name: Option<String>,
    club_address: Option<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Issue a receipt for a CASH_IN MEMBERSHIP or SPONSORSHIP transaction.
///
/// - Idempotent: returns `Ok(None)` if a receipt already exists for this
///   transaction or if the transaction is not receipt-eligible.
/// - Updates `transactions.receipt_number` when a new receipt is created.
/// - `membership_type` is required when `includes_subscription = true` to
///   produce the correct subscription label in the breakdown.
pub async fn issue_receipt(
    pool: &SqlitePool,
    transaction_id: &str,
    issued_by_id: &str,
    issued_by_name: &str,
    membership_type: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    // ── 1. Load transaction ──────────────────────────────────────────────────
    let tx: Option<TxRow> = sqlx::query_as::<_, TxRow>(
        r#"SELECT id,
                  "type"                                   AS tx_type,
                  category,
                  CAST(amount AS REAL)                     AS amount,
                  payment_mode,
                  remark,
                  sponsor_purpose,
                  member_id,
                  sponsor_id,
                  sender_name,
                  CAST(includes_subscription AS INTEGER)   AS includes_subscription,
                  CAST(includes_annual_fee AS INTEGER)     AS includes_annual_fee,
                  CAST(includes_application_fee AS INTEGER) AS includes_application_fee,
                  receipt_number,
                  created_at
           FROM transactions WHERE id = ?1"#,
    )
    .bind(transaction_id)
    .fetch_optional(pool)
    .await?;

    let tx = match tx {
        Some(r) => r,
        None => return Ok(None),
    };

    // ── 2. Eligibility check ─────────────────────────────────────────────────
    if !is_receipt_eligible(&tx.category, &tx.tx_type) {
        return Ok(None);
    }

    // ── 3. Idempotency: bail if receipt already exists ───────────────────────
    let existing: Option<String> =
        sqlx::query_scalar("SELECT receipt_number FROM receipts WHERE transaction_id = ?1")
            .bind(transaction_id)
            .fetch_optional(pool)
            .await?;

    if let Some(rn) = existing {
        return Ok(Some(rn));
    }

    // ── 4. Generate receipt number ───────────────────────────────────────────
    let receipt_number = next_receipt_number(pool).await?;

    // ── 5. Build breakdown & type-specific fields ────────────────────────────
    let is_sponsor = tx.category == "SPONSORSHIP";

    let (
        receipt_type,
        purpose,
        breakdown_json,
        member_name,
        member_code,
        membership_start,
        membership_end,
        sponsor_name,
        sponsor_company,
        sponsor_purpose_str,
    ) = if is_sponsor {
        build_sponsor_fields(pool, &tx.sponsor_id, &tx.sponsor_purpose, &tx.sender_name, tx.amount).await?
    } else {
        build_member_fields(
            pool,
            &tx.member_id,
            &tx.sender_name,
            tx.amount,
            tx.includes_application_fee,
            tx.includes_annual_fee,
            tx.includes_subscription,
            membership_type,
        )
        .await?
    };

    // ── 6. Insert receipt record ─────────────────────────────────────────────
    let id = Uuid::new_v4().to_string();
    let mode_label = payment_mode_label(&tx.payment_mode).to_string();
    let cat_label = category_label(&tx.category).to_string();

    sqlx::query(
        "INSERT INTO receipts (
             id, transaction_id, receipt_number, issued_by_id, issued_at,
             status, \"type\",
             member_name, member_code, membership_start, membership_end,
             sponsor_name, sponsor_company, sponsor_purpose,
             amount, payment_mode, category, purpose, breakdown, remark,
             received_by, club_name, club_address
         ) VALUES (
             ?1,?2,?3,?4,?5,
             'ACTIVE',?6,
             ?7,?8,?9,?10,
             ?11,?12,?13,
             ?14,?15,?16,?17,?18,?19,
             ?20,?21,?22
         )",
    )
    .bind(&id)
    .bind(transaction_id)
    .bind(&receipt_number)
    .bind(issued_by_id)
    .bind(&tx.created_at)
    .bind(&receipt_type)
    .bind(&member_name)
    .bind(&member_code)
    .bind(&membership_start)
    .bind(&membership_end)
    .bind(&sponsor_name)
    .bind(&sponsor_company)
    .bind(&sponsor_purpose_str)
    .bind(tx.amount)
    .bind(&mode_label)
    .bind(&cat_label)
    .bind(&purpose)
    .bind(&breakdown_json)
    .bind(&tx.remark)
    .bind(issued_by_name)
    .bind(CLUB_NAME)
    .bind(CLUB_ADDRESS)
    .execute(pool)
    .await?;

    // ── 7. Update transaction.receipt_number ─────────────────────────────────
    sqlx::query("UPDATE transactions SET receipt_number = ?1 WHERE id = ?2")
        .bind(&receipt_number)
        .bind(transaction_id)
        .execute(pool)
        .await?;

    Ok(Some(receipt_number))
}

// ---------------------------------------------------------------------------
// Receipt number generation
// ---------------------------------------------------------------------------

async fn next_receipt_number(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let year = chrono::Utc::now().year();
    let prefix = format!("BSDS-REC-{year}-");
    let like_pattern = format!("{prefix}%");

    let last: Option<String> = sqlx::query_scalar(
        "SELECT receipt_number FROM receipts WHERE receipt_number LIKE ?1
         ORDER BY receipt_number DESC LIMIT 1",
    )
    .bind(&like_pattern)
    .fetch_optional(pool)
    .await?;

    let counter = last
        .as_deref()
        .and_then(|s| s.split('-').last())
        .and_then(|n| n.parse::<u32>().ok())
        .unwrap_or(0)
        + 1;

    Ok(format!("{prefix}{counter:04}"))
}

// ---------------------------------------------------------------------------
// Field builders
// ---------------------------------------------------------------------------

#[allow(clippy::type_complexity)]
async fn build_sponsor_fields(
    pool: &SqlitePool,
    sponsor_id: &Option<String>,
    sponsor_purpose: &Option<String>,
    sender_name: &Option<String>,
    amount: f64,
) -> Result<
    (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
    sqlx::Error,
> {
    let sponsor: Option<SponsorRow> = if let Some(ref sid) = sponsor_id {
        sqlx::query_as::<_, SponsorRow>("SELECT name, company FROM sponsors WHERE id = ?1")
            .bind(sid)
            .fetch_optional(pool)
            .await?
    } else {
        None
    };

    let sname = sponsor
        .as_ref()
        .map(|s| s.name.clone())
        .or_else(|| sender_name.clone())
        .unwrap_or_default();

    let scompany = sponsor.as_ref().and_then(|s| s.company.clone());

    let spur_raw = sponsor_purpose.as_deref().unwrap_or("");
    let spur_label = sponsor_purpose_label(Some(spur_raw)).to_string();
    let display = if spur_label.is_empty() {
        "Sponsorship".to_string()
    } else {
        spur_label.clone()
    };

    let breakdown = format!(r#"[{{"label":"{display}","amount":{amount}}}]"#);

    Ok((
        "SPONSOR".to_string(),
        display,
        breakdown,
        None,
        None,
        None,
        None,
        Some(sname),
        scompany,
        if spur_label.is_empty() { None } else { Some(spur_label) },
    ))
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
async fn build_member_fields(
    pool: &SqlitePool,
    member_id: &Option<String>,
    sender_name: &Option<String>,
    amount: f64,
    includes_application_fee: bool,
    includes_annual_fee: bool,
    includes_subscription: bool,
    membership_type: Option<&str>,
) -> Result<
    (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
    sqlx::Error,
> {
    // Fetch member + user data
    let member_row: Option<MemberRow> = if let Some(ref mid) = member_id {
        sqlx::query_as::<_, MemberRow>(
            r#"SELECT m.name,
                      u.member_id          AS user_member_id,
                      u.membership_start   AS membership_start,
                      u.membership_expiry  AS membership_expiry
               FROM members m
               LEFT JOIN users u ON u.id = m.user_id
               WHERE m.id = ?1"#,
        )
        .bind(mid)
        .fetch_optional(pool)
        .await?
    } else {
        None
    };

    let mname = member_row
        .as_ref()
        .map(|r| r.name.clone())
        .or_else(|| sender_name.clone());
    let mcode = member_row.as_ref().and_then(|r| r.user_member_id.clone());
    let mstart = member_row.as_ref().and_then(|r| r.membership_start.clone());
    let mend = member_row.as_ref().and_then(|r| r.membership_expiry.clone());

    // Build breakdown items
    let mut items: Vec<(String, f64)> = Vec::new();
    let mut labels: Vec<String> = Vec::new();

    if includes_application_fee {
        let fee = APPLICATION_FEE as f64;
        items.push(("Application Fee".to_string(), fee));
        labels.push("Application Fee".to_string());
    }

    if includes_annual_fee {
        let fee = ANNUAL_MEMBERSHIP_FEE as f64;
        items.push(("Annual Membership Fee".to_string(), fee));
        labels.push("Annual Membership Fee".to_string());
    }

    if includes_subscription {
        let mt = membership_type
            .and_then(|s| MembershipType::from_str_label(s))
            .unwrap_or(MembershipType::Annual);
        let label = subscription_label(mt);
        let fee = crate::support::membership_rules::membership_fee(mt) as f64;
        items.push((label.to_string(), fee));
        labels.push(label.to_string());
    }

    let (purpose, breakdown_amount) = if items.is_empty() {
        // Fallback
        (
            "Membership".to_string(),
            vec![("Membership".to_string(), amount)],
        )
    } else {
        (labels.join(", "), items)
    };

    let bd_items: String = breakdown_amount
        .iter()
        .map(|(l, a)| format!(r#"{{"label":"{l}","amount":{a}}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let breakdown_json = format!("[{bd_items}]");

    Ok((
        "MEMBER".to_string(),
        purpose,
        breakdown_json,
        mname,
        mcode,
        mstart,
        mend,
        None,
        None,
        None,
    ))
}

fn subscription_label(mt: MembershipType) -> &'static str {
    match mt {
        MembershipType::Monthly => "Monthly Subscription",
        MembershipType::HalfYearly => "Half-yearly Subscription",
        MembershipType::Annual => "Annual Subscription",
    }
}

// ---------------------------------------------------------------------------
// Build camelCase JSON response for the frontend
// ---------------------------------------------------------------------------

/// Build the camelCase `ReceiptData` JSON value expected by the frontend.
pub async fn receipt_json_by_transaction(
    pool: &SqlitePool,
    transaction_id: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row: Option<ReceiptRow> = sqlx::query_as::<_, ReceiptRow>(
        r#"SELECT receipt_number, issued_at, status, "type",
                  purpose, breakdown,
                  member_name, member_code, membership_start, membership_end,
                  sponsor_name, sponsor_company, sponsor_purpose,
                  CAST(amount AS REAL) AS amount,
                  payment_mode, category, remark, received_by,
                  club_name, club_address
           FROM receipts WHERE transaction_id = ?1"#,
    )
    .bind(transaction_id)
    .fetch_optional(pool)
    .await?;

    let r = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    // Parse breakdown JSON stored as a string
    let breakdown: Option<serde_json::Value> = r
        .breakdown
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(Some(serde_json::json!({
        "receiptNumber": r.receipt_number,
        "date": r.issued_at,
        "status": r.status,
        "type": r.r#type,
        "purpose": r.purpose,
        "breakdown": breakdown,
        "memberName": r.member_name,
        "memberId": r.member_code,
        "membershipStart": r.membership_start,
        "membershipEnd": r.membership_end,
        "sponsorName": r.sponsor_name,
        "sponsorCompany": r.sponsor_company,
        "sponsorPurpose": r.sponsor_purpose,
        "amount": r.amount,
        "paymentMode": r.payment_mode,
        "category": r.category,
        "remark": r.remark,
        "receivedBy": r.received_by,
        "clubName": r.club_name,
        "clubAddress": r.club_address,
    })))
}
