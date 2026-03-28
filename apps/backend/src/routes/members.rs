use axum::{
    Router,
    routing::get,
    extract::{State, Path, Query},
    Json,
};
use serde::Deserialize;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

use crate::auth::AuthSession;
use crate::auth::permissions::Role;
use crate::routes::AppError;
use crate::services::member_service::{self, RequestedBy};
use crate::repositories::members::MemberListFilters;
use crate::db::models::Member;

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub search: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list_members).post(create_member))
        .route("/:id", get(get_member).patch(update_member).delete(delete_member))
        .route("/:id/sub-members", get(list_sub_members).post(add_sub_member).put(update_sub_member).delete(remove_sub_member))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_role(claims: &crate::auth::SessionClaims) -> Role {
    Role::from_str(&claims.role).unwrap_or(Role::Member)
}

// ---------------------------------------------------------------------------
// List members — ORGANISER+
// ---------------------------------------------------------------------------

async fn list_members(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Organiser) {
        return Err(AppError::Forbidden);
    }
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).max(1);
    let filters = MemberListFilters {
        status: q.status,
        search: q.search,
        page,
        limit,
    };
    let (members_list, total) = member_service::list_members(&pool, &filters)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Batch-fetch user data for all members that have a linked user
    let user_ids: Vec<String> = members_list.iter()
        .filter_map(|m| m.user_id.clone())
        .collect();
    let user_map = fetch_user_data_map(&pool, &user_ids).await;

    let data: Vec<serde_json::Value> = members_list.iter()
        .map(|m| member_to_json(m, m.user_id.as_deref().and_then(|uid| user_map.get(uid))))
        .collect();

    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    Ok(Json(serde_json::json!({
        "data": data,
        "total": total,
        "page": page,
        "limit": limit,
        "totalPages": total_pages,
    })))
}

// ---------------------------------------------------------------------------
// Get single member — ORGANISER+
// ---------------------------------------------------------------------------

async fn get_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Organiser) {
        return Err(AppError::Forbidden);
    }
    let member = member_service::get_member(&pool, &id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user_map = if let Some(ref uid) = member.user_id {
        fetch_user_data_map(&pool, &[uid.clone()]).await
    } else {
        HashMap::new()
    };
    let user_data = member.user_id.as_deref().and_then(|uid| user_map.get(uid));

    let sub_members = member_service::list_sub_members(&pool, &id)
        .await
        .unwrap_or_default();
    let sub_members_json: Vec<serde_json::Value> = sub_members.iter().map(|sm| serde_json::json!({
        "id": sm.id,
        "memberId": sm.member_id,
        "name": sm.name,
        "email": sm.email,
        "phone": sm.phone,
        "relation": sm.relation,
        "canLogin": sm.can_login,
        "createdAt": sm.created_at,
    })).collect();

    let mut json = member_to_json(&member, user_data);
    json["subMembers"] = serde_json::json!(sub_members_json);
    Ok(Json(json))
}

// ---------------------------------------------------------------------------
// Create member — OPERATOR+
// ---------------------------------------------------------------------------

async fn create_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Operator) {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy { id: claims.user_id.clone(), role: claims.role.clone(), name: claims.username.clone() };
    let input = crate::repositories::members::CreateMemberData {
        user_id: None,
        name: body["name"].as_str().unwrap_or("").to_string(),
        phone: body["phone"].as_str().unwrap_or("").to_string(),
        email: body["email"].as_str().unwrap_or("").to_string(),
        address: body["address"].as_str().unwrap_or("").to_string(),
        parent_member_id: body["parentMemberId"].as_str().map(String::from),
    };
    let result = member_service::create_member(&pool, &input, &actor)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "action": result.action,
        "memberId": result.member_id,
        "approvalId": result.approval_id,
    })))
}

// ---------------------------------------------------------------------------
// Update member — OPERATOR+
// ---------------------------------------------------------------------------

