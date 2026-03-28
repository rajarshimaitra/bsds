//! Sponsor Service — business logic for sponsor management and sponsor link generation.
//!
//! Sponsors are companies or individuals who financially support the club.
//! Sponsor links are shareable public payment URLs with a cryptographic token.

use sqlx::SqlitePool;

use crate::db::models::{Sponsor, SponsorLink};
use crate::repositories::{activity_logs, sponsor_links, sponsors};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SponsorServiceError {
    #[error("Sponsor not found")]
    SponsorNotFound,
    #[error("Sponsor link not found")]
    LinkNotFound,
    #[error("Sponsor link is already inactive")]
    LinkAlreadyInactive,
    #[error("Cannot delete sponsor with existing transactions. Deactivate sponsor links instead.")]
    HasTransactions,
    #[error("No fields provided for update")]
    NoFieldsProvided,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Caller identity for logging.
#[derive(Debug, Clone)]
pub struct ActorInfo {
    pub id: String,
    pub name: String,
}

/// Input for creating a sponsor.
#[derive(Debug, Clone)]
pub struct CreateSponsorInput {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub company: Option<String>,
}

/// Input for updating a sponsor.
#[derive(Debug, Clone, Default)]
pub struct UpdateSponsorInput {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company: Option<Option<String>>,
}

/// Input for creating a sponsor link.
#[derive(Debug, Clone)]
pub struct CreateSponsorLinkInput {
    pub sponsor_id: Option<String>,
    pub amount: Option<f64>,
    pub upi_id: String,
    pub bank_details: Option<serde_json::Value>,
    pub expires_at: Option<String>,
    pub sponsor_purpose: String,
}

/// Public data for a sponsor link (safe for unauthenticated access).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PublicSponsorLinkData {
    pub token: String,
    pub sponsor_name: Option<String>,
    pub sponsor_company: Option<String>,
    pub amount: Option<f64>,
    pub purpose: String,
    pub purpose_label: String,
    pub upi_id: String,
    pub bank_details: Option<serde_json::Value>,
    pub is_active: bool,
    pub is_expired: bool,
    pub club_name: String,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CLUB_NAME: &str = "Deshapriya Park Sarbojanin Durgotsav";

fn app_url() -> String {
    std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

/// Human-readable sponsor purpose label.
pub fn sponsor_purpose_label(purpose: &str) -> &str {
    match purpose {
        "TITLE_SPONSOR" => "Title Sponsor",
        "GOLD_SPONSOR" => "Gold Sponsor",
        "SILVER_SPONSOR" => "Silver Sponsor",
        "FOOD_PARTNER" => "Food Partner",
        "MEDIA_PARTNER" => "Media Partner",
        "STALL_VENDOR" => "Stall Vendor",
        "MARKETING_PARTNER" => "Marketing Partner",
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Sponsor CRUD
// ---------------------------------------------------------------------------

/// List all sponsors.
pub async fn list_sponsors(
    pool: &SqlitePool,
) -> Result<Vec<Sponsor>, SponsorServiceError> {
    sponsors::list(pool).await.map_err(Into::into)
}

/// Get a single sponsor by ID.
pub async fn get_sponsor(
    pool: &SqlitePool,
    id: &str,
) -> Result<Sponsor, SponsorServiceError> {
    sponsors::find_by_id(pool, id)
        .await?
        .ok_or(SponsorServiceError::SponsorNotFound)
}

/// Create a new sponsor record.
pub async fn create_sponsor(
    pool: &SqlitePool,
    data: &CreateSponsorInput,
    created_by: &ActorInfo,
) -> Result<Sponsor, SponsorServiceError> {
    let sponsor = sponsors::create(
        pool,
        &sponsors::CreateSponsorData {
            name: data.name.clone(),
            phone: data.phone.clone(),
            email: data.email.clone(),
            company: data.company.clone(),
            created_by_id: created_by.id.clone(),
        },
    )
    .await?;

    let company_str = data
        .company
        .as_deref()
        .map(|c| format!(" ({c})"))
        .unwrap_or_default();

    log_activity(
        pool,
        &created_by.id,
        "sponsor_created",
        &format!("{} created sponsor: {}{}", created_by.name, sponsor.name, company_str),
        Some(serde_json::json!({
            "sponsorId": sponsor.id,
            "name": sponsor.name,
        })),
    )
    .await;

    Ok(sponsor)
}

/// Update sponsor fields.
pub async fn update_sponsor(
    pool: &SqlitePool,
    id: &str,
    data: &UpdateSponsorInput,
    updated_by: &ActorInfo,
) -> Result<Sponsor, SponsorServiceError> {
    let _existing = sponsors::find_by_id(pool, id)
        .await?
        .ok_or(SponsorServiceError::SponsorNotFound)?;

    let has_fields = data.name.is_some()
        || data.phone.is_some()
        || data.email.is_some()
        || data.company.is_some();
    if !has_fields {
        return Err(SponsorServiceError::NoFieldsProvided);
    }

    let updated = sponsors::update(
        pool,
        id,
        &sponsors::UpdateSponsorData {
            name: data.name.clone(),
            phone: data.phone.clone(),
            email: data.email.clone(),
            company: data.company.clone(),
        },
    )
    .await?;

    log_activity(
        pool,
        &updated_by.id,
        "sponsor_updated",
        &format!("{} updated sponsor: {}", updated_by.name, updated.name),
        Some(serde_json::json!({ "sponsorId": id })),
    )
    .await;

    Ok(updated)
}

/// Delete a sponsor. Only allowed if no transactions exist for this sponsor.
pub async fn delete_sponsor(
    pool: &SqlitePool,
    id: &str,
    deleted_by: &ActorInfo,
) -> Result<(), SponsorServiceError> {
    let sponsor = sponsors::find_by_id(pool, id)
        .await?
        .ok_or(SponsorServiceError::SponsorNotFound)?;

    // Check for existing transactions
    let tx_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE sponsor_id = ?1")
            .bind(id)
            .fetch_one(pool)
            .await?;

    if tx_count > 0 {
        return Err(SponsorServiceError::HasTransactions);
    }

    // Deactivate all sponsor links first
    sqlx::query("UPDATE sponsor_links SET is_active = 0 WHERE sponsor_id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    sponsors::delete(pool, id).await?;

    log_activity(
        pool,
        &deleted_by.id,
        "sponsor_deleted",
        &format!("{} deleted sponsor: {}", deleted_by.name, sponsor.name),
        Some(serde_json::json!({
            "sponsorId": id,
            "name": sponsor.name,
        })),
    )
    .await;

    Ok(())
}

// ---------------------------------------------------------------------------
// Sponsor link operations
// ---------------------------------------------------------------------------

/// Generate a new sponsor payment link with a cryptographic token.
pub async fn generate_sponsor_link(
    pool: &SqlitePool,
    data: &CreateSponsorLinkInput,
    created_by: &ActorInfo,
) -> Result<(SponsorLink, String), SponsorServiceError> {
    // Validate sponsor ID if provided
    if let Some(ref sid) = data.sponsor_id {
        let _ = sponsors::find_by_id(pool, sid)
            .await?
            .ok_or(SponsorServiceError::SponsorNotFound)?;
    }

    let token = uuid::Uuid::new_v4().to_string();

    // Store sponsorPurpose in bank_details JSON
    let bank_details_payload = {
        let mut obj = data
            .bank_details
            .as_ref()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        obj.insert(
            "sponsorPurpose".to_string(),
            serde_json::Value::String(data.sponsor_purpose.clone()),
        );
        serde_json::to_string(&serde_json::Value::Object(obj)).ok()
    };

    let link = sponsor_links::create(
        pool,
        &sponsor_links::CreateSponsorLinkData {
            sponsor_id: data.sponsor_id.clone(),
            token: token.clone(),
            amount: data.amount,
            upi_id: data.upi_id.clone(),
            bank_details: bank_details_payload,
            is_active: true,
            created_by_id: created_by.id.clone(),
            expires_at: data.expires_at.clone(),
        },
    )
    .await?;

    let url = format!("{}/sponsor/{}", app_url(), token);

    let amount_str = data
        .amount
        .map(|a| format!(" — Rs.{a}"))
        .unwrap_or_else(|| " (open-ended)".to_string());

    log_activity(
        pool,
        &created_by.id,
        "sponsor_link_created",
        &format!(
            "{} generated sponsor payment link for {}{}",
            created_by.name,
            sponsor_purpose_label(&data.sponsor_purpose),
            amount_str
        ),
        Some(serde_json::json!({
            "linkId": link.id,
            "token": token,
            "sponsorId": data.sponsor_id,
            "purpose": data.sponsor_purpose,
            "amount": data.amount,
            "url": url,
        })),
    )
    .await;

    Ok((link, url))
}

/// Deactivate a sponsor link.
pub async fn deactivate_sponsor_link(
    pool: &SqlitePool,
    id: &str,
    updated_by: &ActorInfo,
) -> Result<(), SponsorServiceError> {
    let link = sponsor_links::find_by_id(pool, id)
        .await?
        .ok_or(SponsorServiceError::LinkNotFound)?;

    if !link.is_active {
        return Err(SponsorServiceError::LinkAlreadyInactive);
    }

    sponsor_links::update_payment_status(pool, id, false, None).await?;

    log_activity(
        pool,
        &updated_by.id,
        "sponsor_link_deactivated",
        &format!("{} deactivated sponsor link {}", updated_by.name, link.token),
        Some(serde_json::json!({
            "linkId": id,
            "token": link.token,
        })),
    )
    .await;

    Ok(())
}

/// List sponsor links by sponsor ID.
pub async fn list_sponsor_links(
    pool: &SqlitePool,
    sponsor_id: &str,
) -> Result<Vec<SponsorLink>, SponsorServiceError> {
    sponsor_links::list_by_sponsor_id(pool, sponsor_id)
        .await
        .map_err(Into::into)
}

/// Get public sponsor link data by token (no auth required).
pub async fn get_public_sponsor_link(
    pool: &SqlitePool,
    token: &str,
) -> Result<PublicSponsorLinkData, SponsorServiceError> {
    let link = sponsor_links::find_by_token(pool, token)
        .await?
        .ok_or(SponsorServiceError::LinkNotFound)?;

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let is_expired = link
        .expires_at
        .as_deref()
        .map(|e| e < now.as_str())
        .unwrap_or(false);

    let bd: Option<serde_json::Value> = link
        .bank_details
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let purpose = bd
        .as_ref()
        .and_then(|v| v["sponsorPurpose"].as_str())
        .unwrap_or("OTHER")
        .to_string();

    // Strip sponsorPurpose from bankDetails before sending to client
    let public_bank_details = bd.and_then(|mut v| {
        if let Some(obj) = v.as_object_mut() {
            obj.remove("sponsorPurpose");
            if obj.is_empty() {
                return None;
            }
        }
        Some(v)
    });

    // Lookup sponsor name/company
    let (sponsor_name, sponsor_company) = if let Some(ref sid) = link.sponsor_id {
        let sponsor = sponsors::find_by_id(pool, sid).await?.map(|s| (s.name, s.company));
        match sponsor {
            Some((name, company)) => (Some(name), company),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    Ok(PublicSponsorLinkData {
        token: link.token,
        sponsor_name,
        sponsor_company,
        amount: link.amount,
        purpose: purpose.clone(),
        purpose_label: sponsor_purpose_label(&purpose).to_string(),
        upi_id: link.upi_id,
        bank_details: public_bank_details,
        is_active: link.is_active,
        is_expired,
        club_name: CLUB_NAME.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn log_activity(
    pool: &SqlitePool,
    user_id: &str,
    action: &str,
    description: &str,
    metadata: Option<serde_json::Value>,
) {
    let _ = activity_logs::create(
        pool,
        &activity_logs::CreateActivityLogData {
            user_id: user_id.to_string(),
            action: action.to_string(),
            description: description.to_string(),
            metadata: metadata.map(|m| serde_json::to_string(&m).unwrap_or_default()),
        },
    )
    .await;
}
