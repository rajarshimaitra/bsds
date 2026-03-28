use std::collections::HashMap;

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use sqlx::SqlitePool;

use crate::{
    auth::AuthSession,
    integrations::razorpay::{CreateOrderOptions, RazorpayClient},
    repositories::{members, sponsor_links, users},
    services::sponsor_service,
    support::{
        member_id::{build_member_order_receipt_reference, build_sponsor_order_receipt_reference},
        membership_rules::{self, MembershipType},
    },
};

use super::AppError;

#[derive(Debug, Deserialize)]
struct CreateMemberOrderRequest {
    #[serde(rename = "memberId")]
    member_id: String,
    #[serde(rename = "membershipType")]
    membership_type: String,
    #[serde(rename = "feeType")]
    fee_type: Option<String>,
    #[serde(rename = "isApplicationFee")]
    is_application_fee: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct VerifyPaymentRequest {
    razorpay_order_id: String,
    razorpay_payment_id: String,
    razorpay_signature: String,
}

#[derive(Debug, Deserialize)]
struct CreateSponsorOrderRequest {
    token: String,
    amount: Option<f64>,
    #[serde(rename = "sponsorPurpose")]
    sponsor_purpose: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateOrderResponse {
    #[serde(rename = "orderId")]
    order_id: String,
    amount: u64,
    currency: String,
    #[serde(rename = "keyId")]
    key_id: String,
    receipt: Option<String>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/create-order", post(create_order))
        .route("/verify", post(verify_payment))
        .route("/sponsor-order", post(create_sponsor_order))
        .route("/sponsor-verify", post(verify_sponsor_payment))
}

async fn create_order(
    State(pool): State<SqlitePool>,
    AuthSession(claims): AuthSession,
    Json(body): Json<CreateMemberOrderRequest>,
) -> Result<Json<CreateOrderResponse>, AppError> {
    require_authenticated(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    if claims.role == "MEMBER" {
        let claim_member_id = claims
            .member_id
            .as_deref()
            .ok_or(AppError::Forbidden)?;
        let own_member = members::find_by_member_id(&pool, claim_member_id)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;

        if own_member.as_ref().map(|member| member.id.as_str()) != Some(body.member_id.as_str()) {
            return Err(AppError::Forbidden);
        }
    }

    let member = members::find_by_id(&pool, &body.member_id)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .ok_or(AppError::NotFound)?;

    let linked_user = match member.user_id.as_deref() {
        Some(user_id) => users::find_by_id(&pool, user_id)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?,
        None => None,
    };

    let fee_type = body.fee_type.as_deref().unwrap_or("SUBSCRIPTION");
    let is_annual_fee = fee_type == "ANNUAL_FEE";
    let is_application_fee = if is_annual_fee {
        false
    } else {
        body.is_application_fee.unwrap_or(false)
    };

    if is_application_fee && linked_user.as_ref().map(|user| user.application_fee_paid).unwrap_or(false)
    {
        return Err(AppError::BadRequest(
            "Application fee has already been paid for this member".to_string(),
        ));
    }

    let membership_amount = if is_annual_fee {
        membership_rules::ANNUAL_MEMBERSHIP_FEE
    } else {
        let membership_type = MembershipType::from_str_label(&body.membership_type)
            .ok_or_else(|| AppError::BadRequest("Invalid membership type".to_string()))?;
        membership_rules::membership_fee(membership_type)
    };
    let total_amount = membership_amount + if is_application_fee {
        membership_rules::APPLICATION_FEE
    } else {
        0
    };

    let client = RazorpayClient::from_env().map_err(|error| AppError::Internal(error.to_string()))?;
    let mut notes = HashMap::new();
    notes.insert("memberId".to_string(), body.member_id.clone());
    notes.insert("membershipType".to_string(), body.membership_type.clone());
    notes.insert("feeType".to_string(), fee_type.to_string());
    notes.insert("isApplicationFee".to_string(), is_application_fee.to_string());
    notes.insert(
        "includesSubscription".to_string(),
        (!is_annual_fee).to_string(),
    );
    notes.insert("includesAnnualFee".to_string(), is_annual_fee.to_string());
    if let Some(user) = linked_user.as_ref() {
        notes.insert("userMemberId".to_string(), user.member_id.clone());
    }
    notes.insert("memberName".to_string(), member.name.clone());

    let order = client
        .create_order(&CreateOrderOptions {
            amount: crate::integrations::razorpay::rupees_to_paise(total_amount as f64),
            currency: "INR".to_string(),
            receipt: build_member_order_receipt_reference(&body.member_id, now_timestamp()),
            notes: Some(notes),
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(CreateOrderResponse {
        order_id: order.id,
        amount: order.amount,
        currency: order.currency,
        key_id: client.key_id().to_string(),
        receipt: order.receipt,
    }))
}

async fn verify_payment(
    AuthSession(claims): AuthSession,
    Json(body): Json<VerifyPaymentRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_authenticated(&claims.role)?;
    require_password_changed(claims.must_change_password)?;

    let client = RazorpayClient::from_env().map_err(|error| AppError::Internal(error.to_string()))?;
    let valid = client.verify_payment_signature(
        &body.razorpay_order_id,
        &body.razorpay_payment_id,
        &body.razorpay_signature,
    );

    if !valid {
        return Err(AppError::BadRequest(
            "Payment signature verification failed".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({ "verified": true })))
}

async fn create_sponsor_order(
    State(pool): State<SqlitePool>,
    Json(body): Json<CreateSponsorOrderRequest>,
) -> Result<Json<CreateOrderResponse>, AppError> {
    let public_link = sponsor_service::get_public_sponsor_link(&pool, &body.token)
        .await
        .map_err(map_sponsor_error)?;

    if !public_link.is_active || public_link.is_expired {
        return Err(AppError::BadRequest(
            if public_link.is_expired {
                "This payment link has expired"
            } else {
                "This payment link is no longer active"
            }
            .to_string(),
        ));
    }

    let link = sponsor_links::find_by_token(&pool, &body.token)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
        .ok_or(AppError::NotFound)?;

    let amount = match public_link.amount {
        Some(amount) => amount,
        None => body
            .amount
            .filter(|value| *value >= 1.0)
            .ok_or_else(|| AppError::BadRequest(
                "Amount is required for open-ended sponsor links (minimum Rs.1)".to_string(),
            ))?,
    };

    let purpose = link
        .bank_details
        .as_deref()
        .and_then(|details| serde_json::from_str::<serde_json::Value>(details).ok())
        .and_then(|value| value.get("sponsorPurpose").and_then(|item| item.as_str()).map(str::to_string))
        .or(body.sponsor_purpose)
        .unwrap_or_else(|| public_link.purpose.clone());

    let client = RazorpayClient::from_env().map_err(|error| AppError::Internal(error.to_string()))?;
    let mut notes = HashMap::new();
    notes.insert("sponsorLinkToken".to_string(), body.token.clone());
    notes.insert("sponsorPurpose".to_string(), purpose);
    if let Some(sponsor_id) = link.sponsor_id.clone() {
        notes.insert("sponsorId".to_string(), sponsor_id);
    }

    let order = client
        .create_order(&CreateOrderOptions {
            amount: crate::integrations::razorpay::rupees_to_paise(amount),
            currency: "INR".to_string(),
            receipt: build_sponsor_order_receipt_reference(&body.token, now_timestamp()),
            notes: Some(notes),
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(CreateOrderResponse {
        order_id: order.id,
        amount: order.amount,
        currency: order.currency,
        key_id: client.key_id().to_string(),
        receipt: order.receipt,
    }))
}

async fn verify_sponsor_payment(
    Json(body): Json<VerifyPaymentRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client = RazorpayClient::from_env().map_err(|error| AppError::Internal(error.to_string()))?;
    let valid = client.verify_payment_signature(
        &body.razorpay_order_id,
        &body.razorpay_payment_id,
        &body.razorpay_signature,
    );

    if !valid {
        return Err(AppError::BadRequest(
            "Payment signature verification failed".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({ "verified": true })))
}

fn require_authenticated(role: &str) -> Result<(), AppError> {
    if role.is_empty() {
        Err(AppError::Unauthorized)
    } else {
        Ok(())
    }
}

fn require_password_changed(must_change_password: bool) -> Result<(), AppError> {
    if must_change_password {
        Err(AppError::Forbidden)
    } else {
        Ok(())
    }
}

fn map_sponsor_error(error: sponsor_service::SponsorServiceError) -> AppError {
    match error {
        sponsor_service::SponsorServiceError::SponsorNotFound
        | sponsor_service::SponsorServiceError::LinkNotFound => AppError::NotFound,
        sponsor_service::SponsorServiceError::LinkAlreadyInactive
        | sponsor_service::SponsorServiceError::HasTransactions
        | sponsor_service::SponsorServiceError::NoFieldsProvided => {
            AppError::BadRequest(error.to_string())
        }
        sponsor_service::SponsorServiceError::Database(_) => AppError::Internal(error.to_string()),
    }
}

fn now_timestamp() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

use serde::{Deserialize, Serialize};