async fn update_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Operator) {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy { id: claims.user_id.clone(), role: claims.role.clone(), name: claims.username.clone() };
    let update_data = crate::repositories::members::UpdateMemberData {
        name: body["name"].as_str().map(String::from),
        phone: body["phone"].as_str().map(String::from),
        email: body["email"].as_str().map(String::from),
        address: body["address"].as_str().map(String::from),
    };
    let result = member_service::update_member(&pool, &id, &update_data, &actor)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "action": result.action,
        "memberId": result.member_id,
        "approvalId": result.approval_id,
    })))
}

// ---------------------------------------------------------------------------
// Delete member — OPERATOR+
// ---------------------------------------------------------------------------

async fn delete_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Operator) {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy { id: claims.user_id.clone(), role: claims.role.clone(), name: claims.username.clone() };
    member_service::delete_member(&pool, &id, &actor)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// List sub-members — ORGANISER+
// ---------------------------------------------------------------------------

async fn list_sub_members(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Organiser) {
        return Err(AppError::Forbidden);
    }
    let subs = member_service::list_sub_members(&pool, &id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Map to camelCase to match frontend expectations
    let data: Vec<serde_json::Value> = subs.iter().map(|sm| serde_json::json!({
        "id": sm.id,
        "memberId": sm.member_id,
        "parentUserId": sm.parent_user_id,
        "name": sm.name,
        "email": sm.email,
        "phone": sm.phone,
        "relation": sm.relation,
        "canLogin": sm.can_login,
        "createdAt": sm.created_at,
    })).collect();

    Ok(Json(serde_json::json!(data)))
}

// ---------------------------------------------------------------------------
// Add sub-member — OPERATOR+
// ---------------------------------------------------------------------------

async fn add_sub_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Operator) {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy {
        id: claims.user_id.clone(),
        role: claims.role.clone(),
        name: claims.username.clone(),
    };
    let name = body["name"].as_str().unwrap_or("").to_string();
    let email = body["email"].as_str().unwrap_or("").to_string();
    let phone = body["phone"].as_str().unwrap_or("").to_string();
    let relation = body["relation"].as_str().unwrap_or("").to_string();

    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".to_string()));
    }

    let result = member_service::add_sub_member(&pool, &id, &name, &email, &phone, &relation, &actor)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "action": result.action,
        "memberId": result.member_id,
        "approvalId": result.approval_id,
        "message": if result.action == "pending_approval" {
            "Sub-member add request submitted for admin approval"
        } else {
            "Sub-member added successfully"
        },
    })))
}

// ---------------------------------------------------------------------------
// Update sub-member — OPERATOR+
// ---------------------------------------------------------------------------

async fn update_sub_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Operator) {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy {
        id: claims.user_id.clone(),
        role: claims.role.clone(),
        name: claims.username.clone(),
    };

    // sub_member_id comes from the body (DPS PUT sends subMemberId in body)
    let sub_member_id = body["subMemberId"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("subMemberId is required".to_string()))?
        .to_string();

    let name = body["name"].as_str();
    let email = body["email"].as_str();
    let phone = body["phone"].as_str();
    let relation = body["relation"].as_str();
    let can_login = body["canLogin"].as_bool();

    let result = member_service::update_sub_member(
        &pool,
        &id,
        &sub_member_id,
        name,
        email,
        phone,
        relation,
        can_login,
        &actor,
    )
    .await
    .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "action": result.action,
        "approvalId": result.approval_id,
        "message": if result.action == "pending_approval" {
            "Sub-member update request submitted for admin approval"
        } else {
            "Sub-member updated successfully"
        },
    })))
}

// ---------------------------------------------------------------------------
// Remove sub-member — OPERATOR+
// ---------------------------------------------------------------------------

async fn remove_sub_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let role = parse_role(&claims);
    if !role.has_at_least(Role::Operator) {
        return Err(AppError::Forbidden);
    }
    let actor = RequestedBy {
        id: claims.user_id.clone(),
        role: claims.role.clone(),
        name: claims.username.clone(),
    };

    // sub_member_id comes from the body (DPS DELETE sends subMemberId in body)
    let sub_member_id = body["subMemberId"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("subMemberId is required".to_string()))?
        .to_string();

    let result = member_service::remove_sub_member(&pool, &id, &sub_member_id, &actor)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "action": result.action,
        "approvalId": result.approval_id,
        "message": if result.action == "pending_approval" {
            "Sub-member remove request submitted for admin approval"
        } else {
            "Sub-member removed successfully"
        },
    })))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Fetch user data for a batch of user IDs, returning a map of user_id → user JSON.
async fn fetch_user_data_map(
    pool: &SqlitePool,
    user_ids: &[String],
) -> HashMap<String, serde_json::Value> {
    if user_ids.is_empty() {
        return HashMap::new();
    }

    let placeholders: Vec<String> = (1..=user_ids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT id, member_id, membership_status, membership_type,
                membership_start, membership_expiry,
                annual_fee_start, annual_fee_expiry,
                CAST(total_paid AS REAL) as total_paid,
                application_fee_paid, annual_fee_paid
         FROM users WHERE id IN ({})",
        placeholders.join(", ")
    );

    let mut query = sqlx::query(&sql);
    for uid in user_ids {
        query = query.bind(uid);
    }

    let rows = match query.fetch_all(pool).await {
        Ok(r) => r,
        Err(_) => return HashMap::new(),
    };

    let mut map = HashMap::new();
    for row in rows {
        let id: String = row.try_get("id").unwrap_or_default();
        let member_id: String = row.try_get("member_id").unwrap_or_default();
        let membership_status: String = row.try_get("membership_status").unwrap_or_default();
        let membership_type: Option<String> = row.try_get("membership_type").unwrap_or(None);
        let membership_start: Option<String> = row.try_get("membership_start").unwrap_or(None);
        let membership_expiry: Option<String> = row.try_get("membership_expiry").unwrap_or(None);
        let annual_fee_start: Option<String> = row.try_get("annual_fee_start").unwrap_or(None);
        let annual_fee_expiry: Option<String> = row.try_get("annual_fee_expiry").unwrap_or(None);
        let total_paid: f64 = row.try_get("total_paid").unwrap_or(0.0);
        let application_fee_paid: bool = row.try_get("application_fee_paid").unwrap_or(false);
        let annual_fee_paid: bool = row.try_get("annual_fee_paid").unwrap_or(false);
        let subscription_fee_paid = membership_status == "ACTIVE";

        map.insert(id, serde_json::json!({
            "memberId": member_id,
            "membershipStatus": membership_status,
            "membershipType": membership_type,
            "membershipStart": membership_start,
            "membershipExpiry": membership_expiry,
            "annualFeeStart": annual_fee_start,
            "annualFeeExpiry": annual_fee_expiry,
            "totalPaid": total_paid,
            "applicationFeePaid": application_fee_paid,
            "annualFeePaid": annual_fee_paid,
            "subscriptionFeePaid": subscription_fee_paid,
        }));
    }
    map
}

/// Convert a Member + optional user data into the JSON shape the frontend expects.
fn member_to_json(member: &Member, user_data: Option<&serde_json::Value>) -> serde_json::Value {
    let display_status = user_data
        .and_then(|u| u["membershipStatus"].as_str())
        .unwrap_or(if member.user_id.is_some() {
            "PENDING_PAYMENT"
        } else {
            "PENDING_APPROVAL"
        });

    serde_json::json!({
        "id": member.id,
        "name": member.name,
        "email": member.email,
        "phone": member.phone,
        "address": member.address,
        "displayMembershipStatus": display_status,
        "joinedAt": member.joined_at,
        "createdAt": member.created_at,
        "updatedAt": member.updated_at,
        "user": user_data,
    })
}
